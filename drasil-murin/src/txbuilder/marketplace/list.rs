use crate::error::MurinError;
use crate::supporting_functions;
use crate::models;
use crate::marketplace::*;
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr, utils as cutils};

pub fn perform_listing(
    fee: &cutils::BigNum,
    sc_addr: &str,
    sc_version: &String,
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

    let mut roy_rate: u64 = 0;
    if let Some(royrate) = mptxd.clone().get_royalties_rate() {
        roy_rate = (royrate * 1000.0) as u64;
    }

    let unpaid_royalties: u64 = mptxd.clone().get_price() / 1000 * roy_rate;
    info!("Royalties: {:?}", unpaid_royalties);
    if roy_rate > 500 {
        //panic!("Royalty Rate is formated wrong (0.015 = 1.5%); or Royalty Rate is larger 50%, artifct does not accept that");

        // ToDo Check roy rate via DbSync
        roy_rate = 500;
    }

    let sc_address = caddr::Address::from_bech32(sc_addr).unwrap();
    let mut roy_pkey = supporting_functions::get_payment_address(&sc_address);
    let roy_addr: caddr::Address;
    if let Some(royaddr) = mptxd.clone().get_royalties_address() {
        if roy_rate > 0u64 {
            roy_addr = royaddr;
            roy_pkey = supporting_functions::get_payment_address(&roy_addr);
        }
    }

    // ToDo:
    let datumpair = supporting_functions::make_datum_mp(
        &mptxd.selling_price.to_string(),
        &gtxd.clone().get_senders_addresses()[0]
            .clone()
            .to_bech32(None)
            .unwrap(),
        &roy_rate.to_string(),
        &hex::encode(&mptxd.get_tokens()[0].0.to_bytes()),
        &hex::encode(&mptxd.get_tokens()[0].1.to_bytes()),
        &roy_pkey,
        sc_version,
    );

    /////////////////////////////////////////////////////////////////////////////////////////////////////
    //
    //Auxiliary Data
    //  Plutus Script and Metadata
    /////////////////////////////////////////////////////////////////////////////////////////////////////
    //let aux_data_set = clib::AuxiliaryDataSet::new();
    let mut aux_data = clib::metadata::AuxiliaryData::new();
    //aux_data.set_plutus_scripts(&sc_scripts);
    let mut general_metadata = clib::metadata::GeneralTransactionMetadata::new();
    let datum = datumpair.2; //.get(0).as_constr_plutus_data().unwrap().data();
    debug!("Datum Metadata: {:?}\n", datum);

    let mut metalist = clib::metadata::MetadataList::new();
    for dat in 0..datum.len() {
        debug!("Datum Metadata Iterator: {:?}", datum.get(dat));
        metalist.add(
            &clib::metadata::TransactionMetadatum::new_bytes(datum.get(dat).unwrap().to_vec())
                .unwrap(),
        );
    }
    let metadata = clib::metadata::TransactionMetadatum::new_list(&metalist);
    general_metadata.insert(&cutils::to_bignum(312u64), &metadata);
    aux_data.set_metadata(&general_metadata);
    let aux_data_hash = cutils::hash_auxiliary_data(&aux_data);

    //info!("From auxData: {:?}",aux_data.plutus_scripts());

    //////////////////////////////////////////////////////////////////////////////////////////////////////
    //Add Inputs and Outputs
    //
    //
    ///////////////////////////////////////////////////////////////////////////////////////////////////////
    let trade_owner = &gtxd.clone().get_senders_addresses()[0];
    let mut txouts = clib::TransactionOutputs::new();

    let smart_contract_address = caddr::Address::from_bech32(sc_addr).unwrap();

    let _script_utxo = make_mp_contract_utxo_output(
        &mut txouts,
        smart_contract_address,
        &datumpair.0,
        &mptxd.tokens,
        true,
    );
    //let mut wallet_utxos = make_wallet_outputs_txb (&mut txouts, &txd.outputs,&!dummy,fee);

    // Need to be changes in the way to extract the TxIn out of the TxUOs
    //let (_, input_txuos) =
    //artifn::make_inputs_txb (&gtxd.get_inputs());

    let mut input_txuos = gtxd.clone().get_inputs();

    // Check if some utxos in inputs are in use and remove them
    if let Some(used_utxos) = crate::utxomngr::usedutxos::check_any_utxo_used(&input_txuos)? {
        input_txuos.remove_used_utxos(used_utxos);
    }

    let collateral_input_txuo = gtxd.clone().get_collateral();

    //cutils::TransactionUnspentOutput::from_bytes(hex::decode(&txd.collateral_input).unwrap()).unwrap();

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

    let mut needed_value = supporting_functions::sum_output_values(&txouts);
    needed_value.set_coin(&needed_value.coin().checked_add(&fee.clone()).unwrap());
    let security = cutils::to_bignum(
        cutils::from_bignum(&needed_value.coin()) / 100 * 10 + (2 * models::MIN_ADA),
    ); // 10% Security for min utxo etc.
    needed_value.set_coin(&needed_value.coin().checked_add(&security).unwrap());

    debug!("Needed Value: {:?}", needed_value);
    debug!(
        "\n\n\n\n\nTxIns Before selection:\n {:?}\n\n\n\n\n",
        input_txuos
    );

    //let mut token_utxos = artitypes::TransactionUnspentOutputs::new();
    //token_utxos.add(&script_utxo);
    //let listing_tokens  = artifn::get_nfts_for_sale(&token_utxos);

    let token_input_utxo = supporting_functions::find_asset_utxos_in_txuos(&input_txuos, mptxd.get_tokens());
    debug!("Token Input Utxos: {:?}", token_input_utxo);

    let (txins, mut input_txuos) = supporting_functions::input_selection(
        None,
        &mut needed_value,
        &input_txuos,
        None, //gtxd.clone().get_collateral()
    );
    let saved_input_txuos = input_txuos.clone();

    let vkey_counter = supporting_functions::get_vkey_count(&input_txuos, collateral_input_txuo.as_ref());
    debug!(
        "\n\n\n\n\nTxIns Before Balance:\n {:?}\n\n\n\n\n",
        input_txuos
    );

    let txouts_fin = supporting_functions::balance_tx(
        &mut input_txuos,
        mptxd.get_tokens(),
        &mut txouts,
        None,
        fee,
        &mut fee_paid,
        &mut first_run,
        &mut txos_paid,
        &mut tbb_values,
        trade_owner,
        change_address,
        &mut acc,
        None,
        &dummy,
    )?;

    let slot = gtxd.clone().get_current_slot() + supporting_functions::get_ttl_tx(&gtxd.clone().get_network());
    let mut txbody = clib::TransactionBody::new_tx_body(&txins, &txouts_fin, fee);
    txbody.set_ttl(&cutils::to_bignum(slot));
    info!("\nTxOutputs: {:?}\n", txbody.outputs());
    info!("\nTxInputs: {:?}\n", txbody.inputs());

    txbody.set_auxiliary_data_hash(&aux_data_hash);

    // Set network Id
    if gtxd.get_network() == clib::NetworkIdKind::Testnet {
        txbody.set_network_id(&clib::NetworkId::testnet());
    } else {
        txbody.set_network_id(&clib::NetworkId::mainnet());
    }

    let txwitness = clib::TransactionWitnessSet::new();

    info!("--------------------Iteration Ended------------------------------");
    info!("Vkey Counter at End: {:?}", vkey_counter);
    Ok((txbody, txwitness, aux_data, saved_input_txuos, vkey_counter))
}

