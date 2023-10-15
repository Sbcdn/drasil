use cdp::provider::CardanoDataProvider;
use cdp::{DBSyncProvider, DataProvider};
use murin::address::{Address, BaseAddress, EnterpriseAddress, StakeCredential};
use murin::cardano;
use murin::crypto::{Ed25519KeyHash, ScriptHash};
use murin::fees::LinearFee;
use murin::plutus::{ConstrPlutusData, CostModel, Costmdls, ExUnitPrices, ExUnits, Language};
use murin::plutus::{
    PlutusData, PlutusDatumSchema, PlutusList, PlutusScript, Redeemer, RedeemerTag, Redeemers,
};
use murin::pparams::ProtocolParameters;
use murin::tx_builder::tx_inputs_builder::{
    DatumSource, PlutusScriptSource, PlutusWitness, TxInputsBuilder,
};
use murin::tx_builder::{TransactionBuilder, TransactionBuilderConfigBuilder};
use murin::utils::{self, to_bignum, Value};
use murin::{wallet, TransactionUnspentOutputs};
use murin::{AssetName, TransactionOutput, UnitInterval};

use super::RegistrationDatum;
use crate::config::RegistrationConfig;
use crate::error::Error;
use crate::models::{BuilderResult, RegisterEarthNode, TransactionSchema, UnsignedTransaction};
use crate::registration::RegistrationRedeemer;

