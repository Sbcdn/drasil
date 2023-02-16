/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
extern crate pretty_env_logger;

mod error;
mod handlers;
mod models;

use deadpool_lapin::Pool;
use gungnir::models::MintReward;
use lapin::ConnectionProperties;
use lazy_static::lazy_static;
use murin::{clib::Assets, utils::to_bignum, AssetName, MultiAsset, PolicyID};
use std::env;

lazy_static! {
    static ref AMQP_ADDR: String =
        std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://rmq:rmq@127.0.0.1:5672/%2f".into());
    static ref QUEUE_NAME: String =
        std::env::var("QUEUE_NAME").unwrap_or_else(|_| "mint_response".to_string());
    static ref CONSUMER_NAME: String =
        std::env::var("CONSUMER_NAME").unwrap_or_else(|_| "work_loki_0".to_string());
}

#[tokio::main]
async fn main() -> Result<(), error::Error> {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();
    // RMQ
    let manager =
        deadpool_lapin::Manager::new(AMQP_ADDR.to_string(), ConnectionProperties::default());
    let pool: deadpool_lapin::Pool = deadpool::managed::Pool::builder(manager)
        .max_size(100)
        .build()
        .expect("can't create pool");
    log::debug!("pool: {:?}", pool);

    let _ = futures::join!(rmq_listen(pool));
    Ok(())
}

async fn get_rmq_con(pool: Pool) -> Result<models::Connection, deadpool_lapin::PoolError> {
    let connection = pool.get().await?;
    Ok(connection)
}

async fn rmq_listen(pool: deadpool_lapin::Pool) -> Result<(), error::Error> {
    let mut retry_interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
    loop {
        retry_interval.tick().await;
        log::debug!("connecting rmq consumer...");
        match init_rmq_listen(pool.clone()).await {
            Ok(_) => log::debug!("rmq listen returned"),
            Err(e) => log::debug!("rmq listen had an error: {}", e),
        };
    }
}

