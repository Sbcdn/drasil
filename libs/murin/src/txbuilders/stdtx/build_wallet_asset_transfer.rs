/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use super::StandardTxData;
use crate::error::MurinError;
use crate::hfn::{self};
use crate::min_ada_for_utxo;
use crate::modules::transfer::models::{Sink, Source, TransBuilder, TransWallets, Transfer};
use crate::txbuilders::TxBO;
use crate::PerformTxb;
use crate::TxData;
use cardano_serialization_lib as clib;
use clib::address::Address;
use clib::utils::{hash_auxiliary_data, to_bignum};
use clib::{MultiAsset, TransactionOutput};

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
        _: &[String],
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
        let aux_data = clib::metadata::AuxiliaryData::new();
        // ToDo:set messages into metadata from wallet
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
        builder.build(*fee).unwrap(); // ToDo: Implement error

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

        //txbody.set_auxiliary_data_hash(&aux_data_hash);

        debug!("TxBody: {:?}", hex::encode(txbody.to_bytes()));
        debug!("--------------------Iteration Ended------------------------------");
        info!("Vkey Counter: {:?}", vkey_counter);

        // empty witness
        let txwitness = clib::TransactionWitnessSet::new();

        Ok((txbody, txwitness, aux_data, saved_input_txuos, vkey_counter))
    }
}
