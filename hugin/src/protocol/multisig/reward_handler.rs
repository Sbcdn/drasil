/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use crate::datamodel::ScriptSpecParams;
use crate::protocol::{create_response, determine_contract};
use crate::BuildMultiSig;
use crate::CmdError;
use murin::PerformTxb;
//use gungnir::schema::whitelist;

pub(crate) async fn handle_rewardclaim(bms: &BuildMultiSig) -> crate::Result<String> {
    info!("verify transaction data...");
    match bms
        .transaction_pattern()
        .script()
        .ok_or("ERROR: No specific contract data supplied")?
    {
        ScriptSpecParams::SpoRewardClaim {
            reward_tokens,
            recipient_stake_addr,
            recipient_payment_addr,
        } => {
            let err = Err(CmdError::Custom {
                str: format!(
                    "ERROR wrong data provided for script specific parameters: '{:?}'",
                    bms.transaction_pattern().script()
                ),
            }
            .into());
            if reward_tokens.is_empty()
                || murin::b_decode_addr(&recipient_stake_addr).await.is_err()
                || murin::b_decode_addr(&recipient_payment_addr).await.is_err()
                || murin::wallet::get_stake_address(
                    &murin::b_decode_addr(&recipient_stake_addr).await?,
                )? != murin::wallet::get_stake_address(
                    &murin::b_decode_addr(&mimir::api::select_addr_of_first_transaction(
                        &murin::decode_addr(&recipient_stake_addr)
                            .await?
                            .to_bech32(None)
                            .unwrap(),
                    )?)
                    .await?,
                )?
            {
                return err;
            }
        }
        _ => {
            return Err(CmdError::Custom {
                str: format!("ERROR wrong data provided for '{:?}'", bms.multisig_type()),
            }
            .into());
        }
    }

    info!("create raw data...");
    let mut rwdtxd = bms
        .transaction_pattern()
        .script()
        .unwrap()
        .into_rwd()
        .await?;
    rwdtxd.set_payment_addr(
        &murin::b_decode_addr(&mimir::api::select_addr_of_first_transaction(
            &rwdtxd
                .get_stake_addr()
                .to_bech32(None)
                .expect("ERROR Could not construct bech32 address for stake address"),
        )?)
        .await?,
    );
    let mut gtxd = bms.transaction_pattern().into_txdata().await?;
    gtxd.set_user_id(bms.customer_id() as u64);

    info!("establish database connections...");
    let mut drasildbcon = crate::database::drasildb::establish_connection()?;
    let mut gcon = gungnir::establish_connection()?;

    info!("determine contract...");
    let mut contract = determine_contract(gtxd.get_contract_id(), bms.customer_id())?;
    if contract.is_none() {
        info!("No contract ID provided, try to select contract automatically");
        let contracts = crate::drasildb::TBContracts::get_all_contracts_for_user_typed(
            bms.customer_id(),
            crate::MultiSigType::SpoRewardClaim.to_string(),
        )?;
        let mut all_tokens_available = Vec::<bool>::new();
        for c in contracts {
            all_tokens_available.extend(rwdtxd.get_reward_tokens().iter().map(|t| {
                let fingerprint = murin::chelper::make_fingerprint(
                    &hex::encode(t.0.to_bytes()),
                    &hex::encode(t.1.name()),
                )
                .unwrap();
                let has_wl = gungnir::TokenWhitelist::has_contract_valid_whitelisting(
                    c.contract_id,
                    bms.customer_id(),
                    &fingerprint,
                )
                .unwrap();
                if has_wl {
                    // Check if whitelist token on contract also has available rewards
                    gungnir::Rewards::get_available_rewards(
                        &mut gcon,
                        &gtxd
                            .get_stake_address()
                            .to_bech32(None)
                            .expect("ERROR Could not construct bech32 address for stake address"),
                        &rwdtxd
                            .get_payment_addr()
                            .to_bech32(None)
                            .expect("ERROR Could not construct bech32 address for payment address"),
                        &fingerprint,
                        c.contract_id as i64,
                        bms.customer_id(),
                        murin::clib::utils::from_bignum(&t.2) as i64,
                    )
                    .is_ok()
                } else {
                    false
                }
            }));
            let sum = all_tokens_available.iter().fold(true, |n, s| *s && n);
            debug!("Available Tokens: {:?}", all_tokens_available);
            debug!("Sum: {:?}", sum);

            if sum {
                contract = Some(c.clone());
                break;
            } else {
                all_tokens_available = Vec::<bool>::new();
            }
        }

        if contract.is_none() {
            return Err(CmdError::Custom {
                str: "Automatic selection failed, no suitable contract found.".to_string(),
            }
            .into());
        }
    }
    let contract = contract.expect("Error: Could not unwrap contract");
    info!(
        "Contract selected: UID: '{}', CID '{}'",
        bms.customer_id(),
        contract.contract_id
    );

    info!("check vesting periods...");
    // Filter Tokens which are in a vesting period out of the reward tokens
    // get the token whitelist of the contract
    let vesting_whitelist = gungnir::TokenWhitelist::get_in_vesting_filtered_whitelist(
        contract.contract_id,
        bms.customer_id(),
    )?;
    // for the remaining whitelisting filter the tokens out of reward tokens.
    let mut rwd_tokens = rwdtxd.get_reward_tokens();
    for vt in vesting_whitelist {
        let policy = murin::chelper::string_to_policy(&vt.policy_id)?;
        let assetname = match vt.tokenname {
            Some(tn) => murin::chelper::string_to_assetname(&tn)?,
            None => {
                murin::clib::AssetName::new(b"".to_vec()).expect("Could not create emtpy tokenname")
            }
        };
        rwd_tokens.retain(|n| n.0 != policy && n.1 != assetname);
    }
    debug!("Reward Tokens after Vesting filte:\n'{:?}'", rwd_tokens);
    rwdtxd.set_reward_tokens(&rwd_tokens);
    if rwdtxd.get_reward_tokens().is_empty() {
        return Err(CmdError::Custom {
            str: "The requested tokens are all still in a vesting period.".to_string(),
        }
        .into());
    }

    info!("check reward balances...");
    //Check token balances
    for token in rwdtxd.get_reward_tokens() {
        let fingerprint = murin::chelper::make_fingerprint(
            &hex::encode(token.0.to_bytes()),
            &hex::encode(token.1.name()),
        )?;
        info!("Fingerprint: '{}'", fingerprint);
        if token.2.compare(&murin::clib::utils::to_bignum(0)) <= 0 {
            return Err(CmdError::Custom {
                str: format!(
                    "ERROR claiming amount '0' of a token is not allowed: '{:?}'",
                    fingerprint
                ),
            }
            .into());
        }
        info!("Try to get available rewards!");
        log::debug!("{:?}", contract);
        log::debug!("{:?}", fingerprint);
        log::debug!("{:?}", murin::clib::utils::from_bignum(&token.2) as i64);
        log::debug!(
            "{:?}",
            gtxd.get_stake_address()
                .to_bech32(None)
                .expect("ERROR Could not construct bech32 address for stake address")
        );
        log::debug!("{:?}", bms.customer_id());
        match gungnir::Rewards::get_available_rewards(
            &mut gcon,
            &gtxd
                .get_stake_address()
                .to_bech32(None)
                .expect("ERROR Could not construct bech32 address for stake address"),
            &rwdtxd
                .get_payment_addr()
                .to_bech32(None)
                .expect("ERROR Could not construct bech32 address for payment address"),
            &fingerprint,
            contract.contract_id as i64,
            bms.customer_id(),
            murin::clib::utils::from_bignum(&token.2) as i64,
        )? {
            i if i >= 0 => {
                info!(
                    "User has enough tokens earned to claim: '{:?}'",
                    fingerprint
                );
            }
            _ => {
                info!("Error in Rewards!");
                return Err(CmdError::Custom {
                    str: format!(
                        "ERROR user did not earned enough tokens to claim this amount: '{:?}'",
                        token
                    ),
                }
                .into());
            }
        }
    }

    info!("lookup additional data...");
    let keyloc = crate::drasildb::TBMultiSigLoc::get_multisig_keyloc(
        &mut drasildbcon,
        &contract.contract_id,
        &contract.user_id,
        &contract.version,
    )?;

    if let Some(fee_addr) = keyloc.fee_wallet_addr {
        if let Ok(addr) = murin::clib::address::Address::from_bech32(&fee_addr) {
            rwdtxd.set_fee_wallet_addr(&addr);
        }
    };
    if let Some(fee) = keyloc.fee {
        rwdtxd.set_fee(&(fee as u64));
    };

    let ns_addr = contract.address;
    let ns_script = contract.plutus;
    let ns_version = contract.version.to_string();

    info!("retrieve blockchain data...");
    let mut dbsync = mimir::establish_connection()?;
    let slot = mimir::get_slot(&mut dbsync)?;
    gtxd.set_current_slot(slot as u64);

    let reward_wallet_utxos = mimir::get_address_utxos(&mut dbsync, &ns_addr)?;
    if reward_wallet_utxos.is_empty() {
        // ToDO :
        // Send Email to Admin that not enough tokens are available on the script
        return Err(CmdError::Custom {
            str: "The contract does not contain enough tokens to claim, please try again later"
                .to_string(),
        }
        .into());
    }

    rwdtxd.set_reward_utxos(&Some(reward_wallet_utxos.clone()));

    //ToDO:
    // - Function to check and split utxos when for size >5kB (cal_min_ada panics on utxos >5kB)
    // - Find a solution for protocal parameters (maybe to database?) at the moment they are hardcoded in list / build_rwd
    let ident = crate::encryption::mident(
        &contract.user_id,
        &contract.contract_id,
        &contract.version,
        &ns_addr,
    );
    let pkvs = crate::encryption::decrypt_pkvs(keyloc.pvks, &ident).await?;
    info!("build transaction...");
    let txb_param: murin::txbuilders::rwdist::AtRWDParams = (
        murin::wallet::b_decode_addr(&ns_addr).await?,
        murin::clib::NativeScript::from_bytes(hex::decode(ns_script)?).map_err::<CmdError, _>(
            |_| CmdError::Custom {
                str: "could not convert string to native script".to_string(),
            },
        )?,
        ns_version,
        &rwdtxd,
    );
    let rwd = murin::txbuilders::rwdist::AtRWDBuilder::new(txb_param);
    let builder = murin::TxBuilder::new(&gtxd, &pkvs);
    let bld_tx = builder.build(&rwd).await?;

    info!("post processing transaction...");
    let tx = murin::utxomngr::RawTx::new(
        &bld_tx.get_tx_body(),
        &bld_tx.get_txwitness(),
        &bld_tx.get_tx_unsigned(),
        &bld_tx.get_metadata(),
        &gtxd.to_string(),
        &rwdtxd.to_string(),
        &bld_tx.get_used_utxos(),
        &hex::encode(gtxd.get_stake_address().to_bytes()),
        &(bms.customer_id()),
        &contract.contract_id,
        &contract.version,
    );
    debug!("RAWTX data: {:?}", tx);

    info!("create response...");
    let ret = create_response(
        &bld_tx,
        &tx,
        bms.transaction_pattern().wallet_type().as_ref(),
    )?;

    Ok(ret.to_string())
}
