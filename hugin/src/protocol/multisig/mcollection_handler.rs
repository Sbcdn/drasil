/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use crate::datamodel::ScriptSpecParams;
use crate::protocol::{convert_nfts_to_minter_token_asset, create_response, determine_contract};
use crate::BuildMultiSig;
use crate::CmdError;
use murin::PerformTxb;

pub(crate) async fn handle_collection_mint(bms: &BuildMultiSig) -> crate::Result<String> {
    match bms
        .transaction_pattern()
        .script()
        .ok_or("ERROR: No specific contract data supplied")?
    {
        ScriptSpecParams::NftMinter {
            receiver_stake_addr,
            receiver_payment_addr,
            ..
        } => {
            let err = Err(CmdError::Custom {
                str: format!(
                    "ERROR wrong data provided for script specific parameters: '{:?}'",
                    bms.transaction_pattern().script()
                ),
            }
            .into());
            if murin::decode_addr(&receiver_payment_addr).await.is_err() {
                return err;
            } else if let Some(saddr) = receiver_stake_addr {
                if murin::wallet::get_stake_address(&murin::decode_addr(&saddr).await?)?
                    != murin::wallet::get_stake_address(
                        &murin::decode_addr(&receiver_payment_addr).await?,
                    )?
                {
                    return err;
                }
            }
        }
        _ => {
            return Err(CmdError::Custom {
                str: format!("ERROR wrong data provided for '{:?}'", bms.multisig_type()),
            }
            .into());
        }
    }
    log::debug!("Checks okay...");

    log::debug!("Try to create raw data...");
    let mut minttxd = bms
        .transaction_pattern()
        .script()
        .unwrap()
        .into_mintdata()
        .await?;
    let mut gtxd = bms.transaction_pattern().into_txdata().await?;

    log::debug!("Check contract...");
    let contract = determine_contract(gtxd.get_contract_id(), bms.customer_id())?
        .expect("Could not find valid contract");

    log::debug!("Try to establish database connection...");
    let mut drasildbcon = crate::database::drasildb::establish_connection()?;

    log::debug!("Try to determine additional data...");
    let keyloc = crate::drasildb::TBMultiSigLoc::get_multisig_keyloc(
        &mut drasildbcon,
        &contract.contract_id,
        &contract.user_id,
        &contract.version,
    )?;

    log::debug!("Try to determine fees...");
    if let Some(feeaddr) = keyloc.fee_wallet_addr {
        match murin::b_decode_addr(&feeaddr).await {
            Ok(a) => minttxd.set_fee_addr(a),
            Err(e) => {
                return Err(CmdError::Custom {
                    str: format!("ERROR could not decode fee address: '{:?}'", e.to_string()),
                }
                .into());
            }
        };
    };

    if let Some(fee) = keyloc.fee {
        minttxd.set_fee(fee)
    };

    let ns_addr: Option<murin::address::Address> =
        Some(murin::b_decode_addr(&contract.address).await?);
    // ToDo:
    //if minttxd.get_to_vendor_script() == true {
    //    ns_addr = Some(contract.address);
    //}
    let ns_script = contract.plutus;
    let ns_version = contract.version.to_string();

    log::debug!("Try to determine nft data...");
    let mut gcon = gungnir::establish_connection()?;
    let mint_project = gungnir::MintProject::get_mintproject_by_uid_cid(
        &mut gcon,
        bms.customer_id(),
        contract.contract_id,
    )?;
    let _whitelist = if let Some(wid) = mint_project.whitelist_id {
        Some(gungnir::Whitelist::get_whitelist(&mut gcon, wid)?)
    } else {
        None
    };

    let policy_id = contract
        .policy_id
        .expect("Error: The provided contract is not eligable to mint, no policy ID");
    let payment_addr_bech32 = minttxd.get_payment_addr_bech32()?;

    let mut nfts_to_mint = Vec::<gungnir::Nft>::new();
    let mut avail_rewards = Vec::<gungnir::Rewards>::new();
    let mut eligable_nfts = Vec::<gungnir::Nft>::new();
    if mint_project.reward_minter {
        nfts_to_mint =
            gungnir::Nft::get_nft_by_payaddr(&mut gcon, mint_project.id, &payment_addr_bech32)?;
        nfts_to_mint.retain(|n| !n.minted);
        if nfts_to_mint.is_empty() {
            return Err(CmdError::Custom {
                str: "ERROR No assets for this address available".to_string(),
            }
            .into());
        }
        for nft in &nfts_to_mint {
            let fingerprint = murin::chelper::make_fingerprint(&policy_id, &nft.asset_name)?;
            let rewards = gungnir::Rewards::get_avail_specific_asset_reward(
                &mut gcon,
                &payment_addr_bech32,
                &murin::cip30::get_bech32_stake_address_from_str(&payment_addr_bech32)?,
                &fingerprint,
                contract.contract_id,
                bms.customer_id(),
            )?;
            if !rewards.is_empty() {
                eligable_nfts.push(nft.clone());
            }
            avail_rewards.extend(rewards.iter().map(|n| n.to_owned()));
        }
        if avail_rewards.len() != eligable_nfts.len() {
            return Err(CmdError::Custom {
                str:
                    "ERROR there is some missmatch in your available rewards please contact support"
                        .to_string(),
            }
            .into());
        }
    } else {
        let claimed = gungnir::Claimed::get_claims(
            &mut gcon,
            &murin::cip30::get_bech32_stake_address_from_str(&payment_addr_bech32)?,
            contract.contract_id,
            bms.customer_id(),
        )?;
        //ToDo: Add parameter for NFTs minted per transaction
        let mut max_nfts = 1;
        if let Some(max) = mint_project.max_mint_p_addr {
            if claimed.len() >= max as usize {
                return Err(CmdError::Custom {
                    str: "ERROR No assets for this address available".to_string(),
                }
                .into());
            }
            max_nfts = max;
        }
        for _ in 0..max_nfts {
            nfts_to_mint.extend(
                gungnir::Nft::get_random_unminted_nft(&mut gcon, mint_project.id)?.into_iter(),
            )
        }
    }

    // create MintTokenAsset datatype for all nfts to be minted
    minttxd.set_mint_tokens(convert_nfts_to_minter_token_asset(
        &nfts_to_mint,
        &policy_id,
    )?);

    // create transaction specific metadata
    let mut metadataassets = Vec::<murin::minter::AssetMetadata>::new();
    for nft in &nfts_to_mint {
        metadataassets.push(serde_json::from_str(&nft.metadata)?)
    }
    let metadata = murin::minter::Cip25Metadata {
        assets: metadataassets,
        other: None,
        version: "1.0".to_string(),
    };
    minttxd.set_metadata(metadata);

    log::debug!("Try to determine slot...");
    let mut dbsync = match mimir::establish_connection() {
        Ok(conn) => conn,
        Err(e) => {
            return Err(CmdError::Custom {
                str: format!("ERROR could not connect to dbsync: '{:?}'", e.to_string()),
            }
            .into());
        }
    };
    let slot = match mimir::get_slot(&mut dbsync) {
        Ok(s) => s,
        Err(e) => {
            return Err(CmdError::Custom {
                str: format!(
                    "ERROR could not determine current slot: '{:?}'",
                    e.to_string()
                ),
            }
            .into());
        }
    };
    gtxd.set_current_slot(slot as u64);

    //ToDO:
    // - Find a solution for protocal parameters (maybe to database?) at the moment they are hardcoded in list / build_rwd

    let ident = crate::encryption::mident(
        &contract.user_id,
        &contract.contract_id,
        &contract.version,
        &contract.address,
    );
    let pkvs = crate::encryption::decrypt_pkvs(keyloc.pvks, &ident).await?;

    log::debug!("Try to build transaction...");

    let txb_param: murin::txbuilders::minter::build_minttx::AtCMParams = (
        ns_addr,
        murin::clib::NativeScript::from_bytes(hex::decode(ns_script)?).map_err::<CmdError, _>(
            |_| CmdError::Custom {
                str: "could not convert string to native script".to_string(),
            },
        )?,
        &minttxd,
    );
    let minter = murin::txbuilders::minter::build_minttx::AtCMBuilder::new(txb_param);
    let builder = murin::TxBuilder::new(&gtxd, &pkvs);
    let bld_tx = builder.build(&minter).await?;

    log::debug!("Try to create raw tx...");
    let tx = murin::utxomngr::RawTx::new(
        &bld_tx.get_tx_body(),
        &bld_tx.get_txwitness(),
        &bld_tx.get_tx_unsigned(),
        &bld_tx.get_metadata(),
        &gtxd.to_string(),
        &minttxd.to_string(),
        &bld_tx.get_used_utxos(),
        &hex::encode(gtxd.get_stake_address().to_bytes()),
        &(bms.customer_id()),
        &contract.contract_id,
        &contract.version,
    );
    debug!("RAWTX data: {:?}", tx);

    log::debug!("Try to create response data...");
    let ret = create_response(
        &bld_tx,
        &tx,
        bms.transaction_pattern().wallet_type().as_ref(),
    )?;

    Ok(ret.to_string())
}
