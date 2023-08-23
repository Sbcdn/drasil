mod error;
mod models;

use drasil_murin::{
    calc_min_ada_for_utxo, calc_txfee, find_token_utxos_na, tokens_to_value, value_to_tokens,
};
use error::UOError;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, UOError>;

pub async fn optimize(addr: String, uid: i64, cid: i64) -> Result<()> {
    log::debug!("Try to connect to dbsync...");
    let contract_utxos = drasil_mimir::get_address_utxos(&addr)?;
    log::debug!("Calculate thresholds...");
    let ada_utxos = contract_utxos.get_coin_only();
    let mut t_utxos = contract_utxos.get_token_only();
    let ada_on_token_utxos = t_utxos.coin_sum();
    let tokens = t_utxos.sum_avail_tokens();
    let mut tokens_on_contract = drasil_murin::Tokens::new();
    log::debug!("Get token whitelistsings...");
    let twl = drasil_gungnir::TokenWhitelist::get_rwd_contract_tokens(cid, uid)?;
    log::debug!("Get contracts...");
    let contract = drasil_hugin::TBContracts::get_contract_uid_cid(uid, cid)?;
    log::debug!("Decode native script...");
    let ns = &drasil_murin::clib::NativeScript::from_bytes(hex::decode(contract.plutus.clone())?)?;
    log::debug!("Decode address...");
    let addr = drasil_murin::b_decode_addr_na(&contract.address)?;
    log::debug!("Get tokens on contract...");
    for t in tokens {
        let tmp = twl.iter().find(|n| {
            hex::encode(t.0.to_bytes()) == n.policy_id
                && hex::encode(t.1.name()) == *n.tokenname.as_ref().unwrap()
        });
        if tmp.is_some() {
            tokens_on_contract.push(t)
        }
    }
    // ToDo: Check if conditions for reallocation are met or to return without working
    log::debug!("Try to get contract liquidity...");
    let liquidity = drasil_murin::clib::utils::from_bignum(&contract.get_contract_liquidity());
    let difference = liquidity as i64 - ada_on_token_utxos as i64;

    log::debug!("Try to reallocate tokens...");
    let transactions = match difference <= 0 {
        true => reallocate_tokens(&mut t_utxos, &tokens_on_contract, &addr, ns, liquidity)?,
        false => {
            let additional_utxos = ada_utxos.coin_value_subset(
                drasil_murin::clib::utils::to_bignum(difference as u64),
                None,
            );
            t_utxos.merge(additional_utxos);
            reallocate_tokens(&mut t_utxos, &tokens_on_contract, &addr, ns, liquidity)?
        }
    };
    let mut txhs = Vec::<String>::new();
    log::debug!("\nTransactions: \n");
    for tx in transactions {
        log::debug!("Tx: for {:?}\n{}", &contract.address, tx.0.to_hex());
        let txh = submit_tx(
            tx.0,
            tx.1,
            contract.user_id,
            contract.contract_id,
            //contract.version,
        )
        .await?;
        txhs.push(txh);
    }
    log::debug!("\n TxHashes for: {:?}, {:?}", &contract.address, txhs);

    Ok(())
}

async fn submit_tx(
    transaction: drasil_murin::clib::Transaction,
    used_utxos: drasil_murin::TransactionUnspentOutputs,
    uid: i64,
    cid: i64,
    //version: f32,
) -> Result<String> {
    let bld_tx = drasil_murin::hfn::tx_output_data(
        transaction.body(),
        transaction.witness_set(),
        None,
        used_utxos.to_hex()?,
        0u64,
        false,
    )?;

    let raw_tx = drasil_murin::utxomngr::RawTx::new(
        &bld_tx.get_tx_body(),
        &bld_tx.get_txwitness(),
        &bld_tx.get_tx_unsigned(),
        &bld_tx.get_metadata(),
        &"".to_string(),
        &"utxoopti".to_string(),
        &bld_tx.get_used_utxos(),
        &"".to_string(),
        &uid,
        &[cid],
    );

    let resp = drasil_hugin::create_response(&bld_tx, &raw_tx, None)?;

    let mut client = drasil_hugin::client::connect(std::env::var("ODIN_URL").unwrap())
        .await
        .unwrap();
    let cmd = drasil_hugin::FinalizeMultiSig::new(
        uid as u64,
        drasil_hugin::MultiSigType::UTxOpti,
        resp.get_id(),
        String::new(),
    );
    match client.build_cmd(cmd).await {
        Ok(o) => Ok(o),
        Err(e) => Err(UOError::OdinError(e.to_string())),
    }
}

