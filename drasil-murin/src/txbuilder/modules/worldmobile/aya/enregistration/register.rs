use super::RegistrationDatum;
use crate::{
    address_from_string_non_async, calc_min_ada_for_utxo, extract_assets, find_utxos_by_address,
    make_fingerprint, min_ada_for_utxo,
    modules::{
        txtools::utxo_handling::input_selection,
        worldmobile::aya::{
            enregistration::{
                error::TransactionBuildingError, RegistrationRedeemer, ENREGCONTRACT,
            },
            models::{
                BuilderResult, EarthNodeConfig, RegisterEarthNode, Token, TransactionSchema,
                UnsignedTransaction,
            },
        },
    },
    payment_keyhash_from_address, transaction_unspent_outputs_from_string,
    transaction_unspent_outputs_from_string_vec,
    txbuilders::get_input_position,
    TransactionUnspentOutput, TransactionUnspentOutputs,
};
use cardano_serialization_lib::{
    address::{Address, BaseAddress, EnterpriseAddress, StakeCredential},
    crypto::{Ed25519KeyHash, ScriptDataHash, ScriptHash},
    fees::LinearFee,
    plutus::{
        self, ExUnitPrices, ExUnits, Languages, PlutusData, PlutusList, PlutusScript, Redeemer,
        RedeemerTag,
    },
    tx_builder::{
        tx_inputs_builder::{
            DatumSource, PlutusScriptSource, PlutusWitness, PlutusWitnesses, TxInputsBuilder,
        },
        CoinSelectionStrategyCIP2, TransactionBuilder, TransactionBuilderConfigBuilder,
    },
    utils::{hash_plutus_data, to_bignum, Value},
    AssetName, Assets, MultiAsset, TransactionInputs, TransactionOutput, UnitInterval,
};
use log::debug;
use serde::{Deserialize, Serialize};
use sha2::Digest;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ENRegMetadata {
    operator_address: String,
    validator_address: String,
    moniker: String,
    ennft: String,
}

