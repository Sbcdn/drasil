    
/*
use super::RegistrationDatum;
use cardano_serialization_lib::{
    address::{Address, EnterpriseAddress, StakeCredential},
    crypto::ScriptHash,
    fees::LinearFee,
    plutus::{self, ExUnitPrices, PlutusData, PlutusScript},
    tx_builder::{
        tx_inputs_builder::TxInputsBuilder, CoinSelectionStrategyCIP2, TransactionBuilder,
        TransactionBuilderConfigBuilder,
    },
    utils::{hash_plutus_data, min_ada_for_output, to_bignum, Value},
    AssetName, Assets, MultiAsset, TransactionOutput, UnitInterval,
};
use cdp::provider::CardanoDataProvider;
use log::debug;
use serde::{Deserialize, Serialize};
use sha2::Digest;
//! UnRegister EarthNode Transaction
//!
//! This module implement the unregistration transaction for WorldMobile's AyA Network on Cardano.
//!

use cardano_serialization_lib as clib;
use cardano_serialization_lib::utils as cutils;
use clib::address::{BaseAddress, EnterpriseAddress, StakeCredential};
use clib::plutus::{ConstrPlutusData, PlutusData, PlutusList, PlutusScripts, self, Redeemer, RedeemerTag, Redeemers, Language, ExUnits};
use clib::utils::{to_bignum, hash_script_data};
use clib::{AssetName, Assets, MultiAsset, TransactionOutput, TransactionOutputs, TransactionInputs};

use crate::cardano::{self, supporting_functions, Tokens};
use crate::error::MurinError;
use crate::modules::txtools::utxo_handling::combine_wallet_outputs;
use crate::pparams::ProtocolParameters;
use crate::txbuilder::{input_selection, TxBO};
use crate::TxData;
use crate::{min_ada_for_utxo, PerformTxb};

/// This type is a staking transaction builder for WMT.
#[derive(Debug, Clone)]
pub struct AtEnUnRegBuilder {
    /// Staking data.
    pub stxd: EnRegTxData,
    /// Configuration data.
    pub config: EnRegConfig,
}

/// This type represents a staking transaction parameters.
pub type AtEnUnRegParams<'param> = &'param EnRegTxData;

impl<'param> PerformTxb<AtEnUnRegParams<'param>> for AtEnUnRegBuilder {
    /// Creates new staking builder.
    fn new(params: AtEnUnRegParams) -> Self {
        let config = EnRegConfig::load();
        Self {
            stxd: params.clone(),
            config,
        }
    }

    fn perform_txb(
        &self,
        fee: &clib::utils::BigNum,
        gtxd: &TxData,
        _pvks: &[String],
        fcrun: bool,
    ) -> std::result::Result<TxBO, MurinError> {
        if fcrun {
            debug!("--------------------------------------------------------------------------------------------------------");
            debug!("-----------------------------------------Fee Calculation------------------------------------------------");
            debug!("---------------------------------------------------------------------------------------------------------\n");
        } else {
            debug!("--------------------------------------------------------------------------------------------------------");
            debug!("-----------------------------------------Build Transaction----------------------------------------------");
            debug!("--------------------------------------------------------------------------------------------------------\n");
        }

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

    let mut datums = plutus::PlutusList::new();
    datums.add(&datum);
    let datumhash = hash_plutus_data(&datum);
    log::info!("DatumHash: {:?}\n", hex::encode(datumhash.to_bytes()));

    // Create unregistration output containing the ENNFT from the smartcontract,
    // sending the ENNFT fback to the owner
    let mut unregistration_value = script_utxo.output().amount();

    unregistration_value.set_coin(&calc_min_ada_for_utxo(&unregistration_value, None)?);
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
    log::debug!("Try to get wallet inputs...");
    let inputs = dcslc::input_selection(None, &mut needed, &utxos, Some(collateral.get(0)), None)?;
    log::debug!("Try to get wallet inputs...");
    let req_signer_inputs = dcslc::find_utxos_by_address(first_address.clone(), &utxos).0;

    if req_signer_inputs.is_empty() {
        return Err(TransactionBuildingError::Custom(format!(
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

    let index = dcslc::get_input_position(txis, script_utxo.clone());
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
        &RegistrationRedeemer::Unregister.redeemer(),
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

    let protocol_parameters =
        crate::modules::txprocessor::protocolparams::ProtocolParameters::read_protocol_parameter(
            &std::env::var("PPPATH")
                .unwrap_or_else(|_| "protocol_parameters_preview.json".to_owned()),
        )
        .unwrap();
    // CostModel
    let cost_models = protocol_parameters.get_CostMdls().unwrap();
    let costmodel = cost_models
        .get(&crate::modules::txprocessor::protocolparams::self_plutus::Language::new_plutus_v2())
        .unwrap();
    let mut pcm = plutus::CostModel::new();
    for (i, o) in costmodel.op_costs.iter().enumerate() {
        pcm.set(i, o)?;
    }
    let mut cstmodls = plutus::Costmdls::new();
    cstmodls.insert(&plutus::Language::new_plutus_v2(), &pcm);

    let costmodel = cost_models
        .get(&crate::modules::txprocessor::protocolparams::self_plutus::Language::new_plutus_v2())
        .unwrap();
    let mut cstmodls_ = crate::modules::txprocessor::protocolparams::self_plutus::Costmdls::new();
    cstmodls_.insert(&costmodel);

    let mut redeemers = plutus::Redeemers::new();
    redeemers.add(&redeemer);
    log::debug!("\nCostModels:\n{:?}\n\n", cstmodls_);

    let scriptdatahash = crate::modules::txprocessor::protocolparams::hash::hash_script_data(
        &redeemers, &cstmodls_, None,
    );
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
}

 */