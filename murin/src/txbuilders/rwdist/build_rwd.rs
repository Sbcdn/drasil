/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use super::*;
use crate::error::MurinError;
use crate::hfn;
use crate::htypes::*;
use crate::wallet::*;
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr, crypto as ccrypto, utils as cutils};
use std::str::from_utf8;

#[allow(clippy::too_many_arguments)]
pub fn perform_rwd(
    fee: &cutils::BigNum,
    ns_addr: &str,
    ns_script: &String,
    ns_version: &String,
    gtxd: &super::TxData,
    rwdtxd: &super::RWDTxData,
    pvks: &[String],
    //    apply_system_fee: &bool,
    dummy: bool,
) -> Result<
    (
        clib::TransactionBody,
        clib::TransactionWitnessSet,
        clib::metadata::AuxiliaryData,
        TransactionUnspentOutputs,
        usize,
    ),
    MurinError,
> {
    if dummy {
        info!("--------------------------------------------------------------------------------------------------------");
        info!("-----------------------------------------Fee calcualtion------------------------------------------------");
        info!("---------------------------------------------------------------------------------------------------------\n");
    } else {
        info!("--------------------------------------------------------------------------------------------------------");
        info!("-----------------------------------------Build Transaction----------------------------------------------");
        info!("--------------------------------------------------------------------------------------------------------\n");
    }

    //let rwd_system_fee = env::var("SYSTEM_FEE_RWD")?.parse::<u64>()?;
    //let rwd_system_fee_wallet = caddr::Address::from_bech32(&env::var("SYSTEM_FEE_WALLET")?)?;
    let native_script_address = caddr::Address::from_bech32(ns_addr)?;

    let recipient_address = rwdtxd.get_payment_addr();
    debug!("Recipent Address: {:?}", recipient_address);
    let recipient_address_bech32 = recipient_address.to_bech32(None)?;
    let _recipient_payment_addr = get_pubkey(&recipient_address)?;
    let _recipient_stake_addr = get_stake_address(&recipient_address)?;

    let recipient_address_bech32_1 = &recipient_address_bech32[0..62];
    let recipient_address_bech32_2 = &recipient_address_bech32[62..];

    /////////////////////////////////////////////////////////////////////////////////////////////////////
    //
    //Auxiliary Data
    //  Plutus Script and Metadata
    /////////////////////////////////////////////////////////////////////////////////////////////////////
    let mut aux_data = clib::metadata::AuxiliaryData::new();
    let mut general_metadata = clib::metadata::GeneralTransactionMetadata::new();
    let mut raw_metadata = Vec::<String>::new();

    for token in rwdtxd.get_reward_tokens() {
        raw_metadata.push(hex::encode(token.0.to_bytes()));
        raw_metadata.push(hex::encode(token.1.name()));
        raw_metadata.push(token.2.to_str());
    }
    raw_metadata.push("rewards distributed to:".to_string());

    debug!("Datum Metadata: {:?}\n", raw_metadata);

    let mut metalist = clib::metadata::MetadataList::new();
    for dat in raw_metadata {
        debug!("Datum Metadata Iterator: {:?}", dat);
        metalist.add(&clib::metadata::TransactionMetadatum::new_bytes(
            dat.as_bytes().to_vec(),
        )?);
    }
    metalist.add(&clib::metadata::TransactionMetadatum::new_bytes(
        rwdtxd.get_stake_addr().to_bytes(),
    )?);
    metalist.add(&clib::metadata::TransactionMetadatum::new_bytes(
        recipient_address_bech32_1.as_bytes().to_vec(),
    )?);
    metalist.add(&clib::metadata::TransactionMetadatum::new_bytes(
        recipient_address_bech32_2.as_bytes().to_vec(),
    )?);
    metalist.add(&clib::metadata::TransactionMetadatum::new_bytes(
        ns_version.as_bytes().to_vec(),
    )?);

    let metadata = clib::metadata::TransactionMetadatum::new_list(&metalist);
    general_metadata.insert(&cutils::to_bignum(631u64), &metadata);
    aux_data.set_metadata(&general_metadata);
    let aux_data_hash = cutils::hash_auxiliary_data(&aux_data);

    //////////////////////////////////////////////////////////////////////////////////////////////////////
    //Add Inputs and Outputs
    //
    //
    ///////////////////////////////////////////////////////////////////////////////////////////////////////
    // TODO: FRONT-END HAVE TO SEND USED WALLET NOT UNUSED!
    //let trade_owner = &gtxd.clone().get_senders_addresses();
    //if !trade_owner.contains(&recipient_address) {
    //    return Err(MurinError::new(
    //        "Error: RewardAddress is not contained in input addresses, this is mandatory",
    //    ));
    //}

    let mut txouts = clib::TransactionOutputs::new();
    let zero_val = cutils::Value::new(&cutils::to_bignum(0u64));

    debug!("Untouched Reward UTXOS: {:?}", rwdtxd.get_reward_utxos());
    debug!("Reward Tokens to claim: {:?}", rwdtxd.get_reward_tokens());

    let mut rwd_val = tokens_to_value(&rwdtxd.get_reward_tokens());
    let min_utxo_val = calc_min_ada_for_utxo(&rwd_val, None);
    rwd_val.set_coin(&min_utxo_val);

    debug!("RWD VAL: {:?}", rwd_val);
    // Add Rewards to Transactions Outputs
    if rwd_val.compare(&zero_val).unwrap() >= 1 {
        txouts.add(&clib::TransactionOutput::new(&recipient_address, &rwd_val));
    }

    let mut rwd_utxos_avail = rwdtxd.get_reward_utxos().unwrap();
    if let Some(used_utxos) = crate::utxomngr::usedutxos::check_any_utxo_used(&rwd_utxos_avail)? {
        debug!("\n\n");
        debug!("USED RWD UTXOS: {:?}", used_utxos);
        debug!("\n\n");
        rwd_utxos_avail.remove_used_utxos(used_utxos);
    }

    // Get Ada Only Utxos
    let mut rwd_coin_only = rwdtxd.get_reward_utxos().unwrap().get_coin_only();
    rwd_coin_only.sort_by_coin();

    // Reward return Value
    let mut rwd_utxo_selection = find_token_utxos_na(
        &rwd_utxos_avail,
        rwdtxd.get_reward_tokens(),
        Some(native_script_address.clone()).as_ref(),
    )?;
    let rwd_utxo_tot_val = rwd_utxo_selection.calc_total_value()?;
    debug!(
        "Selected Inputs Reward UTXOs Total Value: {:?}",
        rwd_utxo_tot_val
    );
    if rwd_utxo_tot_val.multiasset().is_some() {
        for t in rwdtxd.get_reward_tokens() {
            if let Some(assets) = rwd_utxo_tot_val.multiasset().unwrap().get(&t.0) {
                if let Some(amt) = assets.get(&t.1) {
                    if amt.compare(&t.2) < 0 {
                        return Err(MurinError::new(
                                &format!(
                                    "ERROR: Not enough Tokens of '{}' on the contract, the token provider was contacted to provide more tokens",
                                    from_utf8(&t.1.name()).unwrap())
                            ));
                    }
                }
            }
        }
    } else {
        return Err(MurinError::new("ERROR: No Tokens left on script"));
    }

    //Contract fee
    let mut contract_fee = 0u64;
    if let Some(f) = &rwdtxd.get_fee() {
        contract_fee += f;
    }

    // Add SystemFee
    //if *apply_system_fee {
    //    contract_fee = contract_fee + rwd_system_fee;
    //}

    let fee_val = cutils::Value::new(&cutils::to_bignum(contract_fee));
    debug!("\n\nFee Value: {:?}", fee_val);

    debug!("\n\nrwd_utxo_selection: {:?}", rwd_utxo_selection);
    if !rwd_utxo_selection.is_empty() {
        let mut zcoin_rval = rwd_val.clone();
        zcoin_rval.set_coin(&cutils::to_bignum(0u64));
        //Todo: Output splitten if it makes sense, good option for splitting outputs in user transactions to
        //      get more parallelizaton

        let mut return_val = rwd_utxo_tot_val.checked_sub(&zcoin_rval)?;
        return_val.set_coin(&return_val.coin().checked_add(&fee_val.coin())?);
        debug!("\n\nReturn Value: {:?}", return_val);
        if return_val.multiasset().is_some() {
            let return_values = split_value(return_val.clone())?;
            let splitted_rv = minimize_coins_on_values(return_values.0)?;

            let additional_inputs: TransactionUnspentOutputs;
            if let Some(more_ada_needed) = return_values.1 {
                additional_inputs =
                    rwd_coin_only.coin_value_subset(more_ada_needed, Some(&rwd_utxo_selection));
                //ToDO: if we have not enough Ada to Split correctly add another ada only input from
                // script and repeat
                if more_ada_needed.compare(&additional_inputs.calc_total_value()?.coin()) >= 0 {
                    rwd_utxo_selection.merge(additional_inputs);
                    for v in splitted_rv {
                        txouts.add(&clib::TransactionOutput::new(&native_script_address, &v));
                    }
                } else {
                    txouts.add(&clib::TransactionOutput::new(
                        &native_script_address,
                        &return_val,
                    ));
                }
            } else {
                for v in splitted_rv {
                    txouts.add(&clib::TransactionOutput::new(&native_script_address, &v));
                }
            }
        } else {
            txouts.add(&clib::TransactionOutput::new(
                &native_script_address,
                &return_val,
            ));
        }
    } else {
        return Err(MurinError::new(
            "ERROR: No Utxos for Reward Distribution available",
        ));
    }

    // Inputs
    let mut input_txuos = gtxd.clone().get_inputs();
    input_txuos.merge(rwd_utxo_selection.clone());

    debug!("\n Before USED UTXOS");
    // Check if some utxos in inputs are in use and remove them
    if let Some(used_utxos) = crate::utxomngr::usedutxos::check_any_utxo_used(&input_txuos)? {
        debug!("\n\n");
        debug!("USED UTXOS: {:?}", used_utxos);
        debug!("\n\n");
        input_txuos.remove_used_utxos(used_utxos);
    }
    let k = crate::utxomngr::usedutxos::check_any_utxo_used(&input_txuos)?;
    debug!("K: {:?}", k);

    debug!("\n Added reward utxos to inputs\n {:?}", rwd_utxo_selection);

    let collateral_input_txuo = gtxd.clone().get_collateral();

    //cutils::TransactionUnspentOutput::from_bytes(hex::decode(&txd.collateral_input).unwrap()).unwrap();

    debug!("\nCollateral Input: {:?}", collateral_input_txuo);

    // Balance TX
    debug!("Before Balance: Transaction Inputs: {:?}", input_txuos);
    debug!("Before Balance: Transaction Outputs: {:?}", txouts);

    let mut fee_paied = false;
    let mut first_run = true;
    let mut txos_paied = false;
    let mut tbb_values = cutils::Value::new(&cutils::to_bignum(0u64));
    let mut acc = cutils::Value::new(&cutils::to_bignum(0u64));
    let change_address = recipient_address.clone();

    let mut needed_value = hfn::sum_output_values(&txouts);
    needed_value.set_coin(&needed_value.coin().checked_add(&fee.clone()).unwrap());
    let security = cutils::to_bignum(
        cutils::from_bignum(&needed_value.coin()) / 100 * 10 + (2 * htypes::MIN_ADA),
    ); // 10% Security for min utxo etc.
    needed_value.set_coin(&needed_value.coin().checked_add(&security).unwrap());

    debug!("Needed Value: {:?}", needed_value);
    debug!(
        "\n\n\n\n\nTxIns Before selection:\n {:?}\n\n\n\n\n",
        input_txuos
    );

    let (txins, mut input_txuos) = super::input_selection(
        None,
        &mut needed_value,
        &input_txuos,
        gtxd.clone().get_collateral(),
        Some(native_script_address).as_ref(),
    )?;
    let saved_input_txuos = input_txuos.clone();
    debug!("Saved Inputs: {:?}", saved_input_txuos);

    let mut vkey_counter = hfn::get_vkey_count(&input_txuos, collateral_input_txuo.as_ref()) + 1; // +1 dues to signature in finalize
    debug!(
        "\n\n\n\n\nTxIns Before Balance:\n {:?}\n\n\n\n\n",
        input_txuos
    );

    // ToDo:
    // Need to rewrite balancing, we have two change addresses, reward tokens and the minAda on it needs to go back to the
    // reward wallet, the rest user input needs to go back to the users walet, am besten in zwei schritten,
    // erst Outputs für Reward wallet hinzufügen und dann auf user wallet wechseln.
    // -> Should be solved with the value substraction above -> Check
    //let mut already_balanced = cutils::Value::new(&rwd_utxo_tot_val.coin());
    //already_balanced.set_multiasset(multiasset: &MultiAsset);
    debug!("\n\nRWD Tot Value: {:?}", rwd_utxo_tot_val);
    let txouts_fin = hfn::balance_tx(
        &mut input_txuos,
        &rwdtxd.get_reward_tokens(),
        &mut txouts,
        Some(rwd_utxo_tot_val).as_ref(),
        fee,
        &mut fee_paied,
        &mut first_run,
        &mut txos_paied,
        &mut tbb_values,
        &recipient_address,
        &change_address,
        &mut acc,
        None,
        &dummy,
    )?;

    let slot = cutils::to_bignum(
        gtxd.clone().get_current_slot() + hfn::get_ttl_tx(&gtxd.clone().get_network()),
    );
    let mut txbody = clib::TransactionBody::new_tx_body(&txins, &txouts_fin, fee);
    txbody.set_ttl(&slot);
    debug!("\nTxOutputs: {:?}\n", txbody.outputs());
    debug!("\nTxInputs: {:?}\n", txbody.inputs());

    txbody.set_auxiliary_data_hash(&aux_data_hash);

    // Set network Id
    if gtxd.get_network() == clib::NetworkIdKind::Testnet {
        txbody.set_network_id(&clib::NetworkId::testnet());
    } else {
        txbody.set_network_id(&clib::NetworkId::mainnet());
    }
    /*
    let mut req_signers = clib::Ed25519KeyHashes::new();
    let pkh1 = clib::crypto::PrivateKey::from_bech32(&pvks.get(0).unwrap())?.to_public().hash();
    let pkh2 = clib::crypto::PrivateKey::from_bech32(&pvks.get(1).unwrap())?.to_public().hash();

    req_signers.add(&pkh1);
    req_signers.add(&pkh2);
    txbody.set_required_signers(&req_signers);
    */
    let native_script = clib::NativeScript::from_bytes(hex::decode(ns_script)?)?;

    // For Debugging
    // The PKH in required signers are correct with the pkhs of the private keys, why does the native script still fail ?
    //let req_signer = native_script.get_required_signers();
    //debug!("Len Req SIgner: {:?}",req_signer.len());
    //for i in 0..req_signer.len() {
    //    debug!("Required Signer: {:?}" ,req_signer.get(i).to_bech32("pkh_")) //req_signer.len()
    //}

    //ToDO:
    // Make first signature and store the txwitness for reuse
    let mut txwitness = clib::TransactionWitnessSet::new();
    let mut native_scripts = clib::NativeScripts::new();
    native_scripts.add(&native_script);
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
    if vkey_counter < 3 {
        info!("Vkey Counter was smaller than 3 why?: {:?}", vkey_counter);
        info!("Inputs: {:?}", input_txuos);
        vkey_counter = 3;
    }
    info!("Vkey Counter at End: {:?}", vkey_counter);

    Ok((txbody, txwitness, aux_data, saved_input_txuos, vkey_counter))
}

