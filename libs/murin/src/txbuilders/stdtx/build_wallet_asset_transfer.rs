use super::StandardTxData;
use crate::error::MurinError;
use crate::hfn::{self};
use crate::min_ada_for_utxo;
use crate::modules::transfer::models::{Sink, Source, TransBuilder, TransWallets, TransWallet, Transfer};
use crate::txbuilders::{TxBO, harden};
use crate::PerformTxb;
use crate::TxData;

use cardano_serialization_lib as clib;
use cardano_serialization_lib::utils as cutils;
use clib::address::Address;
use clib::metadata::GeneralTransactionMetadata;
use clib::utils::{hash_auxiliary_data, to_bignum};
use clib::{MultiAsset, TransactionOutput};
use std::mem::size_of_val;



// One Shot Minter Builder Type
#[derive(Debug, Clone)]
pub struct AtSATBuilder {
    pub stxd: StandardTxData,
    pub fee_paying_address: Address,
    pub wallets: Option<TransWallets>,
}

impl AtSATBuilder {
    pub fn set_wallets(&mut self, wallets: TransWallets) {
        self.wallets = Some(wallets);
    }
    pub fn get_wallets(&self) -> Option<TransWallets> {
        self.wallets.clone()
    }
}

pub type AtSATParams<'a> = (&'a StandardTxData, &'a TransWallets, &'a Address);

impl<'a> PerformTxb<AtSATParams<'a>> for AtSATBuilder {
    fn new(t: AtSATParams) -> Self {
        AtSATBuilder {
            stxd: t.0.clone(),
            wallets: Some(t.1.clone()),
            fee_paying_address: t.2.clone(),
        }
    }

