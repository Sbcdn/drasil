/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
mod error;
mod models;

use error::UOError;
use murin::{
    calc_min_ada_for_utxo, calc_txfee, find_token_utxos_na, tokens_to_value, value_to_tokens,
};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, UOError>;

pub async fn optimize(addr: &String, uid: i64, cid: i64) -> Result<()> {
    let mut dbsconn = mimir::establish_connection()?;
    let contract_utxos = mimir::get_address_utxos(&mut dbsconn, addr)?;

    let ada_utxos = contract_utxos.get_coin_only();
    let mut t_utxos = contract_utxos.get_token_only();
    let ada_on_token_utxos = t_utxos.coin_sum();
    let tokens = t_utxos.sum_avail_tokens();
    let mut tokens_on_contract = murin::Tokens::new();

    let twl = gungnir::TokenWhitelist::get_rwd_contract_tokens(cid, uid)?;
    let contract = hugin::TBContracts::get_contract_uid_cid(uid, cid)?;
    let ns = &murin::clib::NativeScript::from_bytes(hex::decode(contract.plutus.clone())?)?;
    let addr = murin::b_decode_addr_na(&contract.address)?;
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
    let liquidity = murin::clib::utils::from_bignum(&contract.get_contract_liquidity());
    let difference = liquidity as i64 - ada_on_token_utxos as i64;
    let transactions = match difference <= 0 {
        true => reallocate_tokens(&mut t_utxos, &tokens_on_contract, &addr, ns, liquidity)?,
        false => {
            let additional_utxos =
                ada_utxos.coin_value_subset(murin::clib::utils::to_bignum(difference as u64), None);
            t_utxos.merge(additional_utxos);
            reallocate_tokens(&mut t_utxos, &tokens_on_contract, &addr, ns, liquidity)?
        }
    };
    let mut txhs = Vec::<String>::new();
    println!("\nTransactions: \n");
    for tx in transactions {
        println!("Tx: {}", tx.0.to_hex());
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
    println!("\n TxHashes: {:?}", txhs);

    Ok(())
}

async fn submit_tx(
    transaction: murin::clib::Transaction,
    used_utxos: murin::TransactionUnspentOutputs,
    uid: i64,
    cid: i64,
    //version: f32,
) -> Result<String> {
    let bld_tx = murin::hfn::tx_output_data(
        transaction.body(),
        transaction.witness_set(),
        murin::clib::metadata::AuxiliaryData::new(),
        used_utxos.to_hex()?,
        0u64,
        false,
    )?;

    let raw_tx = murin::utxomngr::RawTx::new(
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

    let resp = hugin::create_response(&bld_tx, &raw_tx, None)?;

    let mut client = hugin::client::connect(std::env::var("ODIN_URL").unwrap())
        .await
        .unwrap();
    let cmd = hugin::FinalizeMultiSig::new(
        uid as u64,
        hugin::MultiSigType::UTxOpti,
        resp.get_id(),
        String::new(),
    );
    match client.build_cmd(cmd).await {
        Ok(o) => Ok(o),
        Err(e) => Err(UOError::OdinError(e.to_string())),
    }
}

fn reallocate_tokens(
    t_utxos: &mut murin::TransactionUnspentOutputs,
    tokens: &murin::Tokens,
    addr: &murin::clib::address::Address,
    script: &murin::clib::NativeScript,
    liquidity: u64,
) -> Result<Vec<(murin::clib::Transaction, murin::TransactionUnspentOutputs)>> {
    let mut out = Vec::<(murin::clib::Transaction, murin::TransactionUnspentOutputs)>::new();
    //let ada = t_utxos.coin_sum();
    let (std_value, minutxo, utxo_count) = get_values_and_tamt_per_utxo(tokens, liquidity);
    println!("\n\nTUTXO: BEFORE FILTER: \n{:?}\n\n", t_utxos);
    let set = t_utxos.filter_values(&std_value, Some(20))?;
    let utxo_count = (utxo_count as usize - set.len()) as u64;
    t_utxos.delete_set(&set);
    println!("\n\nTUTXO: After FILTER: \n{:?}\n\n", t_utxos);

    println!("Std Value: {:?}", std_value);
    println!("Min UTxO value: {:?}", minutxo);
    println!("UTxO Count: {:?}", utxo_count);

    // ToDo: Build recursive transactions
    txbuilder(t_utxos, &std_value, utxo_count, &mut out, addr, script)?;
    Ok(out)
}

fn get_values_and_tamt_per_utxo(
    tokens: &murin::Tokens,
    ada: u64,
) -> (murin::clib::utils::Value, murin::clib::utils::BigNum, u64) {
    //let max_token =
    let max_token = tokens
        .iter()
        .find(|n| {
            murin::clib::utils::from_bignum(&n.2)
                == tokens
                    .iter()
                    .map(|n| murin::clib::utils::from_bignum(&n.2))
                    .max()
                    .unwrap()
        })
        .unwrap();
    let mut v = murin::clib::utils::Value::new(&murin::clib::utils::to_bignum(1000000));
    let mut ma = murin::clib::MultiAsset::new();
    for t in tokens {
        let mut assets = murin::clib::Assets::new();
        assets.insert(&t.1, &max_token.2);
        ma.insert(&t.0, &assets);
    }
    v.set_multiasset(&ma);
    let minutxo = calc_min_ada_for_utxo(&v, None);
    let utxo_count = ada / murin::clib::utils::from_bignum(&minutxo);
    let mut tokens = tokens.clone();
    tokens.iter_mut().for_each(|n| {
        n.2 = murin::clib::utils::to_bignum(murin::clib::utils::from_bignum(&n.2) / utxo_count);
    });
    let mut std_value = tokens_to_value(&tokens);
    std_value.set_coin(&minutxo);
    (std_value, minutxo, utxo_count)
}

/*
struct TxBuilderOut {
    utxos: murin::TransactionUnspentOutputs,
    std_value: murin::clib::utils::Value,
    utxo_amt: u64,
    transactions: Vec<(murin::clib::Transaction, murin::TransactionUnspentOutputs)>,
}
*/
fn txbuilder(
    utxos: &mut murin::TransactionUnspentOutputs,
    std_value: &murin::clib::utils::Value,
    utxo_amt: u64,
    transactions: &mut Vec<(murin::clib::Transaction, murin::TransactionUnspentOutputs)>,
    addr: &murin::clib::address::Address,
    script: &murin::clib::NativeScript,
) -> Result<()> {
    let std_tokens = value_to_tokens(std_value)?;
    let r = find_token_utxos_na(utxos, std_tokens, None);

    match (utxos.len(), utxo_amt, r) {
        (_, 0, _) | (0, _, _) | (_, _, Err(_)) => {
            println!("Stop transaction building");
            Ok(())
        }
        _ => {
            println!("Continue building transactions");
            let (tx, mut used_utxos, new_utxo_amt) =
                make_new_tx(utxos, std_value, &utxo_amt, addr, script)?;
            //.expect("Could not create transaction");
            utxos.delete_set(&used_utxos);
            transactions.push((tx, used_utxos.clone()));
            Ok(txbuilder(
                &mut used_utxos,
                std_value,
                new_utxo_amt,
                transactions,
                addr,
                script,
            )?)
        }
    }
}

fn make_new_tx(
    utxos: &mut murin::TransactionUnspentOutputs,
    std_value: &murin::clib::utils::Value,
    utxo_amt: &u64,
    addr: &murin::clib::address::Address,
    script: &murin::clib::NativeScript,
) -> Result<(
    murin::clib::Transaction,
    murin::TransactionUnspentOutputs,
    u64,
)> {
    let inputs = murin::clib::TransactionInputs::new();
    let outputs = murin::clib::TransactionOutputs::new();

    let txb = murin::clib::TransactionBody::new_tx_body(
        &inputs,
        &outputs,
        &murin::clib::utils::to_bignum(2000000u64),
    );
    let txw = murin::clib::TransactionWitnessSet::new();
    let mut tx = murin::clib::Transaction::new(&txb, &txw, None);
    let mut used_input_utxos = murin::TransactionUnspentOutputs::new();
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
    transaction: &mut murin::clib::Transaction,
    utxos: &mut murin::TransactionUnspentOutputs,
    used_input_utxos: &mut murin::TransactionUnspentOutputs,
    utxo_amt: &u64,
    std_value: &murin::clib::utils::Value,
    addr: &murin::clib::address::Address,
    script: &murin::clib::NativeScript,
) -> Result<(
    murin::clib::Transaction,
    murin::TransactionUnspentOutputs,
    murin::TransactionUnspentOutputs,
    u64,
    murin::clib::utils::Value,
    murin::clib::address::Address,
)> {
    let mut needed_value = std_value.clone();
    needed_value.set_coin(
        &needed_value
            .coin()
            .checked_add(&transaction.body().fee().clone())
            .unwrap(),
    );
    let security = murin::clib::utils::to_bignum(
        murin::clib::utils::from_bignum(&needed_value.coin()) + (2 * murin::htypes::MIN_ADA),
    );
    needed_value.set_coin(&needed_value.coin().checked_add(&security).unwrap());

    let (txins, input_txuos) =
        murin::txbuilders::input_selection(None, &mut needed_value, utxos, None, None)?;
    let txb = transaction.body();
    let mut inputs = txb.inputs();
    let mut outputs = txb.outputs();
    for i in 0..txins.len() {
        inputs.add(&txins.get(i));
    }
    used_input_utxos.merge(input_txuos);
    utxos.delete_set(used_input_utxos);
    outputs.add(&murin::clib::TransactionOutput::new(addr, std_value));

    let mut out_value = murin::hfn::sum_output_values(&outputs);
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
            .compare(&murin::clib::utils::to_bignum(4000000))
            >= 0
    {
        outputs.add(&murin::clib::TransactionOutput::new(addr, std_value));
        out_value = murin::hfn::sum_output_values(&outputs);
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
        println!("Exit add_outputs on if");
        Ok((
            tmp_tx,
            utxos.to_owned(),
            used_input_utxos.to_owned(),
            utxo_amt.to_owned(),
            std_value.to_owned(),
            addr.to_owned(),
        ))
    } else {
        println!("Add more utxos");
        let txb = murin::clib::TransactionBody::new_tx_body(&inputs, &outputs, &fee);
        let mut transaction = murin::clib::Transaction::new(&txb, &transaction.witness_set(), None);
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
    inputs: &murin::clib::TransactionInputs,
    outputs: &murin::clib::TransactionOutputs,
    addr: &murin::clib::address::Address,
    change: &murin::clib::utils::Value,
    fee: &murin::clib::utils::BigNum,
    script: &murin::clib::NativeScript,
) -> Result<(murin::clib::Transaction, murin::clib::utils::BigNum)> {
    let mem = murin::clib::utils::to_bignum(7000000u64); //cutils::to_bignum(7000000u64);
    let steps = murin::clib::utils::to_bignum(2500000000u64); //cutils::to_bignum(3000000000u64);
    let ex_unit_price: murin::htypes::ExUnitPrice = murin::ExUnitPrice {
        priceSteps: 7.21e-5,
        priceMemory: 5.77e-2,
    };
    let a = murin::clib::utils::to_bignum(44u64);
    let b = murin::clib::utils::to_bignum(155381u64);

    let slot = mimir::get_slot(&mut mimir::establish_connection()?)? as u64 + 3600;
    let network = match addr.network_id()? {
        1 => murin::clib::NetworkId::mainnet(),
        _ => murin::clib::NetworkId::testnet(),
    };
    let change = change.checked_add(&murin::clib::utils::Value::new(
        &murin::clib::utils::to_bignum(64),
    ))?;
    let mut tmp_outputs = outputs.clone();
    tmp_outputs.add(&murin::clib::TransactionOutput::new(addr, &change));

    let mut txw = murin::clib::TransactionWitnessSet::new();
    let mut native_scripts = murin::clib::NativeScripts::new();
    native_scripts.add(script);
    txw.set_native_scripts(&native_scripts);
    let vkeys = murin::make_dummy_vkeywitnesses(2);
    txw.set_vkeys(&vkeys);

    let mut tmp_txb = murin::clib::TransactionBody::new_tx_body(inputs, &tmp_outputs, fee);
    tmp_txb.set_ttl(&murin::clib::utils::to_bignum(slot));
    tmp_txb.set_network_id(&network);
    let tmp_transaction = murin::clib::Transaction::new(&tmp_txb, &txw, None);
    let fee = calc_txfee(&tmp_transaction, &a, &b, ex_unit_price, &steps, &mem, true);
    //fee.checked_add(&murin::clib::utils::to_bignum(64))?;
    let mut outputs = outputs.clone();
    outputs.add(&murin::clib::TransactionOutput::new(
        addr,
        &change.checked_sub(&murin::clib::utils::Value::new(&fee))?,
    ));

    let mut txb = murin::clib::TransactionBody::new_tx_body(inputs, &outputs, &fee);
    txb.set_ttl(&murin::clib::utils::to_bignum(slot));
    txb.set_network_id(&network);

    let mut txw = murin::clib::TransactionWitnessSet::new();
    let mut native_scripts = murin::clib::NativeScripts::new();
    native_scripts.add(script);
    txw.set_native_scripts(&native_scripts);

    Ok((murin::clib::Transaction::new(&txb, &txw, None), fee))
}
