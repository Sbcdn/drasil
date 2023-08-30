use crate::error::MurinError;
use crate::supporting_functions::{balance_tx, get_ttl_tx, get_vkey_count, sum_output_values};
use crate::models::*;
use crate::txbuilder::{input_selection, stdtx::DeregTxData, TxBO};
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
  
        let stake_dereg = clib::StakeDeregistration::new(&dereg_stake_creds);
        let dereg_cert = clib::Certificate::new_stake_deregistration(&stake_dereg);
        certs.add(&dereg_cert);
  
        let aux_data = None;
        //////////////////////////////////////////////////////////////////////////////////////////////////////
        //Add Inputs and Outputs
        //
        //
        ///////////////////////////////////////////////////////////////////////////////////////////////////////
  
        let mut txouts = clib::TransactionOutputs::new();
        // ATTENTION DIFFERENT VALUES FOR PREVIEW / PREPROD / MAINNET
        let deposit_val = cutils::Value::new(&cutils::to_bignum(2000000));

        txouts.add(
            &clib::TransactionOutput::new(&owner_address, &deposit_val)
        );
  
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
        let tbb_values = deposit_val.clone();

        let mut acc = cutils::Value::zero();
        let change_address = owner_address.clone();
  
        let mut needed_value = sum_output_values(&txouts);
        needed_value.set_coin(&needed_value.coin().checked_add(&fee.clone()).unwrap());
  
        let security =
            cutils::to_bignum(cutils::from_bignum(&needed_value.coin()) / 100 * 10 + MIN_ADA); // 10% Security for min utxo etc.
        needed_value.set_coin(&needed_value.coin().checked_add(&security).unwrap());
  
        let (txins, mut input_txuos) =
            input_selection(None, &mut needed_value, &input_txuos, None, None)?;
  
        let saved_input_txuos = input_txuos.clone();
        let vkey_counter = get_vkey_count(&input_txuos, None) + 1; // +1 dues to signature in finalize
  
        let txouts_fin = balance_tx(
            &mut input_txuos,
            &Tokens::new(),
            &mut txouts,
            Some(&tbb_values),
            fee,
            &mut fee_paid,
            &mut first_run,
            &mut txos_paid,
            &mut clib::utils::Value::zero(),
            &owner_address,
            &change_address,
            &mut acc,
            None,
            &fcrun,
        )?;
  
        let deadline_slot = gtxd.clone().get_current_slot() + get_ttl_tx(&gtxd.get_network());
        let mut txbody = clib::TransactionBody::new_tx_body(&txins, &txouts_fin, fee);
        txbody.set_ttl(&cutils::to_bignum(deadline_slot));
        txbody.set_certs(&certs);
  
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
    use clib::StakeDeregistration;
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
    fn at_dereg_builder() {
        // initialize
        set_var("REDIS_DB", "redis://127.0.0.1:6379/0");
        set_var("REDIS_DB_URL_UTXOMIND", "redis://127.0.0.1:6379/0");
        set_var("REDIS_CLUSTER", "false");
        let poolhash = "pool1pt39c4va0aljcgn4jqru0jhtws9q5wj8u0xnajtkgk9g7lxlk2t";
        let base_address = "addr_test1qp6crwxyfwah6hy7v9yu5w6z2w4zcu53qxakk8ynld8fgcpxjae5d7xztgf0vyq7pgrrsk466xxk25cdggpq82zkpdcsdkpc68";
        let at_dereg_params = super::DeregTxData::new(poolhash).unwrap();
        let at_dereg_builder = super::AtDeregBuilder::new(&at_dereg_params);

        assert_eq!(at_dereg_builder.stxd.poolhash, poolhash);
        assert_eq!(at_dereg_builder.stxd.poolkeyhash, Ed25519KeyHash::from_bech32(poolhash).unwrap());
        assert_eq!(at_dereg_builder.stxd.registered, None);

        // perform_txb
        let pvks = &[]; // add real data
        let fcrun = true;
        let perform_txb = at_dereg_builder.perform_txb(
            &clib::utils::to_bignum(2_000_000), 
            &TxData::new(
                None, 
                vec![
                    super::caddr::Address::from_bech32(base_address).unwrap()
                ],
                None, 
                crate::TransactionUnspentOutputs::from_hex(
                    "9258668282582016ea4f7dc5f890cf9e701471680a7ee7cb5196124d59fa8122e57c92a6f8ac9600825839007581b8c44bbb7d5c9e6149ca3b4253aa2c729101bb6b1c93fb4e946026977346f8c25a12f6101e0a06385abad18d65530d420203a8560b711a3b64306c58668282582016ea4f7dc5f890cf9e701471680a7ee7cb5196124d59fa8122e57c92a6f8ac9601825839007581b8c44bbb7d5c9e6149ca3b4253aa2c729101bb6b1c93fb4e946026977346f8c25a12f6101e0a06385abad18d65530d420203a8560b711a1db37a7b586682825820d36cf84a8e743ec612d71fb6460c8b95e6946cf1bf81cf3e3464c557de76cb9d01825839007581b8c44bbb7d5c9e6149ca3b4253aa2c729101bb6b1c93fb4e946026977346f8c25a12f6101e0a06385abad18d65530d420203a8560b711a192f334e586682825820d36cf84a8e743ec612d71fb6460c8b95e6946cf1bf81cf3e3464c557de76cb9d03825839007581b8c44bbb7d5c9e6149ca3b4253aa2c729101bb6b1c93fb4e946026977346f8c25a12f6101e0a06385abad18d65530d420203a8560b711a004c4b4058668282582071eebb6da21245f18739ed643263ba044535cc0f2bdff03e0e5850a307f2f96a01825839007581b8c44bbb7d5c9e6149ca3b4253aa2c729101bb6b1c93fb4e946026977346f8c25a12f6101e0a06385abad18d65530d420203a8560b711a492c4ab75866828258206b1d5bc9ee3ddf63474471d0f736ee8cb93e82a0babaabfe6b7b410bcded2b5901825839007581b8c44bbb7d5c9e6149ca3b4253aa2c729101bb6b1c93fb4e946026977346f8c25a12f6101e0a06385abad18d65530d420203a8560b711a44b808c5589482825820ca0d2aefaac46ec34bf590bbff7e411fe6d00f06189b9c4065d0c94313dbb15301825839007581b8c44bbb7d5c9e6149ca3b4253aa2c729101bb6b1c93fb4e946026977346f8c25a12f6101e0a06385abad18d65530d420203a8560b71821a0011d28aa1581cf0ff48bbb7bbe9d59a40f1ce90e9e9d0ff5002ec48f232b49ca0fb9aa14b000de14074657374696e6701586682825820ca0d2aefaac46ec34bf590bbff7e411fe6d00f06189b9c4065d0c94313dbb15302825839007581b8c44bbb7d5c9e6149ca3b4253aa2c729101bb6b1c93fb4e946026977346f8c25a12f6101e0a06385abad18d65530d420203a8560b711a1a1b00f958668282582067d08be2e5a9d85c0dfe1b539439470a16e34c6e922c00236e5d76f51abc18d401825839007581b8c44bbb7d5c9e6149ca3b4253aa2c729101bb6b1c93fb4e946026977346f8c25a12f6101e0a06385abad18d65530d420203a8560b711a323d4b6a586682825820e8034ffa76067902bb4154f393319892d85ac7bdcf42fc78a97e9a4e3097d93a01825839007581b8c44bbb7d5c9e6149ca3b4253aa2c729101bb6b1c93fb4e946026977346f8c25a12f6101e0a06385abad18d65530d420203a8560b711a00490e44589b82825820745b443c1eb1eb64a7adcf9f966706622d33a408f26cef45eafe1c7ca5e1fea601825839007581b8c44bbb7d5c9e6149ca3b4253aa2c729101bb6b1c93fb4e946026977346f8c25a12f6101e0a06385abad18d65530d420203a8560b71821a00124864a1581c99b071ce8580d6a3a11b4902145adb8bfd0d2a03935af8cf66403e15a15053616c7361536f6c61726973436f696e19c350586682825820745b443c1eb1eb64a7adcf9f966706622d33a408f26cef45eafe1c7ca5e1fea602825839007581b8c44bbb7d5c9e6149ca3b4253aa2c729101bb6b1c93fb4e946026977346f8c25a12f6101e0a06385abad18d65530d420203a8560b711a1d9df38e58668282582043cb7614d1b20f4cd373f969e0879a4c26b1c9234fec46c67c962e6df1a9416801825839007581b8c44bbb7d5c9e6149ca3b4253aa2c729101bb6b1c93fb4e946026977346f8c25a12f6101e0a06385abad18d65530d420203a8560b711a637b15f15866828258207695f2942511d17e4bec7647fae0ce62d4e697951230a2338ffc696246e76ff601825839007581b8c44bbb7d5c9e6149ca3b4253aa2c729101bb6b1c93fb4e946026977346f8c25a12f6101e0a06385abad18d65530d420203a8560b711a16eb9e775897828258201f936ee7007b2a69189126ae2051c05fa8503ae27cc64a6d043940b34062286b02825839007581b8c44bbb7d5c9e6149ca3b4253aa2c729101bb6b1c93fb4e946026977346f8c25a12f6101e0a06385abad18d65530d420203a8560b71821a001e8480a1581c99b071ce8580d6a3a11b4902145adb8bfd0d2a03935af8cf66403e15a1465342455252591b0000000137d66d1658bc82825820ca2d0e7ad1ecc39d82c4812dde63b273c33b09e524452bac30a238a3bd19deb401825839007581b8c44bbb7d5c9e6149ca3b4253aa2c729101bb6b1c93fb4e946026977346f8c25a12f6101e0a06385abad18d65530d420203a8560b71821a001473faa2581c4086577ed57c514f8e29b78f42ef4f379363355a3b65b9a032ee30c9a1446c7020021a000f4240581c99b071ce8580d6a3a11b4902145adb8bfd0d2a03935af8cf66403e15a1465242455252591a004c4b40586682825820ca2d0e7ad1ecc39d82c4812dde63b273c33b09e524452bac30a238a3bd19deb402825839007581b8c44bbb7d5c9e6149ca3b4253aa2c729101bb6b1c93fb4e946026977346f8c25a12f6101e0a06385abad18d65530d420203a8560b711a0e6df48e586682825820e66c1e834139ae741e5968e23db9a733e43a23af856a8a506acabaedbe9ba41401825839007581b8c44bbb7d5c9e6149ca3b4253aa2c729101bb6b1c93fb4e946026977346f8c25a12f6101e0a06385abad18d65530d420203a8560b711a004929f0"
                ).unwrap(), 
                clib::NetworkIdKind::Testnet, 
                0,
            ).unwrap(), 
            pvks, 
            fcrun
        ).unwrap();

        // check that the perform_txb output can be used for creating transaction
        let tx: Transaction = Transaction::new(
            &perform_txb.0, 
            &perform_txb.1, 
            perform_txb.2
        );

        // take function output as it is, check that it isn't unintentionally changed by future PR:s (doesn't check against real data)
        let mut tx_inputs = TransactionInputs::new();
        tx_inputs.add(
            &TransactionInput::new(
                &TransactionHash::from_bytes(
                    vec![202, 45, 14, 122, 209, 236, 195, 157, 130, 196, 129, 45, 222, 99, 178, 115, 195, 59, 9, 229, 36, 69, 43, 172, 48, 162, 56, 163, 189, 25, 222, 180]
                ).unwrap(), 
                2
            )
        );
        assert_eq!(
            tx.body().inputs(), 
            tx_inputs
        );
        let mut tx_outputs = TransactionOutputs::new();
        tx_outputs.add(
            &TransactionOutput::new(
                &BaseAddress::new(
                    0, 
                    &StakeCredential::from_keyhash(
                        &Ed25519KeyHash::from_bytes(
                            vec![117, 129, 184, 196, 75, 187, 125, 92, 158, 97, 73, 202, 59, 66, 83, 170, 44, 114, 145, 1, 187, 107, 28, 147, 251, 78, 148, 96]
                        ).unwrap()
                    ), 
                    &StakeCredential::from_keyhash(
                        &Ed25519KeyHash::from_bytes(
                            vec![38, 151, 115, 70, 248, 194, 90, 18, 246, 16, 30, 10, 6, 56, 90, 186, 209, 141, 101, 83, 13, 66, 2, 3, 168, 86, 11, 113]
                        ).unwrap()
                    )
                ).to_address(),
                &Value::new(&BigNum::from_str("2000000").unwrap())
            )
        );
        tx_outputs.add(
            &TransactionOutput::new(
                &BaseAddress::new(
                    0,
                    &StakeCredential::from_keyhash(
                        &Ed25519KeyHash::from_bytes(
                            vec![117, 129, 184, 196, 75, 187, 125, 92, 158, 97, 73, 202, 59, 66, 83, 170, 44, 114, 145, 1, 187, 107, 28, 147, 251, 78, 148, 96]
                        ).unwrap()
                    ),
                    &StakeCredential::from_keyhash(
                        &Ed25519KeyHash::from_bytes(
                            vec![38, 151, 115, 70, 248, 194, 90, 18, 246, 16, 30, 10, 6, 56, 90, 186, 209, 141, 101, 83, 13, 66, 2, 3, 168, 86, 11, 113]
                        ).unwrap()
                    ),
                ).to_address(),
                &Value::new(&BigNum::from_str("240087054").unwrap())
            )
        );
        assert_eq!(
            tx.body().outputs(), 
            tx_outputs
        );
        assert_eq!(
            tx.body().fee(),
            BigNum::from_str("2000000").unwrap()
        );
        assert_eq!(
            tx.body().ttl_bignum().unwrap(), 
            BigNum::from_str("1800").unwrap()
        );
        let mut cert = Certificates::new();
        cert.add(&Certificate::new_stake_deregistration(
            &StakeDeregistration::new(
                &StakeCredential::from_keyhash(
                    &Ed25519KeyHash::from_bytes(
                        vec![38, 151, 115, 70, 248, 194, 90, 18, 246, 16, 30, 10, 6, 56, 90, 186, 209, 141, 101, 83, 13, 66, 2, 3, 168, 86, 11, 113]
                    ).unwrap()
                )
            )
        ));
        assert_eq!(
            tx.body().certs(), 
            Some(
                cert
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