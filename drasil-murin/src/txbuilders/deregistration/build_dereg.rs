use crate::error::MurinError;
use crate::hfn::{balance_tx, get_ttl_tx, get_vkey_count, sum_output_values};
use crate::htypes::*;
use crate::txbuilders::{deregistration::DeregTxData, input_selection, TxBO};
use crate::PerformTxb;
use crate::TxData;
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr, utils as cutils};

// Deregistration Builder Type
#[derive(Debug, Clone)] 
pub struct AtDeregBuilder {
    pub stxd: DeregTxData,
}
  
pub type AtDeregParams<'a> = &'a DeregTxData;
  
impl<'a> PerformTxb<AtDeregParams<'a>> for AtDeregBuilder {
    fn new(t: AtDeregParams) -> Self {
        AtDeregBuilder { stxd: t.clone() }
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

        let owner_base_addr = if let Some(address) = caddr::BaseAddress::from_address(&owner_address) {
            address
        } else {
            return Err(MurinError::new(
                "The given stake owner address isn't a base address. See https://docs.cardano.org/learn/cardano-addresses/"
            ))
        };

        let owner_stakecred = owner_base_addr.stake_cred();
        let dereg_rwd_addr = caddr::RewardAddress::from_address(&delegators_address).unwrap();
        let dereg_stake_creds = dereg_rwd_addr.payment_cred();
        if owner_stakecred.to_bytes() != dereg_stake_creds.to_bytes() {
            return Err(MurinError::new("Inconsistent Stake Key Data, forbidden!"));
        }
  
        let mut certs = clib::Certificates::new();
  
        if registered {
            let stake_dereg = clib::StakeDeregistration::new(&dereg_stake_creds);
            let dereg_cert = clib::Certificate::new_stake_deregistration(&stake_dereg);
            certs.add(&dereg_cert);
        }
  
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
        if registered {
            tbb_values = deposit_val.clone();
        }
        let mut acc = cutils::Value::new(&cutils::to_bignum(0u64));
        let change_address = owner_address.clone();
  
        let mut needed_value = sum_output_values(&txouts);
        needed_value.set_coin(&needed_value.coin().checked_add(&fee.clone()).unwrap());
  
        if registered {
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
        //    txbody.set_network_id(&clib::NetworkId::testnet());
        //} else {
        //    txbody.set_network_id(&clib::NetworkId::mainnet());
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
    use clib::TransactionBody;
    use clib::TransactionInputs;
    use clib::TransactionOutputs;
    use clib::TransactionWitnessSet;

    use crate::MurinError;
    use crate::PerformTxb;
    use crate::TxData;
    use cardano_serialization_lib as clib;

    #[test]
    fn at_dereg_builder() -> Result<(), MurinError>{
        // initialize
        let poolhash = "pool1pt39c4va0aljcgn4jqru0jhtws9q5wj8u0xnajtkgk9g7lxlk2t";
        // let stake_address = "stake_test1uqnfwu6xlrp95yhkzq0q5p3ct2adrrt92vx5yqsr4ptqkugn5s708";
        let base_address = "addr_test1qp6crwxyfwah6hy7v9yu5w6z2w4zcu53qxakk8ynld8fgcpxjae5d7xztgf0vyq7pgrrsk466xxk25cdggpq82zkpdcsdkpc68";
        let at_dereg_params = super::DeregTxData::new(poolhash)?;
        let at_dereg_builder = super::AtDeregBuilder::new(&at_dereg_params);

        assert_eq!(at_dereg_builder.stxd.poolhash, poolhash);
        assert_eq!(at_dereg_builder.stxd.poolkeyhash, Ed25519KeyHash::from_bech32(poolhash)?);
        assert_eq!(at_dereg_builder.stxd.registered, None);

        // perform_txb
        let fee = clib::utils::BigNum::from_str("1")?;
        let contract_id = None;
        let saddress = super::caddr::Address::from_bech32(base_address)?;
        let saddresses = vec![saddress];
        let sstake = None;
        let inputs = crate::TransactionUnspentOutputs::new();
        let network = clib::NetworkIdKind::Testnet;
        let current_slot = 10;
        let gtxd = TxData::new(contract_id, saddresses, sstake, inputs, network, current_slot)?;
        let pvks = &["".to_string()];
        let fcrun = true;
        let perform_txb = at_dereg_builder.perform_txb(&fee, &gtxd, pvks, fcrun)?;

        let inputs = TransactionInputs::new();
        let outputs = TransactionOutputs::new();
        let txbody = TransactionBody::new_tx_body(&inputs, &outputs, &fee);
        let txwitness = TransactionWitnessSet::new();
        let aux_data = None;
        let vkey_counter = 10;
        assert_eq!(perform_txb.0, txbody);
        assert_eq!(perform_txb.1, txwitness);
        assert_eq!(perform_txb.2, aux_data);
        // assert_eq!(perform_txb.3, "PartialEq impl Missing");
        assert_eq!(perform_txb.4, vkey_counter);

        Ok(())
    }
}