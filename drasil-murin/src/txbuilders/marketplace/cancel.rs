use crate::error::MurinError;
use crate::hfn;
use crate::htypes;
use crate::marketplace::*;
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr, crypto as ccrypto, plutus, utils as cutils};

pub fn perform_cancel(
    fee: &cutils::BigNum,
    sc_scripts: &String,
    sc_addr: &str,
    gtxd: &super::TxData,
    mptxd: &super::marketplace::MpTxData,
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
        info!("-----------------------------------------Fee Calculation------------------------------------------------");
        info!("---------------------------------------------------------------------------------------------------------\n");
    } else {
        info!("--------------------------------------------------------------------------------------------------------");
        info!("-----------------------------------------Build Transaction----------------------------------------------");
        info!("--------------------------------------------------------------------------------------------------------\n");
    }

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

    ///////////////////////////////////////////////////////////////////////////////////////////////////////////////
    //Import Plutus Script
    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    let smart_contract = plutus::PlutusScript::new(hex::decode(sc_scripts)?);
    let mut sc_scripts = plutus::PlutusScripts::new();
    sc_scripts.add(&smart_contract);

    /////////////////////////////////////////////////////////////////////////////////////////////////////
    //Auxiliary Data
    //  Plutus Script and Metadata
    /////////////////////////////////////////////////////////////////////////////////////////////////////
    let mut aux_data = clib::metadata::AuxiliaryData::new();
    let general_metadata = clib::metadata::GeneralTransactionMetadata::new();
    aux_data.set_metadata(&general_metadata);
    let aux_data_hash = cutils::hash_auxiliary_data(&aux_data);

    /////////////////////////////////////////////////////////////////////////////////////////////////////
    //Create Datum
    /////////////////////////////////////////////////////////////////////////////////////////////////////
    let mut metadata = Vec::<String>::new();
    if let Some(meta) = mptxd.get_metadata() {
        metadata = meta;
    };

    let decoded_datum = hfn::decode_datum_mp(metadata, &gtxd.get_network())?;

    //////////////////////////////////////////////////////////////////////////////////////////////////////
    //Add Inputs and Outputs
    //
    //
    ///////////////////////////////////////////////////////////////////////////////////////////////////////
    let trade_owner = clib::address::Address::from_bech32(&decoded_datum.1).unwrap();
    let mut txouts = clib::TransactionOutputs::new();
    let smart_contract_address = caddr::Address::from_bech32(sc_addr).unwrap();
    let _script_utxo = make_mp_contract_utxo_output(
        &mut txouts,
        trade_owner.clone(),
        &decoded_datum.6,
        &mptxd.tokens,
        false,
    );

    let mut input_txuos = gtxd.clone().get_inputs();

    // Check if some utxos in inputs are in use and remove them
    if let Some(used_utxos) = crate::utxomngr::usedutxos::check_any_utxo_used(&input_txuos)? {
        input_txuos.remove_used_utxos(used_utxos);
    }

    let collateral_input_txuo = gtxd.get_collateral();
    info!("\nCollateral Input: {:?}", collateral_input_txuo);

    // Balance TX
    debug!("Before Balance: Transaction Inputs: {:?}", input_txuos);
    debug!("Before Balance: Transaction Outputs: {:?}", txouts);

    let mut fee_paid = false;
    let mut first_run = true;
    let mut txos_paid = false;
    let mut tbb_values = cutils::Value::new(&cutils::to_bignum(0u64));
    let mut acc = cutils::Value::new(&cutils::to_bignum(0u64));
    let change_address = &gtxd.clone().get_senders_addresses()[0];

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

    let (signers_address_utxos, _) = hfn::find_utxos_by_address(trade_owner.clone(), &input_txuos);
    info!("\n\nTradeOwner Utxos:{:?}\n\n", signers_address_utxos);

    //
    // TODO: Ensure the Smart Contract UTXO is Part of the Input UTXOS
    //
    let token_input_utxo = hfn::find_asset_utxos_in_txuos(&input_txuos, mptxd.get_tokens());
    debug!("Token Input Utxos: {:?}", token_input_utxo);

    let (mut txins, mut input_txuos) = super::input_selection(
        None,
        &mut needed_value,
        &input_txuos,
        gtxd.get_collateral(),
        Some(smart_contract_address.clone()).as_ref(),
    )?;
    let saved_input_txuos = input_txuos.clone();
    info!("Saved Inputs: {:?}", saved_input_txuos);

    if input_txuos.contains_any(&signers_address_utxos) {
        info!("\n\nUtxo Input set contains minimum one utxo from the listing address\n\n");
    } else if !signers_address_utxos.is_empty() {
        txins.add(&signers_address_utxos.get(0).input());
        input_txuos.add(&signers_address_utxos.get(0));
    } else {
        info!("The utxo set does not contain any utxos from the listing adress, transaction would fail!");
        info!("Creating transaction for internal transfer instead!");
        let mut internal_transfer = hfn::create_ada_tx(
            &cutils::to_bignum(2000000u64), // fee: &cutils::BigNum,
            true,
            &gtxd.get_network(),
            input_txuos.clone(),
            &trade_owner,
            &trade_owner,
            5000000,
            gtxd.get_current_slot(),
        )?;

        let dummy_vkeywitnesses = hfn::make_dummy_vkeywitnesses(internal_transfer.3);
        internal_transfer.1.set_vkeys(&dummy_vkeywitnesses);

        // Build and encode dummy transaction
        let transaction_ = clib::Transaction::new(
            &internal_transfer.0,
            &internal_transfer.1,
            Some(internal_transfer.2.clone()),
        );
        let calculated_fee =
            hfn::calc_txfee(&transaction_, &a, &b, ex_unit_price, &steps, &mem, true);

        let (txbody, txwitness, aux_data, _, used_utxos) = hfn::create_ada_tx(
            &calculated_fee, // fee: &cutils::BigNum,
            true,
            &gtxd.get_network(),
            input_txuos.clone(),
            &trade_owner,
            &trade_owner,
            5000000,
            gtxd.get_current_slot(),
        )?;

        hfn::tx_output_data(
            txbody,
            txwitness,
            Some(aux_data),
            used_utxos.to_hex()?,
            0u64,
            true,
        )?;
        std::process::exit(0);
    }

    let vkey_counter = hfn::get_vkey_count(&input_txuos, collateral_input_txuo.as_ref());
    debug!(
        "\n\n\n\n\nTxIns Before Balance:\n {:?}\n\n\n\n\n",
        input_txuos
    );

    let txouts_fin = hfn::balance_tx(
        &mut input_txuos,
        mptxd.get_tokens(),
        &mut txouts,
        None,
        fee,
        &mut fee_paid,
        &mut first_run,
        &mut txos_paid,
        &mut tbb_values,
        &trade_owner,
        change_address,
        &mut acc,
        Some(smart_contract_address),
        &dummy,
    )?;

    let slot = gtxd.get_current_slot() + 3000;
    let mut txbody = clib::TransactionBody::new_tx_body(&txins, &txouts_fin, fee); //922321
    txbody.set_ttl(&cutils::to_bignum(slot));
    trace!("\nTxOutputs: {:?}\n", txbody.outputs());
    trace!("\nTxInouts: {:?}\n", txbody.inputs());

    //////////////////////////////////////////////////////////////////////////////////////////////////////
    //Required Signer
    //
    ///////////////////////////////////////////////////////////////////////////////////////////////////////
    let base_addr1 = caddr::BaseAddress::from_address(&trade_owner).unwrap();
    let base1_payment = caddr::BaseAddress::payment_cred(&base_addr1);
    let keyh1 = base1_payment.to_keyhash().unwrap();
    let mut req_signers = clib::Ed25519KeyHashes::new();
    req_signers.add(&keyh1);
    debug!("Required Signers: {:?}", req_signers);

    txbody.set_required_signers(&req_signers);
    txbody.set_auxiliary_data_hash(&aux_data_hash);

    info!("Set Fee: {:?}\n", txbody.fee());

    // Collateral Input
    let mut col_inputs = clib::TransactionInputs::new();
    if let Some(collateral) = collateral_input_txuo {
        col_inputs.add(&collateral.input());
        txbody.set_collateral(&col_inputs);
    };
    if txbody.collateral().is_none() {
        return Err(MurinError::new("Error: No collateral provided"));
    }
    //////////////////////////////////////////////////////////////////////////////////////////////////////
    //Redeemer
    //  Build Redeemer
    //
    ///////////////////////////////////////////////////////////////////////////////////////////////////////
    let mut redeemers = plutus::Redeemers::new();
    let token_utxos = find_token_utxos_na(&saved_input_txuos, mptxd.get_tokens().clone(), None)?;

    for utxo in token_utxos {
        let script_input_index = get_input_position(txins.clone(), utxo);

        let redeemer_data =
            plutus::PlutusData::new_constr_plutus_data(&plutus::ConstrPlutusData::new(
                &cutils::to_bignum(1u64), // 10.0.0. Constructor 1 oder 2 für Cancel oder Buy
                //cutils::Int::new(&cutils::to_bignum(1u64)),                 // 9.1.2
                &plutus::PlutusList::new(),
            ));

        let red = plutus::Redeemer::new(
            &plutus::RedeemerTag::new_spend(),
            &cutils::to_bignum(script_input_index.0 as u64),
            &redeemer_data,
            &plutus::ExUnits::new(&mem, &steps),
        );

        redeemers.add(&red);
        debug!("Redeemer: {:?} \n", red);
    }

    debug!("Redeemers: {:?} \n", hex::encode(redeemers.to_bytes()));

    // CostModel
    let mut cstmodls = plutus::Costmdls::new();
    let lang = plutus::Language::new_plutus_v1();
    debug!("Lang from lib: {:?}", lang);
    let costmodel = plutus::CostModel::new();

    cstmodls.insert(&lang, &costmodel);
    let lang_sb = hex::decode(LV_PLUTUSV1).unwrap();
    debug!("Lang from sb: {:?}", lang_sb);

    let mut buf = Vec::new();
    buf.extend(redeemers.to_bytes());
    buf.extend(decoded_datum.5.to_bytes());
    buf.extend(lang_sb);

    let scriptdatahash = ccrypto::ScriptDataHash::from(hfn::blake2b256(&buf));
    debug!(
        "Script Hash Data Concat: {:?}",
        hex::encode(scriptdatahash.to_bytes())
    );

    //let script_hash_data = cutils::hash_script_data(&redeemers,&cstmodls,Some(decoded_datum.5.clone()));
    //debug!("Script Hash Data Lib: {:?}",hex::encode(script_hash_data.to_bytes()));
    //In version 9.2.1 muss der data über concatenierung hergesttelt, werden dadurch wird man den Fehler PPViewHashesDontMatch los
    txbody.set_script_data_hash(&scriptdatahash);
    //txbody.set_script_data_hash(&script_hash_data);

    // Set network Id
    if gtxd.get_network() == clib::NetworkIdKind::Testnet {
        txbody.set_network_id(&clib::NetworkId::testnet());
    } else {
        txbody.set_network_id(&clib::NetworkId::mainnet());
    }

    let mut txwitness = clib::TransactionWitnessSet::new();
    txwitness.set_plutus_scripts(&sc_scripts);
    txwitness.set_plutus_data(&decoded_datum.5.clone());
    txwitness.set_redeemers(&redeemers);

    info!("--------------------Iteration Ended------------------------------");
    info!("Vkey Counter at End: {:?}", vkey_counter);
    Ok((txbody, txwitness, aux_data, saved_input_txuos, vkey_counter))
}

