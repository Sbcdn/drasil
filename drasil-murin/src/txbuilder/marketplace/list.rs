use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr, utils as cutils};
use clib::address::BaseAddress;
use clib::crypto::Ed25519KeyHash;
use clib::plutus::PlutusScript;
use clib::{Assets, MultiAsset, TransactionWitnessSet};
use models::Tokens;

use crate::cardano::{models, supporting_functions};
use crate::error::MurinError;
use crate::marketplace::*;
use crate::modules::txtools::utxo_handling::combine_wallet_outputs;

#[derive(Debug, Clone)]
pub struct AtMPListBuilder {
    pub contract: PlutusScript,
    pub sc_address: caddr::Address,
    pub owner: caddr::Address,
    pub mptxd: MpTxData,
}

pub type AtMPListParam<'a> = (
    &'a PlutusScript,
    &'a caddr::Address,
    &'a caddr::Address,
    &'a MpTxData,
);

impl<'a> super::PerformTxb<AtMPListParam<'a>> for AtMPListBuilder {
    fn new(t: AtMPListParam) -> Self {
        AtMPListBuilder {
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

        let roy_rate = if let Some(royrate) = self.mptxd.clone().get_royalties_rate() {
            (royrate * 1000.0) as u64
        } else {
            0u64
        };

        let unpaid_royalties: u64 = self.mptxd.clone().get_price() / 1000 * roy_rate;

        let royalties_address: Option<caddr::Address>;
        let royalties_pkh: Option<Ed25519KeyHash>;
        if let Some(royaddr) = self.mptxd.clone().get_royalties_address() {
            if roy_rate > 0u64 {
                royalties_address = Some(royaddr);
                royalties_pkh = Some(supporting_functions::get_payment_keyhash(
                    &royalties_address.as_ref().unwrap(),
                ));
            } else {
                royalties_pkh = None;
                royalties_address = None;
            }
        } else {
            royalties_pkh = None;
            royalties_address = None;
        };

        //////////////////////////////////////////////////////////////////////////////////////////////////////
        //Add Inputs and Outputs
        //
        //
        ///////////////////////////////////////////////////////////////////////////////////////////////////////
        let trade_owner = &gtxd.clone().get_senders_addresses()[0];
        let mut txouts = clib::TransactionOutputs::new();

        let tokens = self.mptxd.get_tokens().clone();

        let owner_keyhash = BaseAddress::from_address(&self.owner)
            .unwrap()
            .payment_cred()
            .to_keyhash()
            .unwrap();

        for token in tokens.iter() {
            let mp_datum = MarketPlaceDatum {
                price: self.mptxd.selling_price as u64,
                seller: owner_keyhash.clone(),
                royalties_rate: roy_rate as u64,
                royalties_pkh: royalties_pkh.clone(),
                policy_id: PolicyID::from_bytes(self.mptxd.get_tokens()[0].0.to_bytes())?,
                token_name: AssetName::new(self.mptxd.get_tokens()[0].1.to_bytes())?,
            };
            let datumpair = encode_mp_datum(mp_datum);

            let mut trade_utxo_value = clib::utils::Value::zero();
            let mut trade_utxo_ma = MultiAsset::new();
            let mut trade_assets = Assets::new();
            trade_assets.insert(&token.1, &token.2);
            trade_utxo_ma.insert(&token.0, &trade_assets);
            trade_utxo_value.set_multiasset(&trade_utxo_ma);
            let mut txo = TransactionOutput::new(&self.sc_address, &trade_utxo_value);
            txo.set_plutus_data(&datumpair.1);
            txouts.add(&min_ada_for_utxo(&txo)?);
        }

        if unpaid_royalties > 0 {
            let txo = TransactionOutput::new(
                &royalties_address.unwrap(),
                &clib::utils::Value::new(&to_bignum(unpaid_royalties)),
            );
            txouts.add(&min_ada_for_utxo(&txo)?)
        }

        let mut input_txuos = gtxd.get_inputs().clone();

        // Check if some utxos in inputs are in use and remove them
        if let Some(used_utxos) = crate::utxomngr::usedutxos::check_any_utxo_used(&input_txuos)? {
            input_txuos.remove_used_utxos(used_utxos);
        }

        // Balance TX

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

        debug!("Needed Value: {:?}", needed_value);

        let (txins, mut input_txuos) =
            input_selection(None, &mut needed_value, &input_txuos, None, None)?;
        let saved_input_txuos = input_txuos.clone();

        let vkey_counter = supporting_functions::get_vkey_count(&input_txuos, None);

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
            trade_owner,
            trade_owner,
            &mut acc,
            None,
            &fcrun,
        )?;
        let txouts_fin = combine_wallet_outputs(&txouts_fin);

        let slot = gtxd.clone().get_current_slot()
            + supporting_functions::get_ttl_tx(&gtxd.clone().get_network());
        let mut txbody = clib::TransactionBody::new_tx_body(&txins, &txouts_fin, fee);
        txbody.set_ttl(&cutils::to_bignum(slot));

        let txwitness = TransactionWitnessSet::new();

        debug!("TxWitness: {:?}", hex::encode(txwitness.to_bytes()));
        debug!("TxBody: {:?}", hex::encode(txbody.to_bytes()));
        debug!("--------------------Iteration Ended------------------------------");
        debug!("Vkey Counter at End: {:?}", vkey_counter);
        Ok((
            txbody,
            txwitness,
            None,
            saved_input_txuos,
            vkey_counter,
            false,
        ))
    }
}
