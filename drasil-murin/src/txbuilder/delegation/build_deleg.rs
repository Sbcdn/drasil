use crate::error::MurinError;
use crate::supporting_functions::{balance_tx, get_ttl_tx, get_vkey_count, sum_output_values};
use crate::models::*;
use crate::txbuilder::{delegation::DelegTxData, input_selection, TxBO};
use crate::PerformTxb;
use crate::TxData;
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr, utils as cutils};

// Delegation Builder Type
#[derive(Debug, Clone)]
pub struct AtDelegBuilder {
    pub stxd: DelegTxData,
}

pub type AtDelegParams<'a> = &'a DelegTxData;

impl<'a> PerformTxb<AtDelegParams<'a>> for AtDelegBuilder {
    fn new(t: AtDelegParams) -> Self {
        AtDelegBuilder { stxd: t.clone() }
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

        let registered = self.stxd.get_registered();
        log::info!("\nThis user is registered: {}\n", registered);
        let owner_address = match gtxd.get_senders_address(None) {
            Some(a) => a,
            None => {
                return Err(MurinError::new(
                    "Address of Wallet owner could not be found",
                ))
            }
        };
        let delegators_address: caddr::Address = gtxd.get_stake_address();

        let delegators_address_bech32 = delegators_address.to_bech32(None)?;
        log::info!("Delegator Stake Address: {:?}", delegators_address_bech32);

        let owner_base_addr = caddr::BaseAddress::from_address(&owner_address).unwrap();
        let owner_stakecred = owner_base_addr.stake_cred();
        let deleg_rwd_addr = caddr::RewardAddress::from_address(&delegators_address).unwrap();
        let deleg_stake_creds = deleg_rwd_addr.payment_cred();
        if owner_stakecred.to_bytes() != deleg_stake_creds.to_bytes() {
            return Err(MurinError::new("Inconsistent Stake Key Data, forbidden!"));
        }

        let mut certs = clib::Certificates::new();

        if !registered {
            let stake_reg = clib::StakeRegistration::new(&deleg_stake_creds);
            let reg_cert = clib::Certificate::new_stake_registration(&stake_reg);
            certs.add(&reg_cert);
        }

        let stake_delegation =
            clib::StakeDelegation::new(&deleg_stake_creds, &self.stxd.get_poolkeyhash());
        let deleg_cert = clib::Certificate::new_stake_delegation(&stake_delegation);
        certs.add(&deleg_cert);

        let aux_data = None;
        //////////////////////////////////////////////////////////////////////////////////////////////////////
        //Add Inputs and Outputs
        //
        //
        ///////////////////////////////////////////////////////////////////////////////////////////////////////

        let mut txouts = clib::TransactionOutputs::new();
        // ATTENTION DIFFERENT VALUES FOR PREVIEW / PREPROD / MAINNET
        let deposit_val = cutils::Value::new(&cutils::to_bignum(2000000));

        // Inputs
        let mut input_txuos = gtxd.clone().get_inputs();

        // Check if some utxos in inputs are in use and remove them
        if let Some(used_utxos) = crate::utxomngr::usedutxos::check_any_utxo_used(&input_txuos)? {
            info!("\n\n");
            info!("USED UTXOS: {:?}", used_utxos);
            info!("\n\n");
            input_txuos.remove_used_utxos(used_utxos);
        }

        let k = crate::utxomngr::usedutxos::check_any_utxo_used(&input_txuos)?;
        info!("K: {:?}", k);

        // Balance TX
        let mut fee_paid = false;
        let mut first_run = true;
        let mut txos_paid = false;
        let mut tbb_values = cutils::Value::new(&cutils::to_bignum(0u64));
        if !registered {
            tbb_values = deposit_val.clone();
        }
        let mut acc = cutils::Value::new(&cutils::to_bignum(0u64));
        let change_address = owner_address.clone();

        let mut needed_value = sum_output_values(&txouts);
        needed_value.set_coin(&needed_value.coin().checked_add(&fee.clone()).unwrap());

        if !registered {
            needed_value = needed_value.checked_add(&deposit_val)?;
        }

        let security =
            cutils::to_bignum(cutils::from_bignum(&needed_value.coin()) / 100 * 10 + MIN_ADA); // 10% Security for min utxo etc.
        needed_value.set_coin(&needed_value.coin().checked_add(&security).unwrap());
        let mut needed_value = cutils::Value::new(&needed_value.coin());

        let (txins, mut input_txuos) =
            input_selection(None, &mut needed_value, &input_txuos, None, None)?;

        let saved_input_txuos = input_txuos.clone();
        let vkey_counter = get_vkey_count(&input_txuos, None) + 1; // +1 dues to signature in finalize

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
            &owner_address,
            &change_address,
            &mut acc,
            None,
            &fcrun,
        )?;

        let slot = gtxd.clone().get_current_slot() + get_ttl_tx(&gtxd.clone().get_network());
        let mut txbody = clib::TransactionBody::new_tx_body(&txins, &txouts_fin, fee);
        txbody.set_ttl(&cutils::to_bignum(slot));
        txbody.set_certs(&certs);

        // Set network Id
        //if gtxd.get_network() == clib::NetworkIdKind::Testnet {
        //    txbody.set_network_id(&clib::NetworkId::testnet());
        //} else {
        //    txbody.set_network_id(&clib::NetworkId::mainnet());
        //}

        let txwitness = clib::TransactionWitnessSet::new();

