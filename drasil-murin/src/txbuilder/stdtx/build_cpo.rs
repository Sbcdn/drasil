use crate::error::MurinError;
use crate::supporting_functions::{balance_tx, get_ttl_tx, get_vkey_count, sum_output_values};
use crate::models::*;
use crate::{
    txbuilder::{input_selection, TxBO},
    PerformTxb, TxData,
};
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr, utils as cutils};
use clib::address::{EnterpriseAddress, StakeCredential};
use clib::TransactionOutput;

// One Shot Minter Builder Type
#[derive(Debug, Clone)]
pub struct AtCPOBuilder<'a> {
    pub txo_values: Vec<(
        &'a caddr::Address,
        &'a cutils::Value,
        Option<&'a TransactionUnspentOutputs>,
    )>,
    pub script: clib::NativeScript,
}

pub type AtCPOParams<'a> = (
    Vec<(
        &'a caddr::Address,
        &'a cutils::Value,
        Option<&'a TransactionUnspentOutputs>,
    )>,
    clib::NativeScript,
);

impl<'a> PerformTxb<AtCPOParams<'a>> for AtCPOBuilder<'a> {
    fn new(t: AtCPOParams<'a>) -> Self {
        AtCPOBuilder::<'a> {
            txo_values: t.0,
            script: t.1.clone(),
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

        let cwallet_address = match gtxd.get_senders_address(None) {
            Some(a) => a,
            None => {
                return Err(MurinError::new(
                    "Address of Wallet owner could not be found",
                ))
            }
        };

        let script_hash = self.script.hash();
        let script_address = EnterpriseAddress::new(
            cwallet_address.network_id()?,
            &StakeCredential::from_scripthash(&script_hash),
        )
        .to_address();
        /////////////////////////////////////////////////////////////////////////////////////////////////////
        //
        //Auxiliary Data
        //  Plutus Script and Metadata
        /////////////////////////////////////////////////////////////////////////////////////////////////////
        let aux_data = None;

        //////////////////////////////////////////////////////////////////////////////////////////////////////
        //Add Inputs and Outputs
        //
        //
        ///////////////////////////////////////////////////////////////////////////////////////////////////////

        let mut txouts = clib::TransactionOutputs::new();
        for txo in &self.txo_values {
            txouts.add(&TransactionOutput::new(txo.0, txo.1))
        }

        // Inputs
        let mut input_txuos = gtxd.clone().get_inputs();

        // Check if some utxos in inputs are in use and remove them
        if let Some(used_utxos) = crate::utxomngr::usedutxos::check_any_utxo_used(&input_txuos)? {
            info!("\n\n");
            info!("USED UTXOS: {:?}", used_utxos);
            info!("\n\n");
            input_txuos.remove_used_utxos(used_utxos);
        }

        let mut fee_paid = false;
        let mut first_run = true;
        let mut txos_paid = false;
        let mut tbb_values = cutils::Value::new(&cutils::to_bignum(0u64));
        let mut acc = cutils::Value::new(&cutils::to_bignum(0u64));

        let mut needed_value = sum_output_values(&txouts);
        needed_value.set_coin(&needed_value.coin().checked_add(&fee.clone()).unwrap());
        let security =
            cutils::to_bignum(cutils::from_bignum(&needed_value.coin()) / 100 * 10 + MIN_ADA); // 10% Security for min utxo etc.
        needed_value.set_coin(&needed_value.coin().checked_add(&security).unwrap());

        debug!("Needed Value: {:?}", needed_value);

        let (txins, mut input_txuos) =
            input_selection(None, &mut needed_value, &input_txuos, None, None)?;

        let saved_input_txuos = input_txuos.clone();

        let mut vkey_counter = get_vkey_count(&input_txuos, None) + 1; // +1 dues to signature in finalize

        let txouts_fin = balance_tx(
            &mut input_txuos,
            &Tokens::new(),
            &mut txouts,
            None,
            fee,
            &mut fee_paid,
            &mut first_run,
            &mut txos_paid,
            &mut tbb_values,
            &script_address,
            &script_address,
            &mut acc,
            None,
            &fcrun,
        )?;

        let slot = gtxd.clone().get_current_slot() + get_ttl_tx(&gtxd.clone().get_network());
        let mut txbody = clib::TransactionBody::new_tx_body(&txins, &txouts_fin, fee);
        txbody.set_ttl(&cutils::to_bignum(slot));

        let mut txwitness = clib::TransactionWitnessSet::new();
        let mut native_scripts = clib::NativeScripts::new();
        native_scripts.add(&self.script);
        txwitness.set_native_scripts(&native_scripts);

        debug!("TxWitness: {:?}", hex::encode(txwitness.to_bytes()));
        debug!("TxBody: {:?}", hex::encode(txbody.to_bytes()));
        debug!("--------------------Iteration Ended------------------------------");
        if vkey_counter < 2 {
            info!("Vkey Counter was smaller than 2 why?: {:?}", vkey_counter);
            info!("Inputs: {:?}", input_txuos);
            vkey_counter = 2;
        }
        debug!("Vkey Counter at End: {:?}", vkey_counter);
        Ok((txbody, txwitness, aux_data, saved_input_txuos, vkey_counter))
    }
}
