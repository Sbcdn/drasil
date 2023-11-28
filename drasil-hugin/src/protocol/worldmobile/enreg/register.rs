use drasil_mimir::select_addr_of_first_transaction;
use drasil_murin::{
    address::Address,
    crypto::ScriptHash,
    utils::to_bignum,
    wallet::{self, extract_assets},
    worldmobile::{
        configuration::EnRegistrationConfig,
        enreg::{
            register::{AtEnRegBuilder, AtEnRegParams},
            EnRegistrationDatum,
        },
    },
    MultiAsset, MurinError, PerformTxb, TokenAsset, TransactionUnspentOutput,
    TransactionUnspentOutputs,
};
use log::debug;

use crate::{create_response, BuildContract};

pub async fn handle_en_registration(bc: BuildContract) -> crate::Result<String> {
    let (datum, wallet_addresses, stake_address) = match bc.txpattern.operation() {
        Some(d) => match d {
            crate::Operation::WmEnRegistration {
                datum,
                wallet_addresses,
                stake_address,
            } => (datum, wallet_addresses, stake_address),
            _ => return Err("".into()),
        },
        None => return Err("".into()),
    };
    let en_reg_config = EnRegistrationConfig::load();
    let policy = en_reg_config.ennft_policy_id.clone();
    let mut gtxd = bc.txpattern.into_txdata().await?;
    // check against dataprovider
    let stake_address = &Address::from_bech32(&stake_address)?;

    /*
        let mut txouts = Vec::new();
        if utxos.is_empty() {
            let mut conn = drasil_mimir::establish_connection().map_err(|e| e.to_string())?;
            let utxos_dp = get_stake_address_utxos(&mut conn, &stake_address.to_bech32(None)?)
                .map_err(|e| e.to_string())?;
        }
    */

    let mut dbsync = match drasil_mimir::establish_connection() {
        Ok(conn) => conn,
        Err(e) => {
            return Err(MurinError::new(&format!(
                "ERROR could not connect to dbsync: '{:?}'",
                e.to_string()
            )));
        }
    };

    let wallet_addresses = wallet_addresses
        .iter()
        .fold(Vec::<Address>::new(), |mut acc, n| {
            acc.push(Address::from_bech32(&n).unwrap());
            acc
        });

    if !wallet_addresses.is_empty() {
        let wallet_utxos =
            wallet_addresses
                .iter()
                .fold(TransactionUnspentOutputs::new(), |mut acc, n| {
                    acc.merge(
                        drasil_mimir::get_address_utxos(&n.to_bech32(None).unwrap()).unwrap(),
                    );
                    acc
                });
        gtxd.set_inputs(wallet_utxos);

        // ToDo: go through all addresses and check all stake keys are equal
        let sa = wallet::reward_address_from_address(&wallet_addresses[0])?;
        gtxd.set_stake_address(sa);
        gtxd.set_senders_addresses(wallet_addresses.clone());
    }
    let utxos = gtxd.get_inputs();
    let ennft_utxos = utxos.find_utxo_containing_policy(&policy.to_hex())?;

    let mut assets = Vec::<(MultiAsset, TransactionUnspentOutput)>::new();
    for utxo in ennft_utxos {
        let multiassets = extract_assets(&utxo, &policy.to_hex())?;
        assets.push((multiassets, utxo));
    }

    debug!("Assets:\n{:?}", assets);

    let mut ennfts_cip30 = Vec::<(Address, TokenAsset)>::new();

    for asset in assets {
        let assets = asset.0.get(&policy).unwrap();
        let asset_names = assets.keys();
        for i in 0..assets.len() {
            let an = asset_names.get(i);
            let amt = assets.get(&an).unwrap();
            ennfts_cip30.push((asset.1.output().address(), (policy.clone(), an, amt)));
        }
    }

    let ennft_utxos = utxos.find_utxo_containing_policy(&policy.to_hex())?;

    let mut assets = Vec::<(MultiAsset, TransactionUnspentOutput)>::new();
    for utxo in ennft_utxos {
        let multiassets = extract_assets(&utxo, &policy.to_hex())?;
        assets.push((multiassets, utxo));
    }

    debug!("Assets:\n{:?}", assets);

    for asset in assets {
        let assets = asset.0.get(&policy).unwrap();
        let asset_names = assets.keys();
        for i in 0..assets.len() {
            let an = asset_names.get(i);
            let amt = assets.get(&an).unwrap();
            ennfts_cip30.push((asset.1.output().address(), (policy.clone(), an, amt)));
        }
    }

    let first_address = Address::from_bech32(
        &select_addr_of_first_transaction(&stake_address.to_bech32(None)?)
            .map_err(|e| e.to_string())?,
    )?;

    let ennft_utxo = utxos.find_utxo_containing_policy(&policy.to_hex())?;

    if ennft_utxo.is_empty() {
        return Err(MurinError::Custom(
            "wallet does not contain any ENNFTs, registration not possible without ENNFT"
                .to_owned(),
        ));
    }

    let mut ennfts = Vec::<(Address, TokenAsset)>::new();

    for utxo in ennft_utxo.clone() {
        let ma = utxo.output().amount().multiasset().unwrap();
        let sh = ScriptHash::from_bytes(hex::decode(&policy.to_hex())?)?;
        let assets = ma.get(&sh).unwrap();
        let asset_names = assets.keys();
        for i in 0..assets.len() {
            let an = asset_names.get(i);
            let amt = assets.get(&an).unwrap();
            ennfts.push((utxo.output().address(), (sh.clone(), an, amt)));
        }
    }
    debug!("ENNFTS:\n{:?}", &ennfts);
    let datum = EnRegistrationDatum::from_str_datum(&datum)?;
    let mut valid_ennfts = ennfts.clone();
    valid_ennfts = valid_ennfts
        .iter()
        .filter(|n| n.1 .1 == datum.en_used_nft_tn)
        .map(|n| n.to_owned())
        .collect();

    if valid_ennfts.is_empty() {
        return Err(MurinError::Custom(
            "wallet does not contain valid ENNFTs, please speicfy which ENNFT to use if you have several".to_owned(),
        ));
    }

    // search input utxo
    debug!("Specifying ENNFT Input UTxO...");
    let input_utxo: Vec<_> = ennft_utxo
        .filter(|n| {
            n.output()
                .amount()
                .multiasset()
                .unwrap()
                .get_asset(&valid_ennfts[0].1 .0, &valid_ennfts[0].1 .1)
                .compare(&to_bignum(1))
                == 0
        })
        .collect();

    if input_utxo.len() != 1 {
        return Err(MurinError::Custom("could not select input".to_owned()));
    }
    let input_utxo = input_utxo[0].clone();

    log::debug!("Try to determine slot...");

    let slot = match drasil_mimir::get_slot(&mut dbsync) {
        Ok(s) => s,
        Err(e) => {
            return Err(MurinError::new(&format!(
                "ERROR could not determine current slot: '{:?}'",
                e.to_string()
            )))
        }
    };
    gtxd.set_current_slot(slot as u64);

    info!("build transaction...");
    let txb_param: AtEnRegParams =
        &drasil_murin::txbuilder::worldmobile::enreg::EnRegistrationTxData {
            first_addr_sender_wallet: Some(first_address),
            ennft_utxo: Some(input_utxo),
            enopnft_utxo: None,
            registration_datum: datum,
        };
    let register = AtEnRegBuilder::new(txb_param);
    let builder = drasil_murin::TxBuilder::new(&gtxd, &vec![]);
    let bld_tx = builder.build(&register).await?;

    info!("post processing transaction...");
    let tx = drasil_murin::utxomngr::RawTx::new(
        &bld_tx.get_tx_body(),
        &bld_tx.get_txwitness(),
        &bld_tx.get_tx_unsigned(),
        &bld_tx.get_metadata(),
        &gtxd.to_string(),
        &txb_param.to_string(),
        &bld_tx.get_used_utxos(),
        &hex::encode(gtxd.get_stake_address().to_bytes()),
        &(bc.customer_id()),
        &gtxd.get_contract_id().unwrap(),
    );
    trace!("RAWTX data: {:?}", tx);

    info!("create response...");
    let ret = create_response(
        &bld_tx,
        &tx,
        bc.transaction_pattern().wallet_type().as_ref(),
    )?;

    Ok(ret.to_string())
}
