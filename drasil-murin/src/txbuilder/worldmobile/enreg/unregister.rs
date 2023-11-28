// UnRegister EarthNode Transaction
//
// This module implement the registration transaction for WorldMobile's AyA Network on Cardano.
//
use super::EnRegistrationTxData;
use crate::error::MurinError;
use crate::modules::txtools::utxo_handling::combine_wallet_outputs;
use crate::pparams::ProtocolParameters;
use crate::txbuilder::{input_selection, TxBO};
use crate::TxData;
use crate::{
    cardano::{self, supporting_functions, Tokens},
    usedutxos,
    worldmobile::configuration::EnRegistrationConfig,
};
use crate::{min_ada_for_utxo, PerformTxb};
use cardano_serialization_lib as clib;
use cardano_serialization_lib::utils as cutils;
use cardano_serialization_lib::{
    crypto::ScriptHash,
    plutus::{self, PlutusData},
    utils::{hash_plutus_data, to_bignum, Value},
    Assets, MultiAsset, TransactionOutput,
};
use clib::plutus::{ExUnits, Language, PlutusScripts, Redeemer, RedeemerTag, Redeemers};
use clib::{
    utils::{BigInt, Int},
    TransactionOutputs,
};
use log::debug;
use supporting_functions::{balance_tx, get_input_position, get_ttl_tx};

/// This type is a staking transaction builder for WMT.
#[derive(Debug, Clone)]
pub struct AtUnEnRegBuilder {
    /// Staking data.
    pub rtxd: EnRegistrationTxData,
    /// Configuration data.
    pub config: EnRegistrationConfig,
}

/// This type represents a staking transaction parameters.
pub type AtUnEnRegParams<'param> = &'param EnRegistrationTxData;