pub async fn build_mp_listing(
    gtxd: &super::TxData,
    mptxd: &super::MpTxData,
    sc_addr: &str,
    sc_version: &String,
) -> Result<models::BuildOutput, MurinError> {
    // Temp until Protocol Parameters fixed
    let mem = cutils::to_bignum(7000000u64); //cutils::to_bignum(7000000u64);
    let steps = cutils::to_bignum(2500000000u64); //cutils::to_bignum(3000000000u64);
    let ex_unit_price: models::ExUnitPrice = crate::ExUnitPrice {
        priceSteps: 7.21e-5,
        priceMemory: 5.77e-2,
    };
    //serde_json::from_str("{'executionUnitPrices': {'priceSteps': 7.21e-5,'priceMemory': 5.77e-2}").unwrap();
    let a = cutils::to_bignum(44u64);
    let b = cutils::to_bignum(155381u64);

    //

    //Create Tx
    let (txbody_, mut txwitness_, aux_data_, _used_utxos, vkey_counter) = perform_listing(
        &cutils::to_bignum(2000000),
        sc_addr,
        sc_version,
        gtxd,
        mptxd,
        true,
    )?;

    let dummy_vkeywitnesses = supporting_functions::make_dummy_vkeywitnesses(vkey_counter);
    txwitness_.set_vkeys(&dummy_vkeywitnesses);

    // Build and encode dummy transaction
    let transaction_ = clib::Transaction::new(&txbody_, &txwitness_, Some(aux_data_));

    let calculated_fee = supporting_functions::calc_txfee(
        &transaction_,
        &a,
        &b,
        ex_unit_price.clone(),
        &steps,
        &mem,
        true,
    );
    let (txbody, txwitness, aux_data, used_utxos, vkey_counter_2) =
        perform_listing(&calculated_fee, sc_addr, sc_version, gtxd, mptxd, false)?;

    let transaction2 = clib::Transaction::new(&txbody, &txwitness_, Some(aux_data.clone()));

    if vkey_counter_2 != vkey_counter
        || transaction2.to_bytes().len() != transaction_.to_bytes().len()
    {
        let dummy_vkeywitnesses = supporting_functions::make_dummy_vkeywitnesses(vkey_counter_2);
        txwitness_.set_vkeys(&dummy_vkeywitnesses);

        let calculated_fee =
            supporting_functions::calc_txfee(&transaction2, &a, &b, ex_unit_price, &steps, &mem, true);
        let (txbody, txwitness, aux_data, used_utxos, _) =
            perform_listing(&calculated_fee, sc_addr, sc_version, gtxd, mptxd, false)?;
        info!("Fee: {:?}", calculated_fee);
        debug!("\n\n\nDummy vkey_counter: {:?} \n\n", vkey_counter);
        debug!("\n\n\nDummy vkey_counter_2: {:?} \n\n", vkey_counter_2);
        Ok(supporting_functions::tx_output_data(
            txbody,
            txwitness,
            Some(aux_data),
            used_utxos.to_hex()?,
            0u64,
            false,
        )?)
    } else {
        info!("Fee: {:?}", calculated_fee);
        Ok(supporting_functions::tx_output_data(
            txbody,
            txwitness,
            Some(aux_data),
            used_utxos.to_hex()?,
            0u64,
            false,
        )?)
    }
}
