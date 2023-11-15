use crate::datamodel::Operation;
use crate::protocol::{create_response, determine_contracts};
use crate::{discount, CmdError};
use crate::{BuildMultiSig, TBMultiSigLoc};
use drasil_murin::modules::transfer::models::*;
use drasil_murin::{wallet, PerformTxb};
//use drasil_gungnir::schema::whitelist;

/// The behaviors/actions to execute when the user wants to build a multisig transaction with
/// multisig type `SpoRewardClaim`
pub(crate) async fn handle_rewardclaim(bms: &BuildMultiSig) -> crate::Result<String> {
    info!("verify transaction data...");
    match bms
        .transaction_pattern()
        .operation()
        .ok_or("ERROR: No specific contract data supplied")?
    {
        Operation::SpoRewardClaim {
            rewards,
            recipient_stake_addr,
            recipient_payment_addr,
        } => {
            let err = Err(CmdError::Custom {
                str: format!(
                    "ERROR wrong data provided for script specific parameters: '{:?}'",
                    bms.transaction_pattern().operation()
                ),
            }
            .into());
            if rewards.is_empty()
                || wallet::address_from_string(&recipient_stake_addr)
                    .await
                    .is_err()
                || wallet::address_from_string(&recipient_payment_addr)
                    .await
                    .is_err()
                || wallet::stake_keyhash_from_address(
                    &wallet::address_from_string(&recipient_stake_addr).await?,
                )? != wallet::stake_keyhash_from_address(
                    &wallet::address_from_string(
                        &drasil_mimir::api::select_addr_of_first_transaction(
                            &wallet::decode_address_from_bytes(&recipient_stake_addr)
                                .await?
                                .to_bech32(None)
                                .unwrap(),
                        )?,
                    )
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
        .operation()
        .unwrap()
        .into_rwd()
        .await?;
    rwdtxd.set_payment_addr(
        &wallet::address_from_string(&drasil_mimir::api::select_addr_of_first_transaction(
            &rwdtxd
                .get_stake_addr()
                .to_bech32(None)
                .expect("ERROR Could not construct bech32 address for stake address"),
        )?)
        .await?,
    );
    let mut gtxd = bms.transaction_pattern().into_txdata().await?;
    gtxd.set_user_id(bms.customer_id());

    info!("establish database connections...");
    let mut gcon = drasil_gungnir::establish_connection()?;

    info!("check rewards and collect contract ids...");
    let mut contract_ids = Vec::<i64>::new();
    for rwd in &rwdtxd.get_rewards() {
        if rwd.get_stake_addr()?
            != drasil_murin::clib::address::RewardAddress::from_address(&rwdtxd.get_stake_addr())
                .unwrap()
        {
            return Err(CmdError::Custom {
                str: format!(
                    "wallet addresses insufficient '{:?}'",
                    rwd.get_stake_addr_str()
                ),
            }
            .into());
        }
        if drasil_gungnir::Rewards::get_available_rewards(
            &mut gcon,
            &rwd.get_stake_addr_str(),
            &rwdtxd
                .get_payment_addr()
                .to_bech32(None)
                .map_err(|_| drasil_murin::MurinError::new(""))?,
            &rwd.get_fingerprint(),
            rwd.get_contract_id(),
            gtxd.get_user_id().unwrap(),
            drasil_murin::clib::utils::from_bignum(&rwd.get_amount().unwrap()) as i128,
        )
        .is_err()
        {
            return Err(CmdError::Custom {
                str: format!("provided reward is faulty'{:?}'", rwd.get_fingerprint()),
            }
            .into());
        } else if !contract_ids.contains(&rwd.get_contract_id()) {
            contract_ids.push(rwd.get_contract_id());
        }
    }
    gtxd.set_contract_id(contract_ids);

    info!("determine contract...");
    let contract = determine_contracts(gtxd.get_contract_id(), bms.customer_id())?;
    let contract = contract.expect("Error: could not unwrap contracts");
    info!(
        "Contract selected: UID: '{}', CID '{:?}'",
        bms.customer_id(),
        contract
    );

    info!("check vesting periods...");
    // Filter Tokens which are in a vesting period out of the reward tokens
    // get the token whitelist of the contract
    let mut vesting_whitelist = Vec::<drasil_gungnir::TokenWhitelist>::new();
    for c in &contract {
        vesting_whitelist.extend(
            drasil_gungnir::TokenWhitelist::get_in_vesting_filtered_whitelist(
                c.contract_id,
                bms.customer_id(),
            )?
            .iter()
            .map(|n| n.to_owned()),
        );
    }
    // for the remaining whitelisting filter the tokens out of reward tokens.
    let mut rewards = rwdtxd.get_rewards();
    for vt in vesting_whitelist {
        let policy = drasil_murin::cardano::string_to_policy(&vt.policy_id)?;
        let assetname = match vt.tokenname {
            Some(tn) => drasil_murin::cardano::string_to_assetname(&tn)?,
            None => drasil_murin::clib::AssetName::new(b"".to_vec())
                .expect("Could not create emtpy tokenname"),
        };
        rewards.retain(|n| {
            n.get_policy_id().unwrap() != policy && n.get_assetname().unwrap() != assetname
        });
    }
    debug!("Reward Tokens after Vesting filter:\n'{:?}'", rewards);
    rwdtxd.set_reward_tokens(&rewards);
    if rwdtxd.get_rewards().is_empty() {
        return Err(CmdError::Custom {
            str: "The requested tokens are all still in a vesting period.".to_string(),
        }
        .into());
    }

    info!("check reward balances...");

    // Muss umgeschrieben werden anhand anderer input form für tokens
    /*
        //Check token balances
        for token in rwdtxd.get_reward_tokens() {
            let fingerprint = drasil_murin::chelper::make_fingerprint(
                &hex::encode(token.0.to_bytes()),
                &hex::encode(token.1.name()),
            )?;
            info!("Fingerprint: '{}'", fingerprint);
            if token.2.compare(&drasil_murin::clib::utils::to_bignum(0)) <= 0 {
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
            log::debug!("{:?}", drasil_murin::clib::utils::from_bignum(&token.2) as i64);
            log::debug!(
                "{:?}",
                gtxd.get_stake_address()
                    .to_bech32(None)
                    .expect("ERROR Could not construct bech32 address for stake address")
            );
            log::debug!("{:?}", bms.customer_id());
            match drasil_gungnir::Rewards::get_available_rewards(
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
                drasil_murin::clib::utils::from_bignum(&token.2) as i64,
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
    */
    info!("lookup additional data...");
    let mut keylocs = Vec::<TBMultiSigLoc>::new();
    for c in &contract {
        keylocs.push(crate::drasildb::TBMultiSigLoc::get_multisig_keyloc(
            &c.contract_id,
            &c.user_id,
            &c.version,
        )?);
    }

    let mut fees = keylocs.iter().fold(
        Vec::<(drasil_murin::address::Address, i64)>::new(),
        |mut l, n| {
            if let Some(fee_addr) = &n.fee_wallet_addr {
                if let Ok(addr) = drasil_murin::clib::address::Address::from_bech32(fee_addr) {
                    l.push((addr, n.fee.unwrap()))
                }
            }
            l
        },
    );
    fees.sort_by(|a, b| a.1.cmp(&b.1));
    if !fees.is_empty() {
        rwdtxd.set_fee_wallet_addr(&fees.last().unwrap().0);
        rwdtxd.set_fee(&(fees.last().unwrap().1 as u64));
    }
    let mut r = Vec::<i64>::new();
    for c in &contract {
        r.push(discount(gtxd.get_inputs(), c.contract_id, c.user_id));
    }

    r.sort();
    let discnt = r[r.len() - 1];

    if discnt > 0 {
        let fee = rwdtxd.get_fee().unwrap_or(0);
        rwdtxd.set_fee(&(fee - (fee as f64 * (discnt as f64 / 100.0)) as u64));
        if fee == 0 {
            rwdtxd.set_nofee();
        }
    }

    let mut wallets = TransWallets::new();
    let mut dbsync = drasil_mimir::establish_connection()?;
    for c in contract {
        let keyloc = crate::drasildb::TBMultiSigLoc::get_multisig_keyloc(
            &c.contract_id,
            &c.user_id,
            &c.version,
        )?;
        info!("retrieve blockchain data...");

        let wallet_utxos = drasil_mimir::get_address_utxos(&c.address)?;
        if wallet_utxos.is_empty() {
            // ToDO :
            // Send Email to Admin that not enough tokens are available on the script
            return Err(CmdError::Custom {
                str: "The contract does not contain utxos, please try again later".to_string(),
            }
            .into());
        } else {
            log::info!("UTxO Count for TransWallet: {}", wallet_utxos.len());
        }

        let ident = crate::encryption::mident(&c.user_id, &c.contract_id, &c.version, &c.address);
        let pkvs = crate::encryption::decrypt_pkvs(keyloc.pvks.clone(), &ident).await?;
        let tw_addr = wallet::address_from_string(&c.address).await?;
        let tw_script = drasil_murin::clib::NativeScript::from_bytes(hex::decode(c.plutus)?)
            .map_err::<CmdError, _>(|_| CmdError::Custom {
            str: "could not convert string to native script".to_string(),
        })?;
        let mut w = TransWallet::new(&tw_addr, &wallet_utxos);
        let s = CardanoNativeScript::new(&tw_addr, &tw_script, c.version, vec![pkvs[0].clone()]);
        w.set_native_script(s);
        w.set_cid(c.contract_id);
        wallets.add_wallet(&w);
    }
    // Add user wallet
    let uw = TransWallet::new(&rwdtxd.get_payment_addr(), &gtxd.get_inputs());
    wallets.add_wallet(&uw);
    log::debug!("Wallets in reward_handler: {:?}", wallets);
    let slot = drasil_mimir::get_slot(&mut dbsync)?;
    gtxd.set_current_slot(slot as u64);

    //rwdtxd.set_reward_utxos(&Some(reward_wallet_utxos.clone()));

    //ToDO:
    // - Function to check and split utxos when for size >5kB (cal_min_ada panics on utxos >5kB)
    // - Find a solution for protocal parameters (maybe to database?) at the moment they are hardcoded in list / build_rwd
    /*
    let ident = crate::encryption::mident(
        &contract[0].user_id,
        &contract[0].contract_id,
        &contract[0].version,
        &ns_addr,
    );
    let pkvs = crate::encryption::decrypt_pkvs(keylocs[0].pvks.clone(), &ident).await?;
    */
    info!("build transaction...");
    let txb_param: drasil_murin::txbuilder::rwdist::AtRWDParams = (&rwdtxd, Some(wallets));
    let rwd = drasil_murin::txbuilder::rwdist::AtRWDBuilder::new(txb_param);
    let builder = drasil_murin::TxBuilder::new(&gtxd, &vec![]);
    let bld_tx = builder.build(&rwd).await?;

    info!("post processing transaction...");
    let tx = drasil_murin::utxomngr::RawTx::new(
        &bld_tx.get_tx_body(),
        &bld_tx.get_txwitness(),
        &bld_tx.get_tx_unsigned(),
        &bld_tx.get_metadata(),
        &gtxd.to_string(),
        &rwdtxd.to_string(),
        &bld_tx.get_used_utxos(),
        &hex::encode(gtxd.get_stake_address().to_bytes()),
        &(bms.customer_id()),
        &gtxd.get_contract_id().unwrap(),
    );
    trace!("RAWTX data: {:?}", tx);

    info!("create response...");
    let ret = create_response(
        &bld_tx,
        &tx,
        bms.transaction_pattern().wallet_type().as_ref(),
    )?;

    Ok(ret.to_string())
}