pub async fn build_mp_cancel(
    gtxd: &super::TxData,
    mptxd: &super::MpTxData,
    sc_script: &String,
    sc_addr: &str,
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
    let (txbody_, mut txwitness_, aux_data_, _, vkey_counter) = perform_cancel(
        &cutils::to_bignum(2000000),
        sc_script,
        sc_addr,
        gtxd,
        mptxd,
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
    let (txbody, txwitness, aux_data, used_utxos, vkey_counter_2) =
        perform_cancel(&calculated_fee, sc_script, sc_addr, gtxd, mptxd, false)?;

    let transaction2 = clib::Transaction::new(&txbody, &txwitness_, Some(aux_data.clone()));

    if vkey_counter_2 != vkey_counter
        || transaction2.to_bytes().len() != transaction_.to_bytes().len()
    {
        let dummy_vkeywitnesses = hfn::make_dummy_vkeywitnesses(vkey_counter_2);
        txwitness_.set_vkeys(&dummy_vkeywitnesses);

        let calculated_fee =
            hfn::calc_txfee(&transaction2, &a, &b, ex_unit_price, &steps, &mem, true);
        let (txbody, txwitness, aux_data, used_utxos, _) =
            perform_cancel(&calculated_fee, sc_script, sc_addr, gtxd, mptxd, false)?;
        info!("Fee: {:?}", calculated_fee);
        debug!("\n\n\nDummy vkey_counter: {:?} \n\n", vkey_counter);
        debug!("\n\n\nDummy vkey_counter_2: {:?} \n\n", vkey_counter_2);
        Ok(hfn::tx_output_data(
            txbody,
            txwitness,
            Some(aux_data),
            used_utxos.to_hex()?,
            0u64,
            false,
        )?)
    } else {
        info!("Fee: {:?}", calculated_fee);
        Ok(hfn::tx_output_data(
            txbody,
            txwitness,
            Some(aux_data),
            used_utxos.to_hex()?,
            0u64,
            false,
        )?)
    }
}