async fn init_rmq_listen(pool: Pool) -> Result<(), error::Error> {
    let rmq_con = get_rmq_con(pool).await.map_err(|e| {
        log::error!("could not get rmq con: {}", e);
        e
    })?;
    let channel = rmq_con.create_channel().await?;

    let queue = channel
        .queue_declare(
            &QUEUE_NAME.to_string(),
            lapin::options::QueueDeclareOptions::default(),
            lapin::types::FieldTable::default(),
        )
        .await?;
    log::debug!("Declared queue {:?}", queue);

    let mut consumer = channel
        .basic_consume(
            &QUEUE_NAME.to_string(),
            &CONSUMER_NAME.to_string(),
            lapin::options::BasicConsumeOptions::default(),
            lapin::types::FieldTable::default(),
        )
        .await?;

    log::debug!("rmq consumer connected, waiting for messages");
    while let Some(delivery) = tokio_stream::StreamExt::next(&mut consumer).await {
        if let Ok(deliv) = delivery {
            log::debug!("consumer received msg: {:?}", deliv);
            // Do something with message
            let data: models::ClaimMintRewards =
                serde_json::from_str(std::str::from_utf8(&deliv.data)?)?;
            log::debug!("Data {:?}", data);
            // Acknowledge message if successfull
            log::debug!("try to get mint project ...");
            let mp = match gungnir::minting::models::MintProject::get_mintproject_by_id(data.mpid) {
                Ok(o) => o,
                Err(e) => {
                    log::error!("could not find mint project: {}", e.to_string());
                    channel
                        .basic_reject(
                            deliv.delivery_tag,
                            lapin::options::BasicRejectOptions::default(),
                        )
                        .await?;
                    return Err(crate::error::Error::Custom(
                        "could not find mint project:".to_owned(),
                    ));
                }
            };

            if !mp.active {
                log::error!("requesed to mint on an inactive project");
                channel
                    .basic_ack(
                        deliv.delivery_tag,
                        lapin::options::BasicAckOptions::default(),
                    )
                    .await?;
                return Err(crate::error::Error::Custom(
                    "requesed to mint on an inactive project, request discarded".to_owned(),
                ));
            }

            log::debug!("check data ...");
            let address = murin::address::Address::from_bech32(&data.claim_addr)?;
            // header has 4 bits addr type discrim then 4 bits network discrim.
            // Copied from shelley.cddl:
            //
            // shelley payment addresses:
            // bit 7: 0
            // bit 6: base/other
            // bit 5: pointer/enterprise [for base: stake cred is keyhash/scripthash]
            // bit 4: payment cred is keyhash/scripthash
            // bits 3-0: network id
            //
            // reward addresses:
            // bits 7-5: 111
            // bit 4: credential is keyhash/scripthash
            // bits 3-0: network id
            //
            // byron addresses:
            // bits 7-4: 1000
            let stake_address: String = match address.to_bytes()[0] {
                //base
                0b0000 | 0b0001 => murin::get_reward_address(&address)?.to_bech32(None)?,
                //script address
                0b0010 | 0b0011 => {
                    log::error!("script address cannot claim");
                    return Err(crate::error::Error::Custom(
                        "script address cannot claim:".to_owned(),
                    ));
                }
                //pointer
                0b0100 | 0b0101 => {
                    log::error!("pointer address cannot claim");
                    return Err(crate::error::Error::Custom(
                        "pointer address cannot claim:".to_owned(),
                    ));
                }
                //enterprise
                0b0110 | 0b0111 => {
                    log::error!("enterprise address cannot claim");
                    return Err(crate::error::Error::Custom(
                        "enterprise address cannot claim:".to_owned(),
                    ));
                }
                //reward
                0b1110 | 0b1111 => data.claim_addr.clone(),
                //byron 0b1000
                _ => {
                    log::error!("byron or undefined cannot claim");
                    return Err(crate::error::Error::Custom(
                        "byron address cannot claim:".to_owned(),
                    ));
                }
            };

            // get first payment address
            let mut payment_addr = mimir::api::select_addr_of_first_transaction(&stake_address)?;

            // check whitelists
            let valid_addresses = if let Some(wl) = mp.whitelists.clone() {
                let mut va = Vec::<(gungnir::WlEntry, i64)>::new();

                for w in wl {
                    let claim_wl =
                        gungnir::WlAlloc::check_pay_address_in_whitelist(&w, &data.claim_addr)?;
                    if !claim_wl.is_empty() {
                        va.extend(claim_wl.into_iter().map(|n| (n, w)));
                        payment_addr = data.claim_addr.clone();
                        break;
                    } else {
                        va.extend(
                            gungnir::WlAlloc::check_stake_address_in_whitelist(&w, &stake_address)?
                                .into_iter()
                                .map(|n| (n, w)),
                        );
                        va.extend(
                            gungnir::WlAlloc::check_pay_address_in_whitelist(&w, &payment_addr)?
                                .into_iter()
                                .map(|n| (n, w)),
                        );
                    }
                }
                Some(va)
            } else {
                None
            };
            if valid_addresses.is_none() && mp.whitelists.is_some() {
                log::error!("requesting wallet is not whitelisted");
                return Err(crate::error::Error::Custom(
                    "requesting wallet is not whitelisted".to_owned(),
                ));
            }

            if let Some(i) = mp.max_mint_p_addr {
                let nfts = gungnir::minting::models::Nft::get_nft_by_claim_addr(
                    mp.id,
                    &payment_addr,
                    &mp.nft_table_name,
                )?;
                log::debug!("already minted NFTs: {:?}", nfts);
                if nfts.len() >= i as usize {
                    log::error!("reached maximum allowed mints: {}", payment_addr);
                    channel
                        .basic_reject(
                            deliv.delivery_tag,
                            lapin::options::BasicRejectOptions::default(),
                        )
                        .await?;
                    return Err(crate::error::Error::Custom(
                        "reached maximum allowed mints".to_owned(),
                    ));
                }
            }

            log::debug!("try to claim nft ...");
            let nft = match gungnir::minting::models::Nft::claim_random_unminted_nft(
                mp.id,
                &mp.nft_table_name,
                &payment_addr,
                0,
            )
            .await
            {
                Ok(o) => o,
                Err(_) => {
                    channel
                        .basic_reject(
                            deliv.delivery_tag,
                            lapin::options::BasicRejectOptions::default(),
                        )
                        .await?;
                    None
                }
            };

            let mint_contract = hugin::database::TBContracts::get_contract_uid_cid(
                mp.user_id,
                mp.mint_contract_id,
            )?;

            match nft {
                Some(n) => {
                    log::debug!("nft: {:?}", n);
                    let mut mint_value = murin::clib::utils::Value::zero();
                    let mut assets = Assets::new();
                    assets.insert(&AssetName::new(n.asset_name_b.clone())?, &to_bignum(1));
                    let mut ma = MultiAsset::new();
                    ma.insert(
                        &PolicyID::from_hex(&mint_contract.policy_id.unwrap()).unwrap(),
                        &assets,
                    );
                    mint_value.set_multiasset(&ma);

                    MintReward::create_mintreward(
                        mp.user_id,
                        mp.mint_contract_id,
                        &payment_addr,
                        vec![&n.asset_name_b],
                        vec![&mint_value.to_bytes()],
                    )?;
                }
                None => {
                    return Err(crate::error::Error::Custom(
                        "Could not reserve NFT".to_owned(),
                    ));
                }
            }

            channel
                .basic_ack(
                    deliv.delivery_tag,
                    lapin::options::BasicAckOptions::default(),
                )
                .await?
        }
    }
    Ok(())
}
