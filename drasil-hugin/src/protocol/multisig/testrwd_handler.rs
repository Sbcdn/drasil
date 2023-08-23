use crate::BuildMultiSig;
use crate::CmdError;
use drasil_murin::PerformTxb;

use rand::Rng;

pub(crate) async fn handle_testrewards(bms: &BuildMultiSig) -> crate::Result<String> {
    log::info!("Handle Testreward ....");
    let mut minttxd = bms
        .transaction_pattern()
        .script()
        .unwrap()
        .into_mintdata()
        .await?;
    log::info!("Convert General Transaction Data ....");
    let mut gtxd = bms.transaction_pattern().into_txdata().await?;

    log::info!("Determine network....");
    if gtxd.get_network() != murin::clib::NetworkIdKind::Testnet {
        return Err(CmdError::Custom {
            str: "ERROR: this functions is just for testing".to_string(),
        }
        .into());
    }
    log::info!("Randomize....");
    let t1: Vec<murin::MintTokenAsset> = vec![
        (
            None,
            murin::clib::AssetName::new("ttFLZC".as_bytes().to_vec()).unwrap(),
            murin::clib::utils::to_bignum(138),
        ),
        (
            None,
            murin::clib::AssetName::new("ttSIL".as_bytes().to_vec()).unwrap(),
            murin::clib::utils::to_bignum(142),
        ),
        (
            None,
            murin::clib::AssetName::new("ttDRSL".as_bytes().to_vec()).unwrap(),
            murin::clib::utils::to_bignum(63),
        ),
    ];
    let metadataarray = vec![
    "{\"assets\":[{\"name\":\"ttFLZC\",\"tokenname\":\"ttFLZC\",\"mediaType\":\"image/png\",\"descritpion\":[\"MyDescription\"],\"image_url\":\"nourl\",\"files\":[],\"other\":[]}],\"version\":\"1.0\"}".to_string()
    ,"{\"assets\":[{\"name\":\"ttSIL\",\"tokenname\":\"ttSIL\",\"mediaType\":\"image/png\",\"descritpion\":[\"MyDescription\"],\"image_url\":\"nourl\",\"files\":[],\"other\":[]}],\"version\":\"1.0\"}".to_string()
    ,"{\"assets\":[{\"name\":\"ttDRSL\",\"tokenname\":\"ttDRSL\",\"mediaType\":\"image/png\",\"descritpion\":[\"MyDescription\"],\"image_url\":\"nourl\",\"files\":[],\"other\":[]}],\"version\":\"1.0\"}".to_string()
    ];
    let tns = vec![
        hex::encode("tFLZC".as_bytes()),
        hex::encode("tSIL".as_bytes()),
        hex::encode("tDRSL".as_bytes()),
    ];
    let mut rng: rand::rngs::StdRng = rand::SeedableRng::from_entropy();
    let i: usize = rng.gen_range(0..3);
    let tokens = vec![t1[i].clone()];

    let t_minter_contract_id = 111;
    let t_minter_user_id = 111;
    let sporwc_tcontract_id = 1;
    let sporwc_user_id = 0;

    log::info!("Created raw data!");
    let mut drasildbcon = crate::database::drasildb::establish_connection()?;
    log::info!("Established connection!!");
    let contract =
        crate::drasildb::TBContracts::get_contract_uid_cid(t_minter_user_id, t_minter_contract_id)?;

    let sporwc_flz =
        crate::drasildb::TBContracts::get_contract_uid_cid(sporwc_user_id, sporwc_tcontract_id)?;

    log::info!("Got contract!");
    let contract_address = murin::address::Address::from_bech32(&contract.address).unwrap();
    let _sporwc_cadress = murin::address::Address::from_bech32(&sporwc_flz.address).unwrap();
    let rnd_mintdata = murin::minter::MinterTxData::new(
        tokens,
        None,
        contract_address.clone(), //reward contract address
        serde_json::from_str(&metadataarray[i])
            .expect("ERROR: Could not deserialize metadata in testrewards method"),
        true, //auto_mint
        None,
        None,
        t_minter_contract_id,
    );

    let keyloc = crate::drasildb::TBMultiSigLoc::get_multisig_keyloc(
        &mut drasildbcon,
        &contract.contract_id,
        &contract.user_id,
        &contract.version,
    )?;
    log::info!("Drasil Connection!");
    log::info!("keyloc: {:?}", keyloc);

    if let Some(saddr) = keyloc.fee_wallet_addr {
        match murin::b_decode_addr(&saddr).await {
            Ok(a) => minttxd.set_fee_addr(a),
            Err(e) => {
                return Err(CmdError::Custom {
                    str: format!(
                        "ERROR could not decode signers address: '{:?}'",
                        e.to_string()
                    ),
                }
                .into());
            }
        };
    };

    let minttxd = rnd_mintdata;
    let ns_addr: Option<murin::address::Address> =
        Some(murin::b_decode_addr(&sporwc_flz.address).await?);
    // ToDo:
    //if minttxd.get_to_vendor_script() == true {
    //    ns_addr = Some(contract.address);
    //}
    let ns_script = contract.plutus.clone();

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
    log::info!("DB Sync Slot: {}", slot);
    //ToDO:
    // - Function to check and split utxos when for size >5kB (cal_min_ada panics on utxos >5kB)
    // - Find a solution for protocal parameters (maybe to database?) at the moment they are hardcoded in list / build_rwd
    let policy_script_utxos = mimir::get_address_utxos(&mut dbsync, &contract.address)?;

    gtxd.set_inputs(policy_script_utxos);

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

    info!("Build Successful!");
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
        &[contract.contract_id],
    );
    debug!("RAWTX data: {:?}", tx);
    let used_utxos = tx.get_usedutxos().clone();
    let txh = murin::finalize_rwd(
        &hex::encode(&murin::clib::TransactionWitnessSet::new().to_bytes()),
        tx,
        pkvs,
    )
    .await?;
    murin::utxomngr::usedutxos::store_used_utxos(
        &txh,
        &murin::TransactionUnspentOutputs::from_hex(&used_utxos)?,
    )?;

    use gungnir::FromPrimitive;

    // Create reward entry
    let mut gconn = gungnir::establish_connection()?;
    let policy = hex::encode(
        murin::clib::NativeScript::from_bytes(hex::decode(contract.plutus).unwrap())
            .unwrap()
            .hash()
            .to_bytes(),
    );
    let fingerprint = murin::make_fingerprint(&policy, &tns[i])?;
    let rewards = gungnir::Rewards::get_rewards_per_token(
        &mut gconn,
        &gtxd.get_stake_address().to_bech32(None).unwrap(),
        sporwc_flz.contract_id as i64,
        sporwc_flz.user_id as i64,
        &fingerprint,
    )?;
    let current_epoch = mimir::get_epoch(&mut mimir::establish_connection()?)? as i64;
    println!("Rewards: {:?}", rewards);
    if rewards.len() == 1 && rewards[0].fingerprint == fingerprint {
        let tot_earned = rewards[0].tot_earned.clone()
            + (gungnir::BigDecimal::from_u64(murin::clib::utils::from_bignum(
                &minttxd.get_mint_tokens()[0].2,
            ))
            .unwrap()
                * gungnir::BigDecimal::from_u64(1000000).unwrap());

        let stake_rwd = gungnir::Rewards::update_rewards(
            &mut gconn,
            &rewards[0].stake_addr,
            &rewards[0].fingerprint,
            &rewards[0].contract_id,
            &rewards[0].user_id,
            &tot_earned,
            &current_epoch,
        );
        println!("Stake Rewards Update: {:?}", stake_rwd);
    }
    if rewards.is_empty() {
        let tot_earned = gungnir::BigDecimal::from_u64(murin::clib::utils::from_bignum(
            &minttxd.get_mint_tokens()[0].2,
        ))
        .unwrap()
            * gungnir::BigDecimal::from_u64(1000000).unwrap();

        let stake_rwd = gungnir::Rewards::create_rewards(
            &mut gconn,
            &gtxd.get_stake_address().to_bech32(None).unwrap(),
            &mimir::api::select_addr_of_first_transaction(
                &gtxd
                    .get_stake_address()
                    .to_bech32(None)
                    .expect("ERROR Could not construct bech32 address for stake address"),
            )?,
            &fingerprint,
            &(sporwc_flz.contract_id as i64),
            &(sporwc_flz.user_id as i64),
            &tot_earned,
            &gungnir::BigDecimal::from_i32(0).unwrap(),
            &true,
            &current_epoch,
        );
        println!("Stake Rewards New: {:?}", stake_rwd);
    }

    let ret = "Creating Rewards for you was successfull";
    Ok(ret.to_string())
}
