use drasil_mimir::select_addr_of_first_transaction;
use drasil_murin::{
    address::{Address, EnterpriseAddress, StakeCredential},
    crypto::ScriptHash,
    wallet::{self, extract_assets},
    worldmobile::{
        configuration::EnRegistrationConfig,
        enreg::{
            unregister::{AtUnEnRegBuilder, AtUnEnRegParams},
            EnRegistrationDatum,
        },
    },
    AssetName, MultiAsset, MurinError, PerformTxb, TransactionUnspentOutput,
    TransactionUnspentOutputs,
};
use log::debug;

use crate::{create_response, BuildContract};

pub async fn handle_en_unregistration(bc: BuildContract) -> crate::Result<String> {
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
    let ennft_policy = en_reg_config.ennft_policy_id.clone();
    let enop_nft_minting_policy_id: ScriptHash = en_reg_config.enop_nft_minting_policy.hash();
    let mut gtxd = bc.txpattern.into_txdata().await?;
    // check against dataprovider
    let stake_address = &Address::from_bech32(&stake_address)?;
    let network = stake_address.network_id()?;
    let val_cred = StakeCredential::from_scripthash(
        &en_reg_config.registration_validator_smart_contract.hash(),
    );
    let val_address = EnterpriseAddress::new(network, &val_cred).to_address();

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
    let wallet_utxos = gtxd.get_inputs();
    let enopnft_utxos =
        wallet_utxos.find_utxo_containing_policy(&enop_nft_minting_policy_id.to_hex())?;

    if enopnft_utxos.len() != 1 {
        return Err(MurinError::Custom("No valid ENOPNFT found".to_string()));
    }
    let enopnft_utxo = enopnft_utxos.get(0);

    let mut assets = Vec::<(MultiAsset, TransactionUnspentOutput)>::new();
    for utxo in enopnft_utxos {
        let multiassets = extract_assets(&utxo, &enop_nft_minting_policy_id.to_hex())?;
        assets.push((multiassets, utxo));
    }

    debug!("Assets:\n{:?}", assets);

    let first_address = Address::from_bech32(
        &select_addr_of_first_transaction(&stake_address.to_bech32(None)?)
            .map_err(|e| e.to_string())?,
    )?;

    let val_utxos = drasil_mimir::get_address_utxos(&val_address.to_bech32(None).unwrap())
        .map_err(|_| {
            MurinError::Custom("Error, could not retrieve smart contract utxos".to_string())
        })?;
    let ennft_utxo = val_utxos.find_utxos_containing_asset(
        &ennft_policy,
        &AssetName::new(hex::decode(&datum.en_used_nft_tn)?)?,
    )?;

    if ennft_utxo.is_empty() {
        return Err(MurinError::Custom(
            "wallet does not contain any ENNFTs, registration not possible without ENNFT"
                .to_owned(),
        ));
    }

    // search input utxo
    debug!("Specifying ENNFT Input UTxO...");
    if ennft_utxo.len() != 1 {
        return Err(MurinError::Custom("could not select input".to_owned()));
    }
    let input_ennft_utxo = ennft_utxo.get(0).clone();

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

    let datum = EnRegistrationDatum::from_str_datum(&datum)?;

    info!("build transaction...");
    let txb_param: AtUnEnRegParams =
        &drasil_murin::txbuilder::worldmobile::enreg::EnRegistrationTxData {
            first_addr_sender_wallet: Some(first_address),
            ennft_utxo: Some(input_ennft_utxo),
            enopnft_utxo: Some(enopnft_utxo),
            registration_datum: datum,
        };
    let register = AtUnEnRegBuilder::new(txb_param);
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
