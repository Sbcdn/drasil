use crate::error::MurinError;
use crate::hfn::{balance_tx, get_ttl_tx, get_vkey_count, sum_output_values};
use crate::htypes::*;
use crate::txbuilders::{delegation::DelegTxData, input_selection, TxBO};
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
            info!("-----------------------------------------Fee calcualtion------------------------------------------------");
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
            return Err(MurinError::new("Inconsitent Stake Key Data, forbidden!"));
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

        let aux_data = clib::metadata::AuxiliaryData::new();
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
        let mut fee_paied = false;
        let mut first_run = true;
        let mut txos_paied = false;
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
            &mut fee_paied,
            &mut first_run,
            &mut txos_paied,
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