        debug!("TxWitness: {:?}", hex::encode(txwitness.to_bytes()));
        debug!("TxBody: {:?}", hex::encode(txbody.to_bytes()));
        debug!("--------------------Iteration Ended------------------------------");
        debug!("Vkey Counter at End: {:?}", vkey_counter);
        Ok((txbody, txwitness, aux_data, saved_input_txuos, vkey_counter))
    }
}

#[cfg(test)]
mod tests {
    use cardano_serialization_lib::crypto::Ed25519KeyHash;
    use clib::Certificate;
    use clib::Certificates;
    use clib::StakeDelegation;
    use clib::StakeRegistration;
    use clib::Transaction;
    use clib::TransactionInput;
    use clib::TransactionInputs;
    use clib::TransactionOutput;
    use clib::TransactionOutputs;
    use clib::TransactionWitnessSet;
    use clib::address::BaseAddress;
    use clib::address::StakeCredential;
    use clib::crypto::TransactionHash;
    use clib::utils::BigNum;
    use clib::utils::Value;
    use std::env::set_var;

    use crate::PerformTxb;
    use crate::TxData;
    use cardano_serialization_lib as clib;

    #[test]
    fn at_deleg_builder() {
        // initialize
        set_var("REDIS_DB", "redis://127.0.0.1:6379/0");
        set_var("REDIS_DB_URL_UTXOMIND", "redis://127.0.0.1:6379/0");
        set_var("REDIS_CLUSTER", "false");
        let poolhash = "pool1pt39c4va0aljcgn4jqru0jhtws9q5wj8u0xnajtkgk9g7lxlk2t";
        let base_address = "addr_test1qp6crwxyfwah6hy7v9yu5w6z2w4zcu53qxakk8ynld8fgcpxjae5d7xztgf0vyq7pgrrsk466xxk25cdggpq82zkpdcsdkpc68";
        let at_deleg_params = super::DelegTxData::new(poolhash).unwrap();
        let at_deleg_builder = super::AtDelegBuilder::new(&at_deleg_params);

        assert_eq!(at_deleg_builder.stxd.poolhash, poolhash);
        assert_eq!(at_deleg_builder.stxd.poolkeyhash, Ed25519KeyHash::from_bech32(poolhash).unwrap());
        assert_eq!(at_deleg_builder.stxd.registered, None);

        // perform_txb
        let perform_txb = at_deleg_builder.perform_txb(
            &clib::utils::BigNum::from_str("1").unwrap(), 
            &TxData::new(
                None, 
                vec![
                    super::caddr::Address::from_bech32(base_address).unwrap()
                ], 
                None, 
                crate::TransactionUnspentOutputs::new(), 
                clib::NetworkIdKind::Testnet, 
                10
            ).unwrap(), 
            &["".to_string()], 
            true
        ).unwrap();

        // check that the perform_txb output can be used for creating transaction
        let tx: Transaction = Transaction::new(
            &perform_txb.0, 
            &perform_txb.1, 
            perform_txb.2
        );

        // take function output as it is, check that it isn't unintentionally changed by future PR:s (doesn't check against real data)
        assert_eq!(
            tx.body().inputs(), 
            TransactionInputs::new()
        );
        assert_eq!(
            tx.body().outputs(), 
            TransactionOutputs::new()
        );
        assert_eq!(
            tx.body().fee(),
            BigNum::from_str("1").unwrap() // This value doesn't make sense to me (I think 2000000 lovelace makes more sense). Maybe something's wrong with perform_txb(), but this investigation is for a future task
        );
        assert_eq!(
            tx.body().ttl_bignum().unwrap(), 
            BigNum::from_str("1810").unwrap()
        );
        let mut certs = Certificates::new();
        certs.add(&Certificate::new_stake_registration(
            &StakeRegistration::new(
                &StakeCredential::from_keyhash(
                    &Ed25519KeyHash::from_bytes(
                        vec![38, 151, 115, 70, 248, 194, 90, 18, 246, 16, 30, 10, 6, 56, 90, 186, 209, 141, 101, 83, 13, 66, 2, 3, 168, 86, 11, 113]
                    ).unwrap()
                )
            )
        ));
        certs.add(&Certificate::new_stake_delegation(
            &StakeDelegation::new(
                &StakeCredential::from_keyhash(
                    &Ed25519KeyHash::from_bytes(
                        vec![38, 151, 115, 70, 248, 194, 90, 18, 246, 16, 30, 10, 6, 56, 90, 186, 209, 141, 101, 83, 13, 66, 2, 3, 168, 86, 11, 113]
                    ).unwrap()
                ),
                &Ed25519KeyHash::from_bech32(poolhash).unwrap()
            )
        ));
        assert_eq!(
            tx.body().certs(), 
            Some(
                certs
            )
        );
        assert_eq!(tx.body().withdrawals(), None);
        assert_eq!(tx.body().update(), None);
        assert_eq!(tx.body().auxiliary_data_hash(), None);
        assert_eq!(
            tx.body().validity_start_interval_bignum(), 
            None
        );
        assert_eq!(tx.body().mint(), None);
        assert_eq!(tx.body().script_data_hash(), None);
        assert_eq!(tx.body().collateral(), None);
        assert_eq!(tx.body().required_signers(), None);
        assert_eq!(tx.body().network_id(), None);
        assert_eq!(tx.body().collateral_return(), None);
        assert_eq!(tx.body().total_collateral(), None);
        assert_eq!(tx.body().reference_inputs(), None);
        assert_eq!(
            tx.witness_set(), 
            TransactionWitnessSet::new()
        );
        assert_eq!(tx.is_valid(), true);
        assert_eq!(tx.auxiliary_data(), None);
    }
}