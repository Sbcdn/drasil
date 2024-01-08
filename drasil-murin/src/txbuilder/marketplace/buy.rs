use crate::cardano::models;
use crate::error::MurinError;
use crate::marketplace::*;
use crate::modules::txtools::utxo_handling::combine_wallet_outputs;
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr, utils as cutils};
use clib::plutus::{ExUnits, PlutusScript, PlutusScripts};
use models::Tokens;

#[derive(Debug, Clone)]
pub struct AtMPBuyBuilder {
    pub contract: PlutusScript,
    pub sc_address: caddr::Address,
    pub owner: caddr::Address,
    pub mptxd: MpTxData,
}

pub type AtMPBuyParam<'a> = (
    &'a PlutusScript,
    &'a caddr::Address,
    &'a caddr::Address,
    &'a MpTxData,
);

impl<'a> super::PerformTxb<AtMPBuyParam<'a>> for AtMPBuyBuilder {
    fn new(t: AtMPBuyParam) -> Self {
        AtMPBuyBuilder {
            contract: t.0.clone(),
            sc_address: t.1.clone(),
            owner: t.2.clone(),
            mptxd: t.3.clone(),
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
            info!("--------------------------------------------------------------------------------------------------------");
            info!("-----------------------------------------Fee Calculation------------------------------------------------");
            info!("---------------------------------------------------------------------------------------------------------\n");
        } else {
            info!("--------------------------------------------------------------------------------------------------------");
            info!("-----------------------------------------Build Transaction----------------------------------------------");
            info!("--------------------------------------------------------------------------------------------------------\n");
        }

        // Is copy of Cancel change to Buy functionallity.
        todo!();

        /////////////////////////////////////////////////////////////////////////////////////////////////////
        //Restore Datum
        /////////////////////////////////////////////////////////////////////////////////////////////////////
        let contract_utxos = self.mptxd.token_utxos.clone();
        let mut datums = Vec::<MarketPlaceDatum>::new();
        for utxo in contract_utxos.0.iter() {
            if let Some(data) = utxo.output().plutus_data() {
                datums.push(decode_mp_datum(&data.to_bytes())?);
            }
        }

        if datums.len() != 1 {
            return Err("More than one datum found in contract utxos, currently only one trade at a time is supported".into());
        }

        //////////////////////////////////////////////////////////////////////////////////////////////////////
        //Add Inputs and Outputs
        ///////////////////////////////////////////////////////////////////////////////////////////////////////

        let mut txouts = clib::TransactionOutputs::new();
        let script_utxos = self.mptxd.token_utxos.clone();
        for utxo in script_utxos.0.iter() {
            let trade_utxo_value = utxo.output().amount();
            txouts.add(&min_ada_for_utxo(&TransactionOutput::new(
                &self.owner,
                &trade_utxo_value,
            ))?);
        }

        let mut input_txuos = gtxd.clone().get_inputs();

        // Check if some utxos in inputs are in use and remove them
        if let Some(used_utxos) = crate::utxomngr::usedutxos::check_any_utxo_used(&input_txuos)? {
            input_txuos.remove_used_utxos(used_utxos);
        }

        let collateral_input_txuo = gtxd.get_collateral();
        info!("\nCollateral Input: {:?}", collateral_input_txuo);

        // Balance TX
        debug!("Before Balance: Transaction Inputs: {:?}", input_txuos);
        debug!("Before Balance: Transaction Outputs: {:?}", txouts);

        let mut fee_paid = false;
        let mut first_run = true;
        let mut txos_paid = false;
        let mut tbb_values = cutils::Value::new(&cutils::to_bignum(0u64));
        let mut acc = cutils::Value::new(&cutils::to_bignum(0u64));

        let mut needed_value = supporting_functions::sum_output_values(&txouts);
        needed_value.set_coin(&needed_value.coin().checked_add(&fee.clone()).unwrap());
        let security = cutils::to_bignum(
            cutils::from_bignum(&needed_value.coin()) / 100 * 10 + (2 * models::MIN_ADA),
        ); // 10% Security for min utxo etc.
        needed_value.set_coin(&needed_value.coin().checked_add(&security).unwrap());

        let (signers_address_utxos, _) =
            supporting_functions::find_utxos_by_address(self.owner.clone(), &input_txuos);

        if signers_address_utxos.is_empty() {
            return Err(format!("The Pubkey which signed the trade has no utxos, please send some Ada to it. Address:{:?}",self.owner.to_bech32(None)).into());
        }

        let (mut txins, mut input_txuos) =
            input_selection(None, &mut needed_value, &input_txuos, None, None)?;
        let saved_input_txuos = input_txuos.clone();

        if !input_txuos.contains_any(&signers_address_utxos) {
            txins.add(&signers_address_utxos.0[0].input());
            input_txuos.add(&signers_address_utxos.0[0])
        }

        for utxo in script_utxos.0.iter() {
            txins.add(&utxo.input());
            input_txuos.add(&utxo);
        }

        let vkey_counter =
            supporting_functions::get_vkey_count(&input_txuos, collateral_input_txuo.as_ref());

        let txouts_fin = supporting_functions::balance_tx(
            &mut input_txuos,
            &Tokens::new(),
            &mut txouts,
            None,
            fee,
            &mut fee_paid,
            &mut first_run,
            &mut txos_paid,
            &mut tbb_values,
            &self.owner,
            &self.owner,
            &mut acc,
            None,
            &fcrun,
        )?;
        let txouts_fin = combine_wallet_outputs(&txouts_fin);

        let slot = gtxd.get_current_slot() + 3000;
        let mut txbody = clib::TransactionBody::new_tx_body(&txins, &txouts_fin, fee); //922321
        txbody.set_ttl(&cutils::to_bignum(slot));
        trace!("\nTxOutputs: {:?}\n", txbody.outputs());
        trace!("\nTxInouts: {:?}\n", txbody.inputs());

        //////////////////////////////////////////////////////////////////////////////////////////////////////
        //Required Signer
        //
        ///////////////////////////////////////////////////////////////////////////////////////////////////////
        let mut req_signers = clib::Ed25519KeyHashes::new();
        let owner_keyhash = caddr::BaseAddress::from_address(&self.owner)
            .unwrap()
            .payment_cred()
            .to_keyhash()
            .unwrap();
        req_signers.add(&owner_keyhash);

        txbody.set_required_signers(&req_signers);

        // Collateral Input
        let mut col_inputs = clib::TransactionInputs::new();
        if let Some(collateral) = collateral_input_txuo {
            col_inputs.add(&collateral.input());
            txbody.set_collateral(&col_inputs);
        };
        if txbody.collateral().is_none() {
            return Err(MurinError::new("Error: No collateral provided"));
        }
        //////////////////////////////////////////////////////////////////////////////////////////////////////
        //Redeemer
        //  Build Redeemer
        //
        ///////////////////////////////////////////////////////////////////////////////////////////////////////
        let mut redeemers = plutus::Redeemers::new();

        let protocol_parameters: ProtocolParameters = ProtocolParameters::read_protocol_parameter(
            &std::env::var("CARDANO_PROTOCOL_PARAMETER_PATH")
                .unwrap_or_else(|_| "/odin/protocol_parameters_babbage.json".to_owned()),
        )
        .unwrap();
        let exunits = ExUnits::new(
            &to_bignum(protocol_parameters.max_tx_execution_units.memory as u64 / 4),
            &to_bignum(protocol_parameters.max_tx_execution_units.steps as u64 / 4),
        );

        for utxo in script_utxos.0.iter() {
            let script_input_index = get_input_position(txins.clone(), utxo.clone());

            let redeemer_data =
                plutus::PlutusData::new_constr_plutus_data(&plutus::ConstrPlutusData::new(
                    &cutils::to_bignum(1u64),
                    &plutus::PlutusList::new(),
                ));

            let red = plutus::Redeemer::new(
                &plutus::RedeemerTag::new_spend(),
                &cutils::to_bignum(script_input_index.0 as u64),
                &redeemer_data,
                &exunits,
            );

            redeemers.add(&red);
            debug!("Redeemer: {:?} \n", red);
        }

        debug!("Redeemers: {:?} \n", hex::encode(redeemers.to_bytes()));

        // CostModel
        let cost_models = protocol_parameters.get_CostMdls().unwrap();
        let costmodel = cost_models
            .get(&crate::pparams::mp_plutus::Language::new_plutus_v2())
            .unwrap();
        let mut pcm = plutus::CostModel::new();
        for (i, o) in costmodel.op_costs.iter().enumerate() {
            pcm.set(i, o)?;
        }
        let mut cstmodls = plutus::Costmdls::new();
        cstmodls.insert(&plutus::Language::new_plutus_v2(), &pcm);

        let costmodel = cost_models
            .get(&crate::pparams::mp_plutus::Language::new_plutus_v2())
            .unwrap();
        let mut cstmodls_ = crate::pparams::mp_plutus::Costmdls::new();
        cstmodls_.insert(&costmodel);

        let scriptdatahash = crate::pparams::hash::hash_script_data(&redeemers, &cstmodls_, None);
        log::debug!(
            "ScriptDataHash: {:?}\n",
            hex::encode(scriptdatahash.to_bytes())
        );
        txbody.set_script_data_hash(&scriptdatahash);

        let mut txwitness = clib::TransactionWitnessSet::new();
        let mut scripts = PlutusScripts::new();
        scripts.add(&self.contract);
        txwitness.set_plutus_scripts(&scripts);
        txwitness.set_redeemers(&redeemers);

        info!("--------------------Iteration Ended------------------------------");
        info!("Vkey Counter at End: {:?}", vkey_counter);
        Ok((
            txbody,
            txwitness,
            None,
            saved_input_txuos,
            vkey_counter,
            true,
        ))
    }
}
