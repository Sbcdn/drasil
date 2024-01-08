use super::*;
use crate::error::MurinError;
use crate::modules::transfer::models::Sink;
use crate::modules::transfer::models::Source;
use crate::modules::transfer::models::TransBuilder;
use crate::modules::transfer::models::TransModificator;
use crate::modules::transfer::models::TransWallets;
use crate::modules::transfer::models::Transfer;
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{crypto as ccrypto, utils as cutils};

// Reward Transaction Builder Type
#[derive(Debug, Clone)]
pub struct AtRWDBuilder {
    pub stxd: RWDTxData,
    pub wallets: Option<TransWallets>,
}

pub type AtRWDParams<'a> = (&'a RWDTxData, Option<TransWallets>);

impl AtRWDBuilder {
    pub fn set_wallets(&mut self, wallets: TransWallets) {
        self.wallets = Some(wallets);
    }
    pub fn get_wallets(&self) -> Option<TransWallets> {
        self.wallets.clone()
    }
}

impl<'a> super::PerformTxb<AtRWDParams<'a>> for AtRWDBuilder {
    fn new(t: AtRWDParams) -> Self {
        AtRWDBuilder {
            stxd: t.0.clone(),
            wallets: t.1,
        }
    }

    fn perform_txb(
        &self,
        fee: &clib::utils::BigNum,
        gtxd: &TxData,
        _pvks: &[String], // Deprecated
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

        //let native_script_address = self.script_addr.clone();
        let reward_recipient_address = self.stxd.get_payment_addr();

        let mut builder = TransBuilder::new(&reward_recipient_address);

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
            log::debug!("\n\nCreated empty wallet in build_rwd");
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
        let recipient_address_bech32 = reward_recipient_address.to_bech32(None)?;

        let recipient_address_bech32_1 = &recipient_address_bech32[0..62];
        let recipient_address_bech32_2 = &recipient_address_bech32[62..];

        let mut aux_data = clib::metadata::AuxiliaryData::new();
        let mut general_metadata = clib::metadata::GeneralTransactionMetadata::new();
        let mut raw_metadata = Vec::<String>::new();

        for token in self.stxd.get_rewards() {
            raw_metadata.push(hex::encode(token.get_policy_id()?.to_bytes()));
            raw_metadata.push(hex::encode(token.get_assetname()?.name()));
            raw_metadata.push(token.get_amount()?.to_str());
        }
        raw_metadata.push("rewards distributed to:".to_string());

        trace!("Datum Metadata: {:?}\n", raw_metadata);

        let mut metalist = clib::metadata::MetadataList::new();
        metalist.add(&clib::metadata::TransactionMetadatum::new_text(
            "smartclaimz.io".to_string(),
        )?);
        for dat in raw_metadata {
            trace!("Datum Metadata Iterator: {:?}", dat);
            metalist.add(&clib::metadata::TransactionMetadatum::new_bytes(
                dat.as_bytes().to_vec(),
            )?);
        }
        metalist.add(&clib::metadata::TransactionMetadatum::new_bytes(
            self.stxd.get_stake_addr().to_bytes(),
        )?);
        metalist.add(&clib::metadata::TransactionMetadatum::new_bytes(
            recipient_address_bech32_1.as_bytes().to_vec(),
        )?);
        metalist.add(&clib::metadata::TransactionMetadatum::new_bytes(
            recipient_address_bech32_2.as_bytes().to_vec(),
        )?);

        let metadata = clib::metadata::TransactionMetadatum::new_list(&metalist);
        general_metadata.insert(&cutils::to_bignum(631u64), &metadata);
        aux_data.set_metadata(&general_metadata);
        let aux_data_hash = cutils::hash_auxiliary_data(&aux_data);

        debug!("Reward Tokens to claim: {:?}", self.stxd.get_rewards());

        let rwd_contract_ids =
            self.stxd
                .get_rewards()
                .iter()
                .fold(Vec::<i64>::new(), |mut acc, n| {
                    if !acc.contains(&n.get_contract_id()) {
                        acc.push(n.get_contract_id());
                    }
                    acc
                });

        let mut transfers = Vec::<Transfer>::new();
        for id in &rwd_contract_ids {
            let c_rewards =
                &self
                    .stxd
                    .get_rewards()
                    .iter()
                    .fold(Vec::<RewardHandle>::new(), |mut acc, n| {
                        if n.contract_id == *id {
                            acc.push(n.clone());
                        }
                        acc
                    });
            let rwd_val = RewardHandle::total_value(c_rewards)?;
            let min_utxo_val =
                min_ada_for_utxo(&TransactionOutput::new(&reward_recipient_address, &rwd_val))?
                    .amount();
            let mut zcrwd_val = rwd_val.clone();
            zcrwd_val.set_coin(&cutils::to_bignum(0));

            log::debug!("script wallet getter");
            let script_wallet = wallets.get_wallet_cid(*id)?;

            let sink = Sink::new(&reward_recipient_address, &min_utxo_val);
            let mut source = Source::new(&script_wallet.script.unwrap().script_addr);
            source.add_subtraction(&cutils::Value::new(&min_utxo_val.coin()));
            source.set_pay_value(zcrwd_val);
            let trans = Transfer::new(&source, &vec![sink]);
            transfers.push(trans);
        }

        //Contract fee
        let mut contract_fee = 0u64;
        if let Some(f) = &self.stxd.get_fee() {
            contract_fee += f;
        }
        //log::debug!("any wallet call.....");
        let any_script_wallet = wallets.get_wallet_cid(rwd_contract_ids[0])?;

        let cfee_val = cutils::Value::new(&cutils::to_bignum(contract_fee));
        let contract_fee_sink =
            Sink::new(&any_script_wallet.script.unwrap().script_addr, &cfee_val);

        let mut fee_paying_source = Source::new(&reward_recipient_address);
        fee_paying_source.set_pay_value(cfee_val);

        for t in transfers.clone() {
            let v = t.get_source().get_modificator();
            for m in v {
                match m {
                    TransModificator::Add(d) => {
                        fee_paying_source.add_subtraction(&d);
                    }
                    TransModificator::Sub(d) => {
                        fee_paying_source.add_addition(&d);
                    }
                }
            }
        }

        transfers.push(Transfer::new(&fee_paying_source, &vec![contract_fee_sink]));

        builder.transfers = transfers;
        builder.wallets = wallets;

        builder.build(*fee).unwrap(); // ToDo: Implement error

        let saved_input_txuos = builder.tx.clone().unwrap().0;
        let mut vkey_counter =
            supporting_functions::get_vkey_count(&builder.tx.as_ref().unwrap().0, None)
                + rwd_contract_ids.len(); // +1 due to signature in finalize
        let slot = cutils::to_bignum(
            gtxd.clone().get_current_slot()
                + supporting_functions::get_ttl_tx(&gtxd.clone().get_network()),
        );
        let mut txbody = clib::TransactionBody::new_tx_body(
            &builder.tx.as_ref().unwrap().1,
            &builder.tx.as_ref().unwrap().2,
            fee,
        );
        txbody.set_ttl(&slot);

        txbody.set_auxiliary_data_hash(&aux_data_hash);

        let mut txwitness = clib::TransactionWitnessSet::new();
        let mut vkeywitnesses = ccrypto::Vkeywitnesses::new();
        let mut native_scripts = clib::NativeScripts::new();

        for id in rwd_contract_ids {
            log::debug!("wallet_id: {:?}", id);
            //log::debug!("wallets: {:?}", self.wallets);
            let w = builder.wallets.get_wallet_cid(id)?;
            native_scripts.add(&w.script.as_ref().unwrap().script);

            let root_key1 = clib::crypto::Bip32PrivateKey::from_bytes(&hex::decode(
                &w.script.as_ref().unwrap().get_pvks()[0],
            )?)?;
            let account_key1 = root_key1
                .derive(harden(1852u32))
                .derive(harden(1815u32))
                .derive(harden(0u32));
            let prv1 = account_key1.to_raw_key(); // for signatures
            let vkwitness_1d1 =
                cutils::make_vkey_witness(&cutils::hash_transaction(&txbody), &prv1);

            vkeywitnesses.add(&vkwitness_1d1);

            debug!("TxWitness: {:?}", hex::encode(txwitness.to_bytes()));
        }
        txwitness.set_native_scripts(&native_scripts);
        txwitness.set_vkeys(&vkeywitnesses);

        debug!("TxBody: {:?}", hex::encode(txbody.to_bytes()));
        debug!("--------------------Iteration Ended------------------------------");
        if vkey_counter < 3 {
            info!("Vkey Counter was smaller than 3 why?: {:?}", vkey_counter);
            //info!("Inputs: {:?}", input_txuos);
            vkey_counter = 3;
        }
        info!("Vkey Counter at End: {:?}", vkey_counter);

        Ok((
            txbody,
            txwitness,
            Some(aux_data),
            saved_input_txuos,
            vkey_counter,
            false,
        ))
    }
}