pub(crate) async fn register_earth_node(
    tx_schema: TransactionSchema,
) -> Result<BuilderResult, TransactionBuildingError> {
    let op_data = match tx_schema.operation {
        Some(d) => serde_json::from_value::<RegisterEarthNode>(d)?,
        None => return Err(TransactionBuildingError::StandardTransactionBuildingError),
    };

    let policy =
        std::env::var("ENNFT_POLICY").expect("No ENNFT policyID set for this tx-building service");

    let smartcontract = PlutusScript::from_bytes_v2(hex::decode(ENREGCONTRACT.as_str())?)?;
    let scripthash = smartcontract.hash();
    let cred = StakeCredential::from_scripthash(&scripthash);

    // check against wallet
    let utxos: TransactionUnspentOutputs = transaction_unspent_outputs_from_string_vec(
        &tx_schema.utxos.unwrap(),
        tx_schema.collateral.as_ref(),
        tx_schema.excludes.as_ref(),
    )
    .into()?;

    let ennft_utxos: TransactionUnspentOutputs =
        utxos.find_utxo_containing_policy(&policy).into()?;

    let mut assets = Vec::<(MultiAsset, TransactionUnspentOutput)>::new();
    for utxo in ennft_utxos {
        let multiassets = extract_assets(&utxo, &policy).into()?;
        assets.push((multiassets, utxo));
    }

    debug!("Assets:\n{:?}", assets);

    let mut ennfts_cip30 = Vec::<(Address, Token)>::new();

    for asset in assets {
        let sh = ScriptHash::from_bytes(hex::decode(&policy)?)?;
        let assets = asset.0.get(&sh).unwrap();
        let asset_names = assets.keys();
        for i in 0..assets.len() {
            let an = asset_names.get(i);
            let amt = assets.get(&an).unwrap();
            ennfts_cip30.push((asset.1.output().address(), (sh.clone(), an, amt)));
        }
    }

    // check against dataprovider
    //let dp = cdp::DataProvider::new(cdp::DBSyncProvider::new(cdp::Config {
    //    db_path: std::env::var("DBSYNC_URL").unwrap(),
    //}));

    let stake_address = if let Some(stake_addr) = tx_schema.stake_address {
        address_from_string_non_async(&stake_addr[0].clone())
            .unwrap()
            .to_bech32(None)
            .unwrap()
    } else {
        return Err(TransactionBuildingError::RewardAddressNotFound);
    };
    debug!("Stake Address: {}", stake_address);
    let utxos_dp = dp.wallet_utxos(&stake_address).await.unwrap();
    debug!("\n\nWallet UTxOs empty: {:?}\n", &utxos_dp);
    let first_address = dp.first_transaction_from_stake_addr(&stake_address).await?;
    let script_address = EnterpriseAddress::new(first_address.network_id()?, &cred).to_address();
    log::debug!("\nScript Address: {}\n", script_address.to_bech32(None)?);

    let pubkeyhash = payment_keyhash_from_address(&first_address).into()?;

    let ennft_utxo = utxos_dp.find_utxo_containing_policy(&policy)?;

    if ennft_utxo.is_empty() {
        return Err(TransactionBuildingError::Custom(
            "wallet does not contain any ENNFTs, registration not possible without ENNFT"
                .to_owned(),
        ));
    }

    let mut ennfts = Vec::<(Address, Token)>::new();

    for utxo in ennft_utxo.clone() {
        let ma = utxo.output().amount().multiasset().unwrap();
        let sh = ScriptHash::from_bytes(hex::decode(&policy)?)?;
        let assets = ma.get(&sh).unwrap();
        let asset_names = assets.keys();
        for i in 0..assets.len() {
            let an = asset_names.get(i);
            let amt = assets.get(&an).unwrap();
            ennfts.push((utxo.output().address(), (sh.clone(), an, amt)));
        }
    }
    debug!("ENNFTS:\n{:?}", &ennfts);
    //ToDo: assert_eq fails sometimes due to ordering, build a test function which checks on equal content
    //assert_eq!(ennfts, ennfts_cip30);

    let token_info = dp.token_info(&op_data.ennft_assetname).await?;
    assert_eq!(token_info.policy, policy);

    let mut valid_ennfts = ennfts.clone();
    valid_ennfts = valid_ennfts
        .iter()
        .filter(|n| n.1 .1 == AssetName::new(hex::decode(&token_info.tokenname).unwrap()).unwrap())
        .map(|n| n.to_owned())
        .collect();

    if valid_ennfts.is_empty() {
        return Err(TransactionBuildingError::Custom(
            "wallet does not contain valid ENNFTs, please speicfy which ENNFT to use if you have several".to_owned(),
        ));
    }

    // search input utxo
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
        return Err(TransactionBuildingError::Custom(
            "could not select input".to_owned(),
        ));
    }
    let input_utxo = input_utxo[0].clone();

    // Create specific config hash
    let ennft_fingerprint = make_fingerprint(
        &valid_ennfts[0].1 .0.to_hex(),
        &hex::encode(valid_ennfts[0].1 .1.name()),
    )
    .into()?;

    //let stake_address = get_stakeaddr_from_addr(&valid_ennfts[0].0)?;

    // Registration Datum
    let regdat = RegistrationDatum {
        operator_address: op_data.config.operator_address.as_bytes().to_vec(),
        validator_address: op_data.config.validator_address.as_bytes().to_vec(),
        moniker: op_data.config.moniker.as_bytes().to_vec(),
        enUsedNftTn: valid_ennfts[0].1 .1.clone(),
        enOwner: pubkeyhash.clone(),
    };

    //
    // Transaction Building
    //
    let mut builderconfig = TransactionBuilderConfigBuilder::new();
    builderconfig = builderconfig.fee_algo(&LinearFee::new(&to_bignum(44), &to_bignum(155381)));
    builderconfig = builderconfig.pool_deposit(&to_bignum(500000000));
    builderconfig = builderconfig.key_deposit(&to_bignum(2000000));
    builderconfig = builderconfig.max_value_size(5000);
    builderconfig = builderconfig.max_tx_size(16384);
    builderconfig = builderconfig.coins_per_utxo_byte(&to_bignum(4310));
    builderconfig = builderconfig.ex_unit_prices(&ExUnitPrices::new(
        &UnitInterval::new(&to_bignum(577), &to_bignum(10000)),
        &UnitInterval::new(&to_bignum(721), &to_bignum(10000000)),
    ));
    builderconfig = builderconfig.prefer_pure_change(false);

    let builderconfig = builderconfig.build()?;
    let mut builder = TransactionBuilder::new(&builderconfig);

    // Create Plutus Datum
    let mut inner = plutus::PlutusList::new();
    inner.add(&PlutusData::new_bytes(regdat.operator_address.to_vec()));
    inner.add(&PlutusData::new_bytes(regdat.validator_address.to_vec()));
    inner.add(&PlutusData::new_bytes(regdat.moniker.to_vec()));
    inner.add(&PlutusData::new_bytes(regdat.enUsedNftTn.name()));
    inner.add(&PlutusData::new_bytes(regdat.enOwner.to_bytes()));

    let datum = &plutus::PlutusData::new_constr_plutus_data(&plutus::ConstrPlutusData::new(
        &to_bignum(0),
        &inner,
    ));
    log::info!("\nDatum: {:?}", datum);

    let mut datums = plutus::PlutusList::new();
    datums.add(datum);
    let datumhash = hash_plutus_data(datum);
    log::info!("DatumHash: {:?}\n", hex::encode(datumhash.to_bytes()));

    // Create registration output containing a valid ENNFT,
    // sending the ENNFT from its source address to the smart contract and apply datum
    let mut registration_value = Value::zero();
    let mut multi_assets = MultiAsset::new();
    let mut assets = Assets::new();
    assets.insert(&valid_ennfts[0].1 .1, &valid_ennfts[0].1 .2);
    multi_assets.insert(&valid_ennfts[0].1 .0, &assets);
    registration_value.set_multiasset(&multi_assets);

    //registration_value.set_coin(&calc_min_ada_for_utxo(
    //    &registration_value,
    //    Some(datumhash),
    //)?);

    debug!("Registration Value: {:?}", registration_value);
    let mut registration_output = TransactionOutput::new(&script_address, &registration_value);
    registration_output.set_plutus_data(datum);
    let registration_output = min_ada_for_utxo(&registration_output).into()?;
    builder.add_output(&registration_output)?;

    debug!("Policy: {:?}", valid_ennfts[0].1 .0.to_hex());
    debug!("Name: {:?}", &hex::encode(valid_ennfts[0].1 .1.name()));

    // Add required signers
    builder.add_required_signer(&pubkeyhash);

    // Metadata
    let registration_metadata = ENRegMetadata {
        operator_address: op_data.config.operator_address,
        validator_address: op_data.config.validator_address,
        moniker: op_data.config.moniker,
        ennft: ennft_fingerprint,
    };
    builder.add_json_metadatum(
        &to_bignum(9819543),
        serde_json::to_string(&registration_metadata)?,
    )?;
    debug!("Metadata: {:?}", &registration_metadata);

    builder.add_inputs_from(
        &utxos.convert_to_csl(),
        CoinSelectionStrategyCIP2::RandomImproveMultiAsset,
    )?;

    let mut diff = match input_utxo
        .output()
        .amount()
        .checked_sub(&registration_value)
    {
        Ok(amount) => amount,
        Err(_) => match registration_value.checked_sub(&input_utxo.output().amount()) {
            Ok(amount) => amount,
            Err(_) => {
                return Err(TransactionBuildingError::Custom(
                    "invalid inputs".to_owned(),
                ))
            }
        },
    };

    let minada_diff = calc_min_ada_for_utxo(&diff, None);
    diff.set_coin(&minada_diff);

    let mut needed = registration_value
        .checked_add(&diff)?
        .checked_add(&Value::new(&to_bignum(2000000)))?;
    let inputs = input_selection(None, &mut needed, &utxos, None, None).into()?;

    let mut ibuilder = TxInputsBuilder::new();

    for i in inputs.1 {
        ibuilder.add_input(&i.output().address(), &i.input(), &i.output().amount())
    }

    builder.set_inputs(&ibuilder);

    debug!("added inputs: {:?}", &builder.get_total_input()?);
    debug!("added outputs: {:?}", &builder.get_total_output()?);

    builder.add_change_if_needed(&first_address)?; //Address::from_hex(&tx_schema.change_address.unwrap())?

    let tx = builder.build_tx()?;

    Ok(BuilderResult::UnsignedTransaction(UnsignedTransaction {
        id: "test_id_register_earth_node".to_string(),
        tx: tx.to_hex(),
    }))
}
