use super::RegistrationDatum;
use crate::{
    address_from_string_non_async, calc_min_ada_for_utxo, find_utxos_by_address,
    modules::{
        txtools::utxo_handling::input_selection,
        worldmobile::aya::{
            enregistration::{
                error::TransactionBuildingError, RegistrationRedeemer, ENREGCONTRACT,
            },
            models::{
                BuilderResult, EarthNodeConfig, RegisterEarthNode, TransactionSchema,
                UnsignedTransaction,
            },
        },
    },
    payment_keyhash_from_address, transaction_unspent_outputs_from_string,
    transaction_unspent_outputs_from_string_vec,
    txbuilders::get_input_position,
    TransactionUnspentOutputs,
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
use sha2::Digest;

pub(crate) async fn admin_register_en(
    tx_schema: TransactionSchema,
) -> Result<BuilderResult, TransactionBuildingError> {
    let op_data = match tx_schema.operation {
        Some(d) => serde_json::from_value::<RegisterEarthNode>(d)?,
        None => return Err(TransactionBuildingError::StandardTransactionBuildingError),
    };

    let policy_str =
        std::env::var("ENNFT_POLICY").expect("No ENNFT policyID set for this tx-building service");
    let policy = ScriptHash::from_bytes(hex::decode(&policy_str)?)?;

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

    let collateral = transaction_unspent_outputs_from_string_vec(
        tx_schema
            .collateral
            .as_ref()
            .expect("no collateral utxos provided"),
        None,
        None,
    )
    .into()?;

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
    debug!("Wallet UTxOs empty: {:?}\n", utxos.is_empty());
    let first_address = first_transaction_from_stake_addr(&stake_address).await?;
    let first_pkh = BaseAddress::from_address(&first_address)
        .unwrap()
        .payment_cred()
        .to_keyhash()
        .unwrap();
    log::debug!(
        "\nPubKeyHash First Address: {}\n",
        hex::encode(first_pkh.to_bytes())
    );
    let script_address = EnterpriseAddress::new(first_address.network_id()?, &cred).to_address();
    log::debug!("\nScript Address: {}\n", script_address.to_bech32(None)?);

    let sutxos = dp
        .script_utxos(&script_address.to_bech32(None)?)
        .await
        .unwrap();

    let ennft_tokeninfo = dp.token_info(&op_data.ennft_assetname).await?;
    let script_utxos = sutxos.find_utxos_containing_asset(
        &policy,
        //&ScriptHash::from_bytes(hex::decode(
        //"d8bebcb0abd89193874c59ed3023f5b4f81b89b6676d187ad7fbdb0e",
        //)?)?,
        &AssetName::new(hex::decode(&ennft_tokeninfo.tokenname)?)?,
    )?;

    let pubkeyhash = payment_keyhash_from_address(&first_address).into()?;

    if script_utxos.len() != 1 {
        return Err(TransactionBuildingError::Custom(
            "smart contract does not contain the specified ENNFT".to_owned(),
        ));
    }
    //assert_eq!(ennft_tokeninfo.policy, policy_str);
    let script_utxo = script_utxos.get(0);
    log::debug!("Try to restore datum...");
    if script_utxo.output().plutus_data().is_none() {
        return Err(TransactionBuildingError::Custom(
            "the utxo of the ENNFT does not contain the correct datum, invalid registration"
                .to_owned(),
        ));
    }

    let datum = script_utxo.output().plutus_data().unwrap();
    let d_str = datum
        .to_json(cardano_serialization_lib::plutus::PlutusDatumSchema::DetailedSchema)?
        .clone();
    let d_svalue = serde_json::from_str::<serde_json::Value>(&d_str)?;
    log::debug!("Deserialized Datum: \n{:?}", &d_str);
    let fields = d_svalue.get("fields").unwrap().as_array().unwrap();
    let operator_address = hex::decode(
        fields[0]
            .as_object()
            .unwrap()
            .get("bytes")
            .unwrap()
            .as_str()
            .unwrap(),
    )?;
    let validator_address = hex::decode(
        fields[1]
            .as_object()
            .unwrap()
            .get("bytes")
            .unwrap()
            .as_str()
            .unwrap(),
    )?;
    let en_moniker = hex::decode(
        fields[2]
            .as_object()
            .unwrap()
            .get("bytes")
            .unwrap()
            .as_str()
            .unwrap(),
    )?;
    let regdat = RegistrationDatum {
        validator_address,
        operator_address,
        moniker: en_moniker,
        enUsedNftTn: AssetName::new(hex::decode(
            fields[3]
                .as_object()
                .unwrap()
                .get("bytes")
                .unwrap()
                .as_str()
                .unwrap(),
        )?)?,
        enOwner: Ed25519KeyHash::from_bytes(hex::decode(
            fields[4]
                .as_object()
                .unwrap()
                .get("bytes")
                .unwrap()
                .as_str()
                .unwrap(),
        )?)?,
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

    // Create Plutus Datum
    //let mut inner = plutus::PlutusMap::new();
    //inner.insert(0, &PlutusData::new_bytes(regdat.enConfigHash.to_vec()));
    //inner.add(&PlutusData::new_bytes(regdat.enPoolID.to_vec()));
    //inner.add(&PlutusData::new_bytes(regdat.enPoolTicker.to_vec()));
    //inner.add(&PlutusData::new_bytes(regdat.enUsedNftTn.name()));
    //inner.add(&PlutusData::new_bytes(regdat.enOwner.to_bytes()));

    let datum_ = &plutus::PlutusData::new_constr_plutus_data(&plutus::ConstrPlutusData::new(
        &to_bignum(0),
        &inner,
    ));

    let mut datums_ = plutus::PlutusList::new();
    datums_.add(datum_);
    let datumhash = hash_plutus_data(datum_);
    log::info!("DatumHash: {:?}\n", hex::encode(datumhash.to_bytes()));

    //// ToDo: Lookup utxo with datumhash on dbsync
    //let contract_utxo = dp
    //    .utxo_by_dataumhash(&script_address.to_bech32(None)?, &datumhash.to_bytes())
    //    .await?;

    // Verify inline datum match provided data

    // Verify Owner Pubkeyhash offchain

    // Create unregistration output containing the ENNFT from the smartcontract,
    // sending the ENNFT fback to the owner
    let mut unregistration_value = script_utxo.output().amount();

    unregistration_value.set_coin(&calc_min_ada_for_utxo(&unregistration_value, None));
    debug!("Registration Value: {:?}", unregistration_value);
    builder.add_output(&TransactionOutput::new(
        &first_address,
        &unregistration_value,
    ))?;

    let mem = to_bignum(7000000u64); //cutils::to_bignum(7000000u64);
    let steps = to_bignum(2500000000u64);
    let exunits = ExUnits::new(&mem, &steps);

    // Add required signers
    builder.add_required_signer(&pubkeyhash);

    let mut needed = Value::new(&to_bignum(3000000));
    let inputs =
        input_selection(None, &mut needed, &utxos, Some(collateral.get(0)), None).into()?;
    let admin_addr = Address::from_bech32("addr_test1qrkt8rppznv4hxfrk6c4uvgvy9dhcp6y7c6hkukx70pqsg7p5n9fnvekx7cv5kye9k5xwlrqgylxlu4hdc7d85mhu6yqmwlh9s").unwrap();
    let req_signer_inputs = find_utxos_by_address(admin_addr.clone(), &utxos).0;

    if req_signer_inputs.is_empty() {
        return Err(TransactionBuildingError::Custom(format!(
            "cannot add UTxOs of the required signer, please populate address: {:?} with UTxOs",
            first_address.clone()
        )));
    }

    let mut txis = inputs.0.clone();
    txis.add(&script_utxo.input());

    let signer_check = !inputs.1.contains_address(admin_addr);
    if signer_check {
        txis.add(&req_signer_inputs.get(0).input());
    }

    let index = get_input_position(txis, script_utxo.clone());
    let mut txinbuilder = TxInputsBuilder::new();

    for i in inputs.1 {
        txinbuilder.add_input(&i.output().address(), &i.input(), &i.output().amount())
    }
    if signer_check {
        txinbuilder.add_input(
            &req_signer_inputs.get(0).output().address(),
            &req_signer_inputs.get(0).input(),
            &req_signer_inputs.get(0).output().amount(),
        );
    }

    let redeemer_data = plutus::PlutusData::new_constr_plutus_data(&plutus::ConstrPlutusData::new(
        &to_bignum(2), //&RegistrationRedeemer::Admin.redeemer(),
        &plutus::PlutusList::new(),
    ));

    let redeemer = Redeemer::new(
        &RedeemerTag::new_spend(),
        &to_bignum(index.0 as u64),
        &redeemer_data,
        &exunits,
    );
    let witness = PlutusWitness::new_with_ref(
        &PlutusScriptSource::new(&smartcontract),
        &DatumSource::new_ref_input(&script_utxo.input()),
        &redeemer,
    );
    //let plist = PlutusList::new();
    //let witness = PlutusWitness::new(&smartcontract, &datum, &redeemer);

    txinbuilder.add_plutus_script_input(
        &witness,
        &script_utxo.input(),
        &script_utxo.output().amount(),
    );
    builder.set_inputs(&txinbuilder);

    let mut colbuilder = TxInputsBuilder::new();
    let collateral = collateral.get(0);
    colbuilder.add_input(
        &collateral.output().address(),
        &collateral.input(),
        &collateral.output().amount(),
    );
    builder.set_collateral(&colbuilder);

    let protocol_parameters = crate::pparams::ProtocolParameters::read_protocol_parameter(
        &std::env::var("PPPATH").unwrap_or_else(|_| "protocol_parameters_preview.json".to_owned()),
    )
    .unwrap();
    // CostModel
    let cost_models = protocol_parameters.get_CostMdls().unwrap();
    let costmodel = cost_models.get(&plutus::Language::new_plutus_v2()).unwrap();
    let mut pcm = plutus::CostModel::new();
    for i in 0..costmodel.len() {
        pcm.set(i, &pcm.get(i)?)?;
    }
    let mut cstmodls = plutus::Costmdls::new();
    cstmodls.insert(&plutus::Language::new_plutus_v2(), &pcm);

    let costmodel = cost_models.get(&plutus::Language::new_plutus_v2()).unwrap();
    let mut cstmodls_ = plutus::Costmdls::new();
    cstmodls_.insert(&plutus::Language::new_plutus_v2(), &costmodel);

    let mut redeemers = plutus::Redeemers::new();
    redeemers.add(&redeemer);
    log::debug!("\nCostModels:\n{:?}\n\n", cstmodls_);

    let scriptdatahash =
        cardano_serialization_lib::utils::hash_script_data(&redeemers, &cstmodls_, None);
    log::debug!(
        "ScriptDataHash: {:?}\n",
        hex::encode(scriptdatahash.to_bytes())
    );
    builder.set_script_data_hash(&scriptdatahash);
    builder.add_change_if_needed(&Address::from_hex(&tx_schema.change_address.unwrap())?)?;
    builder.remove_script_data_hash();
    builder.calc_script_data_hash(&cstmodls)?;

    let tx = builder.build_tx()?;

    Ok(BuilderResult::UnsignedTransaction(UnsignedTransaction {
        id: "test_id_register_earth_node".to_string(),
        tx: tx.to_hex(),
    }))
}