pub async fn build_rwd_multisig(
    gtxd: &super::TxData,
    rwdtxd: &super::RWDTxData,
    pvks: &[String],
    ns_addr: &str,
    ns_script: &String,
    ns_version: &String,
    //    apply_system_fee: &bool,
) -> Result<htypes::BuildOutput, MurinError> {
    // Temp until Protocol Parameters fixed
    let mem = cutils::to_bignum(7000000u64); //cutils::to_bignum(7000000u64);
    let steps = cutils::to_bignum(2500000000u64); //cutils::to_bignum(3000000000u64);
    let ex_unit_price: htypes::ExUnitPrice = crate::ExUnitPrice {
        priceSteps: 7.21e-5,
        priceMemory: 5.77e-2,
    };
    //serde_json::from_str("{'executionUnitPrices': {'priceSteps': 7.21e-5,'priceMemory': 5.77e-2}").unwrap();
    let a = cutils::to_bignum(44u64);
    let b = cutils::to_bignum(155381u64);

    //

    //Create Tx
    let (txbody_, mut txwitness_, aux_data_, _, vkey_counter) = perform_rwd(
        &cutils::to_bignum(2000000),
        ns_addr,
        ns_script,
        ns_version,
        gtxd,
        rwdtxd,
        pvks,
        //    apply_system_fee,
        true,
    )?;

    let dummy_vkeywitnesses = hfn::make_dummy_vkeywitnesses(vkey_counter);
    txwitness_.set_vkeys(&dummy_vkeywitnesses);

    // Build and encode dummy transaction
    let transaction_ = clib::Transaction::new(&txbody_, &txwitness_, Some(aux_data_));

    let calculated_fee = hfn::calc_txfee(
        &transaction_,
        &a,
        &b,
        ex_unit_price.clone(),
        &steps,
        &mem,
        true,
    );
    let (txbody, txwitness, aux_data, used_utxos, vkey_counter_2) = perform_rwd(
        &calculated_fee,
        ns_addr,
        ns_script,
        ns_version,
        gtxd,
        rwdtxd,
        pvks,
        //    apply_system_fee,
        false,
    )?;

    let transaction2 = clib::Transaction::new(&txbody, &txwitness_, Some(aux_data.clone()));

    if vkey_counter_2 != vkey_counter
        || transaction2.to_bytes().len() != transaction_.to_bytes().len()
    {
        let dummy_vkeywitnesses = hfn::make_dummy_vkeywitnesses(vkey_counter_2);
        txwitness_.set_vkeys(&dummy_vkeywitnesses);

        let calculated_fee =
            hfn::calc_txfee(&transaction2, &a, &b, ex_unit_price, &steps, &mem, true);
        let (txbody, txwitness, aux_data, used_utxos, _) = perform_rwd(
            &calculated_fee,
            ns_addr,
            ns_script,
            ns_version,
            gtxd,
            rwdtxd,
            pvks,
            //        apply_system_fee,
            false,
        )?;
        info!("Fee: {:?}", calculated_fee);
        Ok(hfn::tx_output_data(
            txbody,
            txwitness,
            aux_data,
            used_utxos.to_hex()?,
            0u64,
            false,
        )?)
    } else {
        info!("Fee: {:?}", calculated_fee);
        Ok(hfn::tx_output_data(
            txbody,
            txwitness,
            aux_data,
            used_utxos.to_hex()?,
            0u64,
            false,
        )?)
    }
}
