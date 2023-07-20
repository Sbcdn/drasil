use crate::error::MurinError;
use crate::hfn::{balance_tx, get_ttl_tx, get_vkey_count, sum_output_values};
use crate::htypes::*;
use crate::minter::*;
use crate::txbuilders::minter::MinterTxData;
use crate::txbuilders::{calc_min_ada_for_utxo, harden, input_selection, TxBO};
use crate::TxData;
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr, crypto as ccrypto, utils as cutils};

// One Shot Minter Builder Type
#[derive(Debug, Clone)]
pub struct AtOSMBuilder {
    pub liquidity_addr: clib::address::Address,
    pub script: clib::NativeScript,
    pub stxd: MinterTxData,
}

pub type AtOSMParams<'a> = (clib::address::Address, clib::NativeScript, &'a MinterTxData);

impl<'a> super::PerformTxb<AtOSMParams<'a>> for AtOSMBuilder {
    fn new(t: AtOSMParams) -> Self {
        AtOSMBuilder {
            liquidity_addr: t.0,
            script: t.1,
            stxd: t.2.clone(),
        }
    }

    fn perform_txb(
        &self,
        fee: &clib::utils::BigNum,
        gtxd: &TxData,
        pvks: &[String],
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

        let receiving_address: caddr::Address = self.stxd.get_payment_addr();

        let receiving_address_bech32 = receiving_address.to_bech32(None)?;
        debug!("Recipent Address: {:?}", receiving_address_bech32);

        let mintpolicy = self.script.hash();
        debug!("Policy ID: {:?}", hex::encode(mintpolicy.to_bytes()));
        let minttokens = mintasset_into_tokenasset(self.stxd.get_mint_tokens(), mintpolicy.clone());

        /////////////////////////////////////////////////////////////////////////////////////////////////////
        //
        //Auxiliary Data
        //  Plutus Script and Metadata
        /////////////////////////////////////////////////////////////////////////////////////////////////////
        let mut aux_data = clib::metadata::AuxiliaryData::new();
        let metadata = make_mint_metadata_from_json(
            &self.stxd.get_metadata(),
            minttokens.clone(),
            mintpolicy.clone(),
        )?;
        aux_data.set_metadata(&metadata);
        let aux_data_hash = cutils::hash_auxiliary_data(&aux_data);

        //////////////////////////////////////////////////////////////////////////////////////////////////////
        //Add Inputs and Outputs
        //
        //
        ///////////////////////////////////////////////////////////////////////////////////////////////////////

        let mut txouts = clib::TransactionOutputs::new();
        let _zero_val = cutils::Value::new(&cutils::to_bignum(0u64));

        let mut mint_val = tokens_to_value(&minttokens.clone());
        let min_utxo_val = calc_min_ada_for_utxo(&mint_val, None);
        mint_val.set_coin(&min_utxo_val);

        txouts.add(&clib::TransactionOutput::new(&receiving_address, &mint_val));

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
        let mut acc = cutils::Value::new(&cutils::to_bignum(0u64));
        let change_address = self.liquidity_addr.clone();

        let mut needed_value = sum_output_values(&txouts);
        needed_value.set_coin(&needed_value.coin().checked_add(&fee.clone()).unwrap());
        let security =
            cutils::to_bignum(cutils::from_bignum(&needed_value.coin()) / 100 * 10 + (2 * MIN_ADA)); // 10% Security for min utxo etc.
        needed_value.set_coin(&needed_value.coin().checked_add(&security).unwrap());
        let mut needed_value = cutils::Value::new(&needed_value.coin());

        debug!("Needed Value: {:?}", needed_value);

        //let mut signers_address_utxos = (TransactionUnspentOutputs::new(),TransactionUnspentOutputs::new());
        //if let Some(signer) = minttxd.get_signer() {
        //    signers_address_utxos = find_utxos_by_address(signer.clone(), &input_txuos);
        //}

        //println!("Signer Address UTXOS: {:?}",signers_address_utxos.0);
        let (txins, mut input_txuos) = input_selection(
            None,
            &mut needed_value,
            &input_txuos,
            gtxd.clone().get_collateral(),
            None, //Some(native_script_address).as_ref(),
        )?;

        /*
        if let Some(signer) = minttxd.get_signer() {
            if input_txuos.contains_any(&signers_address_utxos.0) {
                info!("\n\nUtxo Input set contains minimum one utxo from the listing address\n\n");
            } else {
                if !signers_address_utxos.0.is_empty() {
                    // ToDo:
                    // In this case it would be better to have some larger Ada only Utxo -> Create a function to find one
                    txins.add(&signers_address_utxos.0.get(0).input());
                    input_txuos.add(&signers_address_utxos.0.get(0));
                } else {
                    return Err(
                        MurinError::new(
                            &format!(
                                "Error: The Address which is needed for signature does not contain ADA, please provide suitable amount of ADA to: {:?}",signer.to_bech32(None)
                            )
                        )
                    )
                }
            }
        }
         */
        let saved_input_txuos = input_txuos.clone();

        let vkey_counter = get_vkey_count(&input_txuos, None) + 1; // +1 dues to signature in finalize

        // ToDo:
        let mut mint_val_zero_coin = mint_val.clone();
        mint_val_zero_coin.set_coin(&cutils::to_bignum(0u64));

        let txouts_fin = balance_tx(
            &mut input_txuos,
            &minttokens,
            &mut txouts,
            Some(mint_val_zero_coin).as_ref(), // but not the ADA!!!!
            fee,
            &mut fee_paied,
            &mut first_run,
            &mut txos_paied,
            &mut tbb_values,
            &change_address, //who is sender ?
            &change_address,
            &mut acc,
            None,
            &fcrun,
        )?;

        ////////////////////////////////////////////////////////////////////////////////////////////
        //
        // MINT ASSETS
        //
        ////////////////////////////////////////////////////////////////////////////////////////////
        let mut mintasset = clib::MintAssets::new();

        for token in minttokens {
            mintasset.insert(&token.1, clib::utils::Int::new(&token.2));
        }

        let mint = clib::Mint::new_from_entry(&mintpolicy, &mintasset);

        let slot = cutils::to_bignum(
            gtxd.clone().get_current_slot() + get_ttl_tx(&gtxd.clone().get_network()),
        );
        let mut txbody = clib::TransactionBody::new_tx_body(&txins, &txouts_fin, fee);
        txbody.set_ttl(&slot);

        txbody.set_auxiliary_data_hash(&aux_data_hash);

        txbody.set_mint(&mint);

        // Set network Id
        if gtxd.get_network() == clib::NetworkIdKind::Testnet {
            txbody.set_network_id(&clib::NetworkId::testnet());
        } else {
            txbody.set_network_id(&clib::NetworkId::mainnet());
        }

        let req_signer = self.script.get_required_signers();
        info!("Len Req SIgner: {:?}", req_signer.len());
        for i in 0..req_signer.len() {
            info!("Required Signer: {:?}", req_signer.get(i).to_bech32("pkh_")) //req_signer.len()
        }

        let mut txwitness = clib::TransactionWitnessSet::new();
        let mut native_scripts = clib::NativeScripts::new();
        native_scripts.add(&self.script);
        txwitness.set_native_scripts(&native_scripts);

        let root_key1 = clib::crypto::Bip32PrivateKey::from_bytes(&hex::decode(&pvks[0])?)?;
        let account_key1 = root_key1
            .derive(harden(1852u32))
            .derive(harden(1815u32))
            .derive(harden(0u32));
        let prv1 = account_key1.to_raw_key(); // for signatures
        let vkwitness_1d1 = cutils::make_vkey_witness(&cutils::hash_transaction(&txbody), &prv1);

        let mut vkeywitnesses = ccrypto::Vkeywitnesses::new();
        vkeywitnesses.add(&vkwitness_1d1);
        txwitness.set_vkeys(&vkeywitnesses);
        debug!("TxWitness: {:?}", hex::encode(txwitness.to_bytes()));

        debug!("TxBody: {:?}", hex::encode(txbody.to_bytes()));
        debug!("--------------------Iteration Ended------------------------------");
        debug!("Vkey Counter at End: {:?}", vkey_counter);
        Ok((txbody, txwitness, aux_data, saved_input_txuos, vkey_counter))
    }
}
