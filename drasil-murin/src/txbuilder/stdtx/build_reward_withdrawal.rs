use crate::cardano::models::*;
use crate::cardano::supporting_functions::{
    balance_tx, get_ttl_tx, get_vkey_count, sum_output_values,
};
use crate::error::MurinError;
use crate::txbuilder::{input_selection, stdtx::WithdrawalTxData, TxBO};
use crate::PerformTxb;
use crate::TxData;
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr, utils as cutils};
use clib::address::{RewardAddress, StakeCredential};
use clib::metadata::AuxiliaryData;
use clib::utils::BigNum;
use clib::{TransactionInputs, TransactionOutputs, TransactionWitnessSet, Withdrawals, Certificates, Certificate, StakeRegistration};

// Withdrawal Builder Type
#[derive(Debug, Clone)]
pub struct AtAWBuilder {
    pub stxd: WithdrawalTxData,
}

pub type AtAWParams<'a> = &'a WithdrawalTxData;

impl<'a> PerformTxb<AtAWParams<'a>> for AtAWBuilder {
    fn new(t: AtAWParams) -> Self {
        AtAWBuilder { stxd: t.clone() }
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

        let txouts = clib::TransactionOutputs::new();
        let mut needed_value = sum_output_values(&txouts);
        let mut input_txuos = gtxd.clone().get_inputs();

        // Add a transaction fee and additional margin to the minimum UTxOs required for transaction
        needed_value.set_coin(&needed_value.coin().checked_add(&fee.clone()).unwrap());
        let security =
            cutils::to_bignum(cutils::from_bignum(&needed_value.coin()) / 100 * 10 + MIN_ADA); // 10% Security for min utxo etc.
        needed_value.set_coin(&needed_value.coin().checked_add(&security).unwrap());
        let mut needed_value = cutils::Value::new(&needed_value.coin());

        // Filter away input utxos that are already in use
        if let Some(used_utxos) = crate::utxomngr::usedutxos::check_any_utxo_used(&input_txuos)? {
            info!("\n\n");
            info!("USED UTXOS: {:?}", used_utxos);
            info!("\n\n");
            input_txuos.remove_used_utxos(used_utxos);
        }

        // Choose inputs for transaction body
        let (txins, _input_txuos) =
        input_selection(None, &mut needed_value, &input_txuos, None, None)?;
        let saved_input_txuos = input_txuos.clone();

        // Chooose outputs for transaction body
        let txouts_fin = balance_tx(
            &mut input_txuos,
            &Tokens::new(),
            &mut TransactionOutputs::new(),
            None,
            fee,
            &mut false,
            &mut true,
            &mut false,
            &mut cutils::Value::new(&cutils::to_bignum(0u64)),
            &gtxd.get_stake_address(),
            &gtxd.get_stake_address(),
            &mut cutils::Value::new(&cutils::to_bignum(0u64)),
            None,
            &fcrun,
        )?;

        let mut txbody = clib::TransactionBody::new_tx_body(&txins, &txouts_fin, fee);

        let mut withdrawals = Withdrawals::new();
        withdrawals.insert(
            &RewardAddress::from_address(&gtxd.get_stake_address()).unwrap(),
            &gtxd.get_withdrawal().unwrap()
        );
        txbody.set_withdrawals(&withdrawals);
        txbody.set_ttl(&cutils::to_bignum(
            gtxd.clone().get_current_slot() + get_ttl_tx(&gtxd.clone().get_network())
        ));

        // Stake address must be registered
        let mut certs = Certificates::new();
        certs.add(
            &Certificate::new_stake_registration(
                &StakeRegistration::new(
                    &caddr::RewardAddress::from_address(
                        &gtxd.get_stake_address()
                    )
                    .unwrap()
                    .payment_cred()
                )
            )
        );

        txbody.set_certs(&certs);

        let txwitness = TransactionWitnessSet::new();
        let aux_data = Some(AuxiliaryData::new());
        let vkey_counter = 0;

        Ok((txbody, txwitness, aux_data, saved_input_txuos, vkey_counter))
    }
}