fn reallocate_tokens(
    t_utxos: &mut drasil_murin::TransactionUnspentOutputs,
    tokens: &drasil_murin::Tokens,
    addr: &drasil_murin::clib::address::Address,
    script: &drasil_murin::clib::NativeScript,
    liquidity: u64,
) -> Result<
    Vec<(
        drasil_murin::clib::Transaction,
        drasil_murin::TransactionUnspentOutputs,
    )>,
> {
    let mut out = Vec::<(
        drasil_murin::clib::Transaction,
        drasil_murin::TransactionUnspentOutputs,
    )>::new();
    //let ada = t_utxos.coin_sum();
    let (std_value, minutxo, utxo_count) = get_values_and_tamt_per_utxo(tokens, liquidity);
    log::trace!("\n\nTUTXO: BEFORE FILTER: \n{:?}\n\n", t_utxos);
    let set = t_utxos.filter_values(&std_value, Some(20))?;
    let utxo_count = (utxo_count as usize - set.len()) as u64;
    t_utxos.delete_set(&set);
    log::trace!("\n\nTUTXO: After FILTER: \n{:?}\n\n", t_utxos);

    log::debug!("Std Value: {:?}", std_value);
    log::debug!("Min UTxO value: {:?}", minutxo);
    log::debug!("UTxO Count: {:?}", utxo_count);
    log::debug!("Contract Address: {:?}", addr.to_bech32(None));

    // ToDo: Build recursive transactions
    txbuilder(t_utxos, &std_value, utxo_count, &mut out, addr, script)?;
    Ok(out)
}

fn get_values_and_tamt_per_utxo(
    tokens: &drasil_murin::Tokens,
    ada: u64,
) -> (
    drasil_murin::clib::utils::Value,
    drasil_murin::clib::utils::BigNum,
    u64,
) {
    //let max_token =
    let max_token = tokens
        .iter()
        .find(|n| {
            drasil_murin::clib::utils::from_bignum(&n.2)
                == tokens
                    .iter()
                    .map(|n| drasil_murin::clib::utils::from_bignum(&n.2))
                    .max()
                    .unwrap()
        })
        .unwrap();
    let mut v =
        drasil_murin::clib::utils::Value::new(&drasil_murin::clib::utils::to_bignum(1000000));
    let mut ma = drasil_murin::clib::MultiAsset::new();
    for t in tokens {
        let mut assets = drasil_murin::clib::Assets::new();
        assets.insert(&t.1, &max_token.2);
        ma.insert(&t.0, &assets);
    }
    v.set_multiasset(&ma);
    let minutxo = calc_min_ada_for_utxo(&v, None);
    let utxo_count = ada / drasil_murin::clib::utils::from_bignum(&minutxo);
    let mut tokens = tokens.clone();
    tokens.iter_mut().for_each(|n| {
        n.2 = drasil_murin::clib::utils::to_bignum(
            drasil_murin::clib::utils::from_bignum(&n.2) / utxo_count,
        );
    });
    let mut std_value = tokens_to_value(&tokens);
    std_value.set_coin(&minutxo);
    (std_value, minutxo, utxo_count)
}

/*
struct TxBuilderOut {
    utxos: drasil_murin::TransactionUnspentOutputs,
    std_value: drasil_murin::clib::utils::Value,
    utxo_amt: u64,
    transactions: Vec<(drasil_murin::clib::Transaction, drasil_murin::TransactionUnspentOutputs)>,
}
*/
fn txbuilder(
    utxos: &mut drasil_murin::TransactionUnspentOutputs,
    std_value: &drasil_murin::clib::utils::Value,
    utxo_amt: u64,
    transactions: &mut Vec<(
        drasil_murin::clib::Transaction,
        drasil_murin::TransactionUnspentOutputs,
    )>,
    addr: &drasil_murin::clib::address::Address,
    script: &drasil_murin::clib::NativeScript,
) -> Result<()> {
    let std_tokens = value_to_tokens(std_value)?;
    let r = find_token_utxos_na(utxos, std_tokens, None);

    match (utxos.len(), utxo_amt, r) {
        (_, 0, _) | (0, _, _) | (_, _, Err(_)) => {
            log::debug!("Stop transaction building");
            Ok(())
        }
        _ => {
            log::debug!("Continue building transactions");
            let (tx, used_utxos, _new_utxo_amt) =
                make_new_tx(utxos, std_value, &utxo_amt, addr, script)?;
            //.expect("Could not create transaction");
            utxos.delete_set(&used_utxos);
            transactions.push((tx, used_utxos.clone()));
            Ok(
                (), /*
                        txbuilder(
                        &mut used_utxos,
                        std_value,
                        new_utxo_amt,
                        transactions,
                        addr,
                        script,
                    )? */
            )
        }
    }
}