    fn perform_txb(
        &self,
        fee: &clib::utils::BigNum,
        gtxd: &TxData,
        pvks: &[String],
        fcrun: bool,
    ) -> std::result::Result<TxBO, MurinError> {

        if size_of_val(pvks) > 64 {
            return Err(MurinError::new("pvks must be max 64 bytes"));
        }
        pvks.iter().for_each(|s| {
            if size_of_val(s) > 64 {
                panic!("pvks element must be max 64 bytes");
            }
            if s.len() > 100 {
                panic!("pvks element must have max 100 characters");
            }
        });

        if fcrun {
            info!("--------------------------------------------------------------------------------------------------------");
            info!("-----------------------------------------Fee Calculation------------------------------------------------");
            info!("---------------------------------------------------------------------------------------------------------\n");
        } else {
            info!("--------------------------------------------------------------------------------------------------------");
            info!("-----------------------------------------Build Transaction----------------------------------------------");
            info!("--------------------------------------------------------------------------------------------------------\n");
        }

        let mut builder = TransBuilder::new(&self.fee_paying_address);

        let wallets = if let Some(mut w) = self.wallets.to_owned() {
            w.wallets.iter_mut().for_each(|n| {
                if let Some(used_utxos) =
                    crate::utxomngr::usedutxos::check_any_utxo_used(&n.utxos).unwrap()
                {
                    n.utxos.remove_used_utxos(used_utxos);
                }
            });
            log::debug!("\n\nTook Input wallets");
            w
        } else {
            log::debug!("\n\nCreated empty wallet in wallet asset transfer");
            TransWallets::new()
        };

        if let Some(w) = self.get_wallets() {
            builder.wallets = w;
        }

        /////////////////////////////////////////////////////////////////////////////////////////////////////
        //
        //Auxiliary Data
        //  Plutus Script and Metadata
        /////////////////////////////////////////////////////////////////////////////////////////////////////
        
        // aux_data_hash
        // ToDo:set messages into metadata from wallet
        let mut aux_data = clib::metadata::AuxiliaryData::new();
        let mut gtm = GeneralTransactionMetadata::new();

        pvks.iter().enumerate().for_each(|(i, s)|{
            gtm.insert(
                &clib::utils::BigNum::from_str(&i.to_string()).unwrap(), 
                &clib::metadata::TransactionMetadatum::new_text(s.to_string()).unwrap()
            );
        });

        aux_data.set_metadata(&gtm);
        let aux_data_hash = hash_auxiliary_data(&aux_data);

        //////////////////////////////////////////////////////////////////////////////////////////////////////
        //Add Inputs and Outputs
        //
        //
        ///////////////////////////////////////////////////////////////////////////////////////////////////////
        let mut transfers = Vec::<Transfer>::new();
        for trans in &self.stxd.transfers {
            let mut value = trans
                .assets
                .iter()
                .fold(clib::utils::Value::zero(), |mut acc, a| {
                    match &a.policy {
                        // Ada
                        None => {
                            if a.tokenname.is_some() {
                                clib::utils::Value::zero()
                            } else {
                                acc = acc
                                    .checked_add(&clib::utils::Value::new(&a.amount))
                                    .unwrap();
                                acc
                            }
                        }
                        // Other
                        Some(p) => {
                            if a.tokenname.is_none() {
                                clib::utils::Value::zero()
                            } else {
                                let mut assets = clib::Assets::new();
                                assets.insert(a.tokenname.as_ref().unwrap(), &a.amount);
                                let mut ma = MultiAsset::new();
                                ma.insert(p, &assets);
                                acc = acc
                                    .checked_add(&clib::utils::Value::new_with_assets(
                                        &to_bignum(0),
                                        &ma,
                                    ))
                                    .unwrap();
                                acc
                            }
                        }
                    }
                });
            let min_utxo_val =
                min_ada_for_utxo(&TransactionOutput::new(&self.fee_paying_address, &value))?
                    .amount();

            if value.coin().compare(&min_utxo_val.coin()) == -1 {
                value.set_coin(&min_utxo_val.coin())
            }

            let sink = Sink::new(&trans.receiver, &value);
            let mut source = Source::new(&self.fee_paying_address);
            source.set_pay_value(value);
            let trans = Transfer::new(&source, &vec![sink]);
            transfers.push(trans);
        }

        // Prepare wallet to pay transaction fees with zero value, the build adds the fee for that wallet
        //let mut fee_paying_source = Source::new(&self.fee_paying_address);
        //fee_paying_source.set_pay_value(clib::utils::Value::zero());
        //transfers.push(Transfer::new(&fee_paying_source, &vec![]));

        builder.transfers = transfers;
        builder.wallets = wallets;
        builder.build(*fee)?;

        let saved_input_txuos = builder.tx.clone().unwrap().0;
        let vkey_counter = hfn::get_vkey_count(&builder.tx.as_ref().unwrap().0, None);
        let slot = to_bignum(
            gtxd.clone().get_current_slot() + hfn::get_ttl_tx(&gtxd.clone().get_network()),
        );
        let mut txbody = clib::TransactionBody::new_tx_body(
            &builder.tx.as_ref().unwrap().1,
            &builder.tx.as_ref().unwrap().2,
            fee,
        );
        txbody.set_ttl(&slot);

        txbody.set_auxiliary_data_hash(&aux_data_hash);

        // empty witness
        let mut txwitness = clib::TransactionWitnessSet::new();

        let mut vkeywitnesses = clib::crypto::Vkeywitnesses::new();
        let root_key1 = clib::crypto::Bip32PrivateKey::from_bytes(&hex::decode(&pvks[0])?)?;
        let account_key1 = root_key1
            .derive(harden(1852u32))
            .derive(harden(1815u32))
            .derive(harden(0u32));
        let prv1 = account_key1.to_raw_key(); // for signatures
        let vkwitness_1d1 = cutils::make_vkey_witness(&cutils::hash_transaction(&txbody), &prv1);
        vkeywitnesses.add(&vkwitness_1d1);
        txwitness.set_vkeys(&vkeywitnesses);

        debug!("TxWitness: {:?}", hex::encode(txwitness.to_bytes()));
        debug!("TxBody: {:?}", hex::encode(txbody.to_bytes()));
        debug!("--------------------Iteration Ended------------------------------");
        debug!("Vkey Counter at End: {:?}", vkey_counter);
        Ok((txbody, txwitness, aux_data, saved_input_txuos, vkey_counter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::*;
    use std::str::FromStr;
    use crate::modules::transfer::models::TransWallet;
    // use hugin::protocol::cmd::FinalizeStdTx;

    #[test]
    fn test1() -> Result<(), MurinError>{
        
        // standard_tx_data
        // let tx_id = "5091ba0e8cc9a3d63468c27b5269bc4665e6f1be7c1c025f1bb4fd2ff2ff7d0a".to_string(); //tx hash
        // let raw_tx = crate::utxomngr::txmind::read_raw_tx(&tx_id)?; // fails test
        // let raw_tx = crate::utxomngr::txmind::read_raw_tx(&"".to_string())?; // fails test
        // let raw_tx = crate::utxomngr::txmind::read_raw_tx(
        //     hugin::protocol::cmd::FinalizeStdTx::new() // importing hugin => cyclic package dependency
        // )?;
        // let standard_tx_data = StandardTxData::from_str(raw_tx.get_tx_specific_rawdata())?;
        // let standard_tx_data = StandardTxData::from_str(&"".to_string())?;

        // // trans_wallets
        // let mut trans_wallets = TransWallets::new();
        // let pay_addr = Address::from_hex("")?;
        // let utxos = &TransactionUnspentOutputs::new();
        // let trans_wallet = TransWallet::new(
        //     &pay_addr,
        //     &utxos,
        // );
        // trans_wallets.add_wallet(&trans_wallet);

        // // address
        // let address = Address::from_hex("")?;

        // // transaction builder
        // let at_sat_builder = AtSATBuilder::new((
        //     &standard_tx_data,
        //     &trans_wallets,
        //     &address,
        // ));

        // left side
        // let perform_txb = at_sat_builder.perform_txb(
        //     &clib::utils::BigNum::from_str("0").unwrap(),
        //     &TxData::new(
        //         Some(vec![0]),
        //         vec![Address::from_hex("").unwrap()],
        //         Some(Address::from_hex("").unwrap()),
        //         TransactionUnspentOutputs::new(),
        //         clib::NetworkIdKind::Testnet,
        //         100,
        //     ).unwrap(),
        //     &["".to_string()],
        //     true,
        // ).unwrap();

        // right side
        let txbo = (
            clib::TransactionBody::new_tx_body(
                &clib::TransactionInputs::new(),
                &clib::TransactionOutputs::new(),
                &clib::utils::BigNum::from_str("0").unwrap()
            ),
            clib::TransactionWitnessSet::new(),
            clib::metadata::AuxiliaryData::new(),
            TransactionUnspentOutputs::new(),
            0,
        );

        // // assertions
        // assert_eq!(perform_txb.0, txbo.0);
        // assert_eq!(perform_txb.1, txbo.1);
        // assert_eq!(perform_txb.2, txbo.2);
        // // assert_eq!(perform_txb.3, txbo.3); // TransactionUnspentOutputs
        // assert_eq!(perform_txb.4, txbo.4);

        Ok(())
    }
}