impl<'param> PerformTxb<AtUnEnRegParams<'param>> for AtUnEnRegBuilder {
    /// Creates new staking builder.
    fn new(params: AtUnEnRegParams) -> Self {
        let config = EnRegistrationConfig::load();
        Self {
            rtxd: params.clone(),
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

        let protocol_parameters: ProtocolParameters = ProtocolParameters::read_protocol_parameter(
            &std::env::var("CARDANO_PROTOCOL_PARAMETER_PATH")
                .unwrap_or_else(|_| "/odin/protocol_parameters_babbage.json".to_owned()),
        )
        .unwrap();

        // We had to fill in the first address of the sender wallet in preperation step (hugin-lib) already, we can safely unwrap here.
        let sender_address = &self.rtxd.first_addr_sender_wallet.as_ref().unwrap().clone();

        // PolicyId EnOpNft Minting Policy
        let enop_nft_minting_policy_id: ScriptHash = self.config.enop_nft_minting_policy.hash();

        // Collateral UTxO had to be filled in the preperation step (hugin-lib) already, we can safely unwrap here.
        let collateral_input_txuo = gtxd.clone().get_collateral();

        // Create Plutus Datum
        let mut inner = plutus::PlutusList::new();
        inner.add(&PlutusData::new_bytes(
            self.rtxd.registration_datum.en_operator_address.to_vec(),
        ));
        inner.add(&PlutusData::new_bytes(
            self.rtxd.registration_datum.en_consensus_pubkey.to_vec(),
        ));
        inner.add(&PlutusData::new_bytes(
            self.rtxd.registration_datum.en_merkle_tree_root.to_vec(),
        ));
        inner.add(&PlutusData::new_bytes(
            self.rtxd.registration_datum.en_cce_address.to_vec(),
        ));
        inner.add(&PlutusData::new_bytes(
            self.rtxd.registration_datum.en_used_nft_tn.name(),
        ));
        inner.add(&PlutusData::new_bytes(
            self.rtxd.registration_datum.en_rwd_wallet.to_bytes(),
        ));
        inner.add(&PlutusData::new_integer(&BigInt::from_str(
            &self.rtxd.registration_datum.en_commission.to_string(),
        )?));
        inner.add(&PlutusData::new_bytes(
            self.rtxd.registration_datum.en_op_nft_cs.to_bytes().clone(),
        ));
        inner.add(&PlutusData::new_bytes(
            self.rtxd.registration_datum.en_signature.clone(),
        ));

        let datum = &plutus::PlutusData::new_constr_plutus_data(&plutus::ConstrPlutusData::new(
            &to_bignum(0),
            &inner,
        ));
        log::info!("\nDatum: {:?}", datum);

        let mut datums = plutus::PlutusList::new();
        datums.add(datum);
        let datumhash = hash_plutus_data(datum);
        log::info!("DatumHash: {:?}\n", hex::encode(datumhash.to_bytes()));

        let ennft_utxo = self.rtxd.ennft_utxo.as_ref().unwrap().clone();
        let enopnft_utxo = self.rtxd.enopnft_utxo.as_ref().unwrap().clone();

        // The returning value is the same as the one on the registration validator + change, the ENOPNFT is going to be burned
        let return_value = ennft_utxo.output().amount();
        let mut rv = Value::zero();
        let mut assets = Assets::new();
        let mut ma = MultiAsset::new();
        assets.insert(&self.rtxd.registration_datum.en_used_nft_tn, &to_bignum(1));
        ma.insert(&enop_nft_minting_policy_id, &assets);
        rv.set_multiasset(&ma);

        let diff = enopnft_utxo.output().amount().checked_sub(&rv)?;
        let return_value = return_value.checked_add(&diff)?;
        let return_output =
            min_ada_for_utxo(&TransactionOutput::new(&sender_address, &return_value))?;

        let mut txouts = TransactionOutputs::new();
        txouts.add(&return_output);

        // Lookup Input UTxOs with at least the tokens for the registration value
        let mut needed = return_value
            // Add 2 Ada reserve
            .checked_add(&Value::new(&to_bignum(2000000)))?;

        let utxos = gtxd.inputs.clone();
        let (mut txis, mut utxos_txis) = input_selection(
            None,
            &mut needed,
            &utxos,
            collateral_input_txuo.clone(),
            None,
        )?;

        txis.add(&ennft_utxo.input());
        txis.add(&enopnft_utxo.input());
        utxos_txis.add(&ennft_utxo);
        utxos_txis.add(&enopnft_utxo);

        // The number of verification keys. This determine the number of signatures required.
        let saved_input_txuos = utxos_txis.clone();
        let vkey_counter = cardano::get_vkey_count(&utxos_txis, collateral_input_txuo.as_ref());

        // Balance TX
        let mut fee_paid = false;
        let mut first_run = true;
        let mut txos_paid = false;
        let mut tbb_values = cutils::Value::new(&cutils::to_bignum(0u64));
        let mut acc = tbb_values.clone();
        let mut input_utxos = utxos_txis.clone();

        let txouts_fin = balance_tx(
            &mut input_utxos,
            &Tokens::new(),
            &mut txouts,
            Some(return_value).as_ref(),
            fee,
            &mut fee_paid,
            &mut first_run,
            &mut txos_paid,
            &mut tbb_values,
            &sender_address,
            &sender_address,
            &mut acc,
            None,
            &fcrun,
        )?;
        let txouts_fin = combine_wallet_outputs(&txouts_fin);

        ////////////////////////////////////////////////////////////////////////////////////////////
        //
        // MINT ASSETS
        //
        ////////////////////////////////////////////////////////////////////////////////////////////
        let mut mintasset = clib::MintAssets::new();
        mintasset.insert(
            &self.rtxd.registration_datum.en_used_nft_tn,
            Int::new_i32(-1),
        );
        let mint = clib::Mint::new_from_entry(&enop_nft_minting_policy_id, &mintasset);
        let slot = cutils::to_bignum(
            gtxd.clone().get_current_slot() + get_ttl_tx(&gtxd.clone().get_network()),
        );

        // CostModel
        let cost_models = protocol_parameters.get_CostMdls().unwrap();
        let costmodel = cost_models.get(&Language::new_plutus_v2()).unwrap();
        let mut pcm = plutus::CostModel::new();
        let mut c = 0;
        for i in 0..costmodel.len() {
            if let Ok(n) = costmodel.get(i) {
                if n != cutils::Int::from_str("0")? {
                    pcm.set(c, &n)?;
                    c += 1;
                }
            }
        }
        let mut cstmodls = plutus::Costmdls::new();
        cstmodls.insert(&plutus::Language::new_plutus_v2(), &pcm);

        // Redeemer
        let redeemer_data = plutus::PlutusData::new_constr_plutus_data(
            &plutus::ConstrPlutusData::new(&to_bignum(1u64), &plutus::PlutusList::new()),
        );

        let exunits = ExUnits::new(
            &to_bignum(protocol_parameters.execution_unit_prices.priceMemory as u64),
            &to_bignum(protocol_parameters.execution_unit_prices.priceSteps as u64),
        );
        let redeemer_index = get_input_position(
            txis.clone(),
            self.rtxd.enopnft_utxo.as_ref().unwrap().clone(),
        );

        let burn_redeemer = Redeemer::new(
            &RedeemerTag::new_mint(),
            &to_bignum(redeemer_index.0 as u64),
            &redeemer_data,
            &exunits,
        );

        // Redeemer
        let val_redeemer_data = plutus::PlutusData::new_constr_plutus_data(
            &plutus::ConstrPlutusData::new(&to_bignum(0u64), &plutus::PlutusList::new()),
        );

        let val_redeemer_index =
            get_input_position(txis.clone(), self.rtxd.ennft_utxo.as_ref().unwrap().clone());

        let val_redeemer = Redeemer::new(
            &RedeemerTag::new_spend(),
            &to_bignum(val_redeemer_index.0 as u64),
            &val_redeemer_data,
            &exunits,
        );

        let mut redeemers = Redeemers::new();
        redeemers.add(&burn_redeemer);
        redeemers.add(&val_redeemer);
        log::debug!("\nCostModels:\n{:?}\n\n", cstmodls);

        let scriptdatahash = cutils::hash_script_data(&redeemers, &cstmodls, Some(datums.clone()));
        log::debug!(
            "ScriptDataHash: {:?}\n",
            hex::encode(scriptdatahash.to_bytes())
        );

        let mut txbody = clib::TransactionBody::new_tx_body(&txis, &txouts_fin, fee);
        txbody.set_ttl(&slot);
        txbody.set_mint(&mint);
        txbody.set_script_data_hash(&scriptdatahash);

        // Create the transaction witnesses.
        let mut txwitness = clib::TransactionWitnessSet::new();
        txwitness.set_redeemers(&redeemers);
        txwitness.set_plutus_data(&datums);
        let mut plutus_scripts = PlutusScripts::new();
        plutus_scripts.add(&self.config.enop_nft_minting_policy);
        plutus_scripts.add(&self.config.registration_validator_smart_contract);
        txwitness.set_plutus_scripts(&plutus_scripts);

        Ok((txbody, txwitness, None, saved_input_txuos, vkey_counter))
    }
}
/*


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