fn make_new_tx(
    utxos: &mut drasil_murin::TransactionUnspentOutputs,
    std_value: &drasil_murin::clib::utils::Value,
    utxo_amt: &u64,
    addr: &drasil_murin::clib::address::Address,
    script: &drasil_murin::clib::NativeScript,
) -> Result<(
    drasil_murin::clib::Transaction,
    drasil_murin::TransactionUnspentOutputs,
    u64,
)> {
    let inputs = drasil_murin::clib::TransactionInputs::new();
    let outputs = drasil_murin::clib::TransactionOutputs::new();

    let txb = drasil_murin::clib::TransactionBody::new_tx_body(
        &inputs,
        &outputs,
        &drasil_murin::clib::utils::to_bignum(2000000u64),
    );
    let txw = drasil_murin::clib::TransactionWitnessSet::new();
    let mut tx = drasil_murin::clib::Transaction::new(&txb, &txw, None);
    let mut used_input_utxos = drasil_murin::TransactionUnspentOutputs::new();
    let new_tx = add_utxos(
        &mut tx,
        utxos,
        &mut used_input_utxos,
        utxo_amt,
        std_value,
        addr,
        script,
    )?;

    Ok((new_tx.0, new_tx.1, new_tx.3))
}

fn add_utxos(
    transaction: &mut drasil_murin::clib::Transaction,
    utxos: &mut drasil_murin::TransactionUnspentOutputs,
    used_input_utxos: &mut drasil_murin::TransactionUnspentOutputs,
    utxo_amt: &u64,
    std_value: &drasil_murin::clib::utils::Value,
    addr: &drasil_murin::clib::address::Address,
    script: &drasil_murin::clib::NativeScript,
) -> Result<(
    drasil_murin::clib::Transaction,
    drasil_murin::TransactionUnspentOutputs,
    drasil_murin::TransactionUnspentOutputs,
    u64,
    drasil_murin::clib::utils::Value,
    drasil_murin::clib::address::Address,
)> {
    let mut needed_value = std_value.clone();
    needed_value.set_coin(
        &needed_value
            .coin()
            .checked_add(&transaction.body().fee().clone())
            .unwrap(),
    );
    let security = drasil_murin::clib::utils::to_bignum(
        drasil_murin::clib::utils::from_bignum(&needed_value.coin())
            + (utxo_amt / 2 * drasil_murin::htypes::MIN_ADA),
    );
    needed_value.set_coin(&needed_value.coin().checked_add(&security).unwrap());

    let (txins, input_txuos) =
        drasil_murin::txbuilders::input_selection(None, &mut needed_value, utxos, None, None)?;
    let txb = transaction.body();
    let mut inputs = txb.inputs();
    let mut outputs = txb.outputs();
    for i in 0..txins.len() {
        inputs.add(&txins.get(i));
    }
    used_input_utxos.merge(input_txuos);
    utxos.delete_set(used_input_utxos);
    outputs.add(&drasil_murin::clib::TransactionOutput::new(addr, std_value));

    let mut out_value = drasil_murin::hfn::sum_output_values(&outputs);
    let in_value = used_input_utxos.calc_total_value()?;
    let mut change = in_value.checked_sub(&out_value)?;

    while change.compare(std_value).unwrap_or(-1) >= 0
        && out_value
            .checked_add(std_value)?
            .compare(&in_value)
            .unwrap_or(1)
            < 0
        && change
            .coin()
            .compare(&drasil_murin::clib::utils::to_bignum(4000000))
            >= 0
    {
        outputs.add(&drasil_murin::clib::TransactionOutput::new(addr, std_value));
        out_value = drasil_murin::hfn::sum_output_values(&outputs);
        change = in_value.checked_sub(&out_value)?;
        let (stmptx, _) = finalize_tx(
            &inputs,
            &outputs,
            addr,
            &change,
            &transaction.body().fee(),
            script,
        )?;
        if stmptx.to_bytes().len() > 15000 {
            break;
        }
    }

    let (tmp_tx, fee) = finalize_tx(
        &inputs,
        &outputs,
        addr,
        &change,
        &transaction.body().fee(),
        script,
    )?;
    let std_tokens = value_to_tokens(std_value)?;
    let r = find_token_utxos_na(utxos, std_tokens, None);
    if tmp_tx.to_bytes().len() > 15000
        || utxos.is_empty()
        || (utxo_amt - outputs.len() as u64) == 0
        || r.is_err()
    {
        log::debug!("Exit add_outputs on if");
        Ok((
            tmp_tx,
            utxos.to_owned(),
            used_input_utxos.to_owned(),
            utxo_amt.to_owned(),
            std_value.to_owned(),
            addr.to_owned(),
        ))
    } else {
        log::debug!("Add more utxos");
        let txb = drasil_murin::clib::TransactionBody::new_tx_body(&inputs, &outputs, &fee);
        let mut transaction =
            drasil_murin::clib::Transaction::new(&txb, &transaction.witness_set(), None);
        add_utxos(
            &mut transaction,
            utxos,
            used_input_utxos,
            utxo_amt,
            std_value,
            addr,
            script,
        )
    }
}