pub(crate) async fn unregister_earth_node(
    config: RegistrationConfig,
    provider: DataProvider<DBSyncProvider>,
    tx_schema: TransactionSchema,
) -> Result<BuilderResult, Error> {
    let op_data = match tx_schema.operation {
        Some(d) => serde_json::from_value::<RegisterEarthNode>(d)?,
        None => return Err(Error::StandardTransactionBuildingError),
    };

    let policy = ScriptHash::from_bytes(hex::decode(&config.policy)?)?;
    let smartcontract = PlutusScript::from_bytes_v2(hex::decode(&config.contract)?)?;
    let scripthash = smartcontract.hash();
    let cred = StakeCredential::from_scripthash(&scripthash);

    // check against wallet
    let utxos: TransactionUnspentOutputs = wallet::transaction_unspent_outputs_from_string_vec(
        &tx_schema.utxos.unwrap(),
        tx_schema.collateral.as_ref(),
        tx_schema.excludes.as_ref(),
    )?;

    let collateral = wallet::transaction_unspent_outputs_from_string_vec(
        tx_schema
            .collateral
            .as_ref()
            .expect("no collateral utxos provided"),
        None,
        None,
    )?;

    let stake_address = if let Some(stake_addr) = tx_schema.stake_address {
        wallet::address_from_string_non_async(&stake_addr[0].clone())
            .unwrap()
            .to_bech32(None)
            .unwrap()
    } else {
        return Err(Error::RewardAddressNotFound);
    };

    tracing::debug!("Stake Address: {}", stake_address);
    tracing::debug!("Wallet UTxOs empty: {:?}\n", utxos.is_empty());

    let first_address = provider
        .first_transaction_from_stake_addr(&stake_address)
        .await?;
    let first_pkh = BaseAddress::from_address(&first_address)
        .unwrap()
        .payment_cred()
        .to_keyhash()
        .unwrap();

    tracing::debug!(
        "\nPubKeyHash First Address: {}\n",
        hex::encode(first_pkh.to_bytes())
    );
    let script_address = EnterpriseAddress::new(first_address.network_id()?, &cred).to_address();
    tracing::debug!("\nScript Address: {}\n", script_address.to_bech32(None)?);

    let sutxos = provider
        .script_utxos(&script_address.to_bech32(None)?)
        .await
        .unwrap();

    let ennft_tokeninfo = provider.token_info(&op_data.ennft_assetname).await?;
    let assert_name = AssetName::new(hex::decode(&ennft_tokeninfo.tokenname)?)?;
    let script_utxos = sutxos
        .find_utxos_containing_asset(&policy, &assert_name)
        .map_err(|err| Error::Custom(err.to_string()))?;

    let pubkeyhash = wallet::payment_keyhash_from_address(&first_address)?;

    if script_utxos.len() != 1 {
        return Err(Error::Custom(
            "smart contract does not contain the specified ENNFT".to_owned(),
        ));
    }
    // TODO: UNCOMMENT
    assert_eq!(ennft_tokeninfo.policy, config.policy);
    let script_utxo = script_utxos.get(0);
    tracing::debug!("Try to restore datum...");
    if script_utxo.output().plutus_data().is_none() {
        return Err(Error::Custom(
            "the utxo of the ENNFT does not contain a datum".to_owned(),
        ));
    }

    let datum = script_utxo.output().plutus_data().unwrap();
    let d_str = datum
        .to_json(PlutusDatumSchema::DetailedSchema)
        .map_err(murin::MurinError::from)?;

    let d_svalue = serde_json::from_str::<serde_json::Value>(&d_str)?;
    tracing::debug!("Deserialized Datum: \n{:?}", &d_str);
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

    let used_nft = AssetName::new(hex::decode(
        fields[3]
            .as_object()
            .unwrap()
            .get("bytes")
            .unwrap()
            .as_str()
            .unwrap(),
    )?)?;
    let owner = Ed25519KeyHash::from_bytes(hex::decode(
        fields[4]
            .as_object()
            .unwrap()
            .get("bytes")
            .unwrap()
            .as_str()
            .unwrap(),
    )?)?;
    let regdat = RegistrationDatum {
        validator_address: validator_address.to_vec(),
        operator_address: operator_address.to_vec(),
        moniker: en_moniker.to_vec(),
        used_nft,
        owner,
    };

    tracing::debug!("\nRestored Inline Datum: {:?}\n", &regdat);
    // Registration Datum
    let regdat_r = RegistrationDatum {
        operator_address: op_data.config.operator_address.as_bytes().to_vec(),
        validator_address: op_data.config.validator_address.as_bytes().to_vec(),
        moniker: op_data.config.moniker.as_bytes().to_vec(),
        used_nft: AssetName::new(hex::decode(&ennft_tokeninfo.tokenname)?)?,
        owner: pubkeyhash.clone(),
    };
    tracing::debug!("\nBuilt Datum: {:?}\n", &regdat_r);
    assert_eq!(
        regdat, regdat_r,
        "send and restored datums: {:?} and \n{:?}",
        regdat, regdat_r
    );
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
    let mut inner = PlutusList::new();
    inner.add(&PlutusData::new_bytes(regdat.operator_address.to_vec()));
    inner.add(&PlutusData::new_bytes(regdat.validator_address.to_vec()));
    inner.add(&PlutusData::new_bytes(regdat.moniker.to_vec()));
    inner.add(&PlutusData::new_bytes(regdat.used_nft.name()));
    inner.add(&PlutusData::new_bytes(regdat.owner.to_bytes()));

    let datum_ = &PlutusData::new_constr_plutus_data(&ConstrPlutusData::new(&to_bignum(0), &inner));

    let mut datums_ = PlutusList::new();
    datums_.add(datum_);
    let datumhash = utils::hash_plutus_data(datum_);
    tracing::info!("DatumHash: {:?}\n", hex::encode(datumhash.to_bytes()));

    //// ToDo: Lookup utxo with datumhash on dbsync
    //let contract_utxo = dp
    //    .utxo_by_dataumhash(&script_address.to_bech32(None)?, &datumhash.to_bytes())
    //    .await?;

    // Verify inline datum match provided data

    // Verify Owner Pubkeyhash offchain

    // Create unregistration output containing the ENNFT from the smartcontract,
    // sending the ENNFT fback to the owner
    let mut unregistration_value = script_utxo.output().amount();

    unregistration_value.set_coin(&murin::calc_min_ada_for_utxo(&unregistration_value, None));
    tracing::debug!("Registration Value: {:?}", unregistration_value);
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
    tracing::debug!("Try to get wallet inputs...");
    let inputs = murin::input_selection(None, &mut needed, &utxos, Some(collateral.get(0)), None)?;
    tracing::debug!("Try to get wallet inputs...");
    let req_signer_inputs = cardano::find_utxos_by_address(first_address.clone(), &utxos).0;

    if req_signer_inputs.is_empty() {
        return Err(Error::Custom(format!(
            "cannot add UTxOs of the required signer, please populate address: {:?} with UTxOs",
            first_address.clone()
        )));
    }

    let mut txis = inputs.0.clone();
    txis.add(&script_utxo.input());

    let signer_check = !inputs.1.contains_address(first_address);
    if signer_check {
        txis.add(&req_signer_inputs.get(0).input());
    }

    let index = murin::get_input_position(txis, script_utxo.clone());
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

    let redeemer_data = PlutusData::new_constr_plutus_data(&ConstrPlutusData::new(
        &RegistrationRedeemer::Unregister.redeemer(),
        &PlutusList::new(),
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

    let protocol_parameters = ProtocolParameters::read_protocol_parameter(
        &std::env::var("PPPATH").unwrap_or_else(|_| "protocol_parameters_preview.json".to_owned()),
    )
    .unwrap();
    // CostModel
    let cost_models = protocol_parameters.get_CostMdls().unwrap();
    let costmodel = cost_models.get(&Language::new_plutus_v2()).unwrap();
    let mut pcm = CostModel::new();
    for i in 0..costmodel.len() {
        pcm.set(i, &pcm.get(i)?)?;
    }
    let mut cstmodls = Costmdls::new();
    cstmodls.insert(&Language::new_plutus_v2(), &pcm);

    let costmodel = cost_models.get(&Language::new_plutus_v2()).unwrap();
    let mut cstmodls_ = Costmdls::new();
    cstmodls_.insert(&Language::new_plutus_v2(), &costmodel);

    let mut redeemers = Redeemers::new();
    redeemers.add(&redeemer);
    tracing::debug!("\nCostModels:\n{:?}\n\n", cstmodls_);

    let scriptdatahash = utils::hash_script_data(&redeemers, &cstmodls_, None);
    tracing::debug!(
        "ScriptDataHash: {:?}\n",
        hex::encode(scriptdatahash.to_bytes())
    );
    tracing::debug!(
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