fn finalize_tx(
    inputs: &drasil_murin::clib::TransactionInputs,
    outputs: &drasil_murin::clib::TransactionOutputs,
    addr: &drasil_murin::clib::address::Address,
    change: &drasil_murin::clib::utils::Value,
    fee: &drasil_murin::clib::utils::BigNum,
    script: &drasil_murin::clib::NativeScript,
) -> Result<(
    drasil_murin::clib::Transaction,
    drasil_murin::clib::utils::BigNum,
)> {
    let mem = drasil_murin::clib::utils::to_bignum(7000000u64); //cutils::to_bignum(7000000u64);
    let steps = drasil_murin::clib::utils::to_bignum(2500000000u64); //cutils::to_bignum(3000000000u64);
    let ex_unit_price: drasil_murin::htypes::ExUnitPrice = drasil_murin::ExUnitPrice {
        priceSteps: 7.21e-5,
        priceMemory: 5.77e-2,
    };
    let a = drasil_murin::clib::utils::to_bignum(44u64);
    let b = drasil_murin::clib::utils::to_bignum(155381u64);

    let slot = drasil_mimir::get_slot(&mut drasil_mimir::establish_connection()?)? as u64 + 3600;
    let network = match addr.network_id()? {
        1 => drasil_murin::clib::NetworkId::mainnet(),
        _ => drasil_murin::clib::NetworkId::testnet(),
    };
    let change = change.checked_add(&drasil_murin::clib::utils::Value::new(
        &drasil_murin::clib::utils::to_bignum(64),
    ))?;
    let mut tmp_outputs = outputs.clone();
    tmp_outputs.add(&drasil_murin::clib::TransactionOutput::new(addr, &change));

    let mut txw = drasil_murin::clib::TransactionWitnessSet::new();
    let mut native_scripts = drasil_murin::clib::NativeScripts::new();
    native_scripts.add(script);
    txw.set_native_scripts(&native_scripts);
    let vkeys = drasil_murin::make_dummy_vkeywitnesses(2);
    txw.set_vkeys(&vkeys);

    let mut tmp_txb = drasil_murin::clib::TransactionBody::new_tx_body(inputs, &tmp_outputs, fee);
    tmp_txb.set_ttl(&drasil_murin::clib::utils::to_bignum(slot));
    tmp_txb.set_network_id(&network);
    let tmp_transaction = drasil_murin::clib::Transaction::new(&tmp_txb, &txw, None);
    let fee = calc_txfee(&tmp_transaction, &a, &b, ex_unit_price, &steps, &mem, true);
    //fee.checked_add(&drasil_murin::clib::utils::to_bignum(64))?;
    let mut outputs = outputs.clone();
    outputs.add(&drasil_murin::clib::TransactionOutput::new(
        addr,
        &change.checked_sub(&drasil_murin::clib::utils::Value::new(&fee))?,
    ));

    let mut txb = drasil_murin::clib::TransactionBody::new_tx_body(inputs, &outputs, &fee);
    txb.set_ttl(&drasil_murin::clib::utils::to_bignum(slot));
    txb.set_network_id(&network);

    let mut txw = drasil_murin::clib::TransactionWitnessSet::new();
    let mut native_scripts = drasil_murin::clib::NativeScripts::new();
    native_scripts.add(script);
    txw.set_native_scripts(&native_scripts);

    Ok((drasil_murin::clib::Transaction::new(&txb, &txw, None), fee))
}
