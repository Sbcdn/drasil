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
use murin::{calc_min_ada_for_utxo, calc_txfee, tokens_to_value};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Reward Calculator",
    about = "Calculates rewards for the drasil - freeloaderz SmartClaimz system."
)]
struct Opt {
    #[structopt(short, long, about = "the epoch rewards should be calcualted for")]
    epoch: Option<i64>,

    #[structopt(
        short,
        long,
        about = "calc from the given epoch up to the latest possible one"
    )]
    from: Option<bool>,

    #[structopt(short, long, about = "the epoch rewards should be calcualted for")]
    t: Option<bool>,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, UOError>;

#[tokio::main]
pub async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let opt = Opt::from_args();

    let current_epoch = mimir::get_epoch(&mimir::establish_connection()?)? as i64;
    let calc_epoch = current_epoch - 2;
    println!("Current Epoch: {}", current_epoch);
    println!("Calculation Epoch: {}", calc_epoch);
    if opt.epoch.is_some() && opt.epoch.unwrap() > calc_epoch {
        return Err(gungnir::RWDError::new(
            "It is not possible to calculate rewards for the current or future epochs",
        )
        .into());
    }
    optimize(
        &"addr_test1wr84fwh5mt0usmwewfmzz5l0qyxrxa897eswwmrtcxz3mls9mwcxy".to_string(),
        0,
        1,
    )?;
    Ok(())
}

pub fn optimize(addr: &String, uid: i64, cid: i64) -> Result<()> {
    let dbsconn = mimir::establish_connection()?;
    let contract_utxos = mimir::get_address_utxos(&dbsconn, addr)?;
    let ada_utxos = contract_utxos.get_coin_only();
    let mut t_utxos = contract_utxos.get_token_only();
    let ada_on_token_utxos = t_utxos.coin_sum();
    let tokens = t_utxos.sum_avail_tokens();
    let mut tokens_on_contract = murin::Tokens::new();

    let twl = gungnir::TokenWhitelist::get_rwd_contract_tokens(cid, uid)?;
    let contract = hugin::TBContracts::get_contract_uid_cid(uid, cid)?;
    let addr = murin::b_decode_addr_na(&contract.address)?;
    for t in tokens {
        let tmp = twl.iter().find(|n| {
            hex::encode(t.0.to_bytes()) == n.policy_id
                && t.1.to_hex() == *n.tokenname.as_ref().unwrap()
        });
        if tmp.is_some() {
            tokens_on_contract.push(t)
        }
    }

    // ToDo: Check if conditions for reallocation are met or to return without working

    let transactions = match ada_on_token_utxos
        >= murin::clib::utils::from_bignum(&contract.get_contract_liquidity())
    {
        true => reallocate_tokens(&mut t_utxos, &tokens_on_contract, &addr)?,
        false => {
            let difference = murin::clib::utils::from_bignum(&contract.get_contract_liquidity())
                - ada_on_token_utxos;
            let additional_utxos =
                ada_utxos.coin_value_subset(murin::clib::utils::to_bignum(difference), None);
            t_utxos.merge(additional_utxos);
            reallocate_tokens(&mut t_utxos, &tokens_on_contract, &addr)?
        }
    };

    println!("\nTransactions: \n");
    transactions
        .iter()
        .for_each(|n| println!("{}", hex::encode(n.to_bytes())));

    Ok(())
}

fn reallocate_tokens(
    t_utxos: &mut murin::TransactionUnspentOutputs,
    tokens: &murin::Tokens,
    addr: &murin::clib::address::Address,
) -> Result<Vec<murin::clib::Transaction>> {
    let mut out = Vec::<murin::Transaction>::new();
    let ada = t_utxos.coin_sum();
    let (std_value, minutxo, utxo_count) = get_values_and_tamt_per_utxo(tokens, ada);
    println!("Min UTxO value: {:?}", minutxo);
    // ToDo: Filter all utxos larger or equal std_value and repeat to find new utxo count

    //T ToDo: Build recursive transactions

    txbuilder(t_utxos, &std_value, utxo_count, &mut out, addr);
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

fn txbuilder(
    utxos: &mut murin::TransactionUnspentOutputs,
    std_value: &murin::clib::utils::Value,
    utxo_amt: u64,
    transactions: &mut Vec<murin::clib::Transaction>,
    addr: &murin::clib::address::Address,
) -> (
    murin::TransactionUnspentOutputs,
    murin::clib::utils::Value,
    u64,
    Vec<murin::clib::Transaction>,
) {
    match (utxos.len(), utxo_amt) {
        (_, 0) => (
            utxos.to_owned(),
            std_value.to_owned(),
            utxo_amt.to_owned(),
            transactions.to_owned(),
        ),
        (0, _) => (
            utxos.to_owned(),
            std_value.to_owned(),
            utxo_amt.to_owned(),
            transactions.to_owned(),
        ),
        _ => {
            let (tx, mut used_utxos, new_utxo_amt) = make_new_tx(utxos, std_value, &utxo_amt, addr)
                .expect("Could not create transaction");
            utxos.delete_set(&used_utxos);
            transactions.push(tx);
            txbuilder(&mut used_utxos, std_value, new_utxo_amt, transactions, addr)
        }
    }
}

fn make_new_tx(
    utxos: &mut murin::TransactionUnspentOutputs,
    std_value: &murin::clib::utils::Value,
    utxo_amt: &u64,
    addr: &murin::clib::address::Address,
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
    let mut txw = murin::clib::TransactionWitnessSet::new();
    let vkeys = murin::make_dummy_vkeywitnesses(2);
    txw.set_vkeys(&vkeys);
    let mut tx = murin::clib::Transaction::new(&txb, &txw, None);
    let mut used_input_utxos = murin::TransactionUnspentOutputs::new();
    let change = murin::clib::utils::Value::new(&murin::clib::utils::to_bignum(0u64));

    let new_tx = add_utxos(
        &mut tx,
        utxos,
        &mut used_input_utxos,
        utxo_amt,
        std_value,
        &change,
        addr,
    )?;

    Ok((new_tx.0, new_tx.1, new_tx.3))
}

fn add_utxos(
    transaction: &mut murin::clib::Transaction,
    utxos: &mut murin::TransactionUnspentOutputs,
    used_input_utxos: &mut murin::TransactionUnspentOutputs,
    utxo_amt: &u64,
    std_value: &murin::clib::utils::Value,
    change: &murin::clib::utils::Value,
    addr: &murin::clib::address::Address,
) -> Result<(
    murin::clib::Transaction,
    murin::TransactionUnspentOutputs,
    murin::TransactionUnspentOutputs,
    u64,
    murin::clib::utils::Value,
    murin::clib::utils::Value,
    murin::clib::address::Address,
)> {
    match (utxo_amt, utxos.len()) {
        (0, _) => {
            let (tmp_tx, _) = finalize_tx(
                transaction,
                &transaction.body().inputs(),
                &transaction.body().outputs(),
                addr,
                change,
                &transaction.body().fee(),
            )?;

            return Ok((
                tmp_tx,
                utxos.to_owned(),
                used_input_utxos.to_owned(),
                utxo_amt.to_owned(),
                std_value.to_owned(),
                murin::clib::utils::Value::new(&murin::clib::utils::to_bignum(0u64)),
                addr.to_owned(),
            ));
        }
        (_, 0) => {
            let (tmp_tx, _) = finalize_tx(
                transaction,
                &transaction.body().inputs(),
                &transaction.body().outputs(),
                addr,
                change,
                &transaction.body().fee(),
            )?;

            return Ok((
                tmp_tx,
                utxos.to_owned(),
                used_input_utxos.to_owned(),
                utxo_amt.to_owned(),
                std_value.to_owned(),
                murin::clib::utils::Value::new(&murin::clib::utils::to_bignum(0u64)),
                addr.to_owned(),
            ));
        }
        _ => {}
    }

    let mut needed_value = std_value.clone();
    needed_value.set_coin(
        &needed_value
            .coin()
            .checked_add(&transaction.body().fee().clone())
            .unwrap(),
    );
    let security = murin::clib::utils::to_bignum(
        murin::clib::utils::from_bignum(&needed_value.coin()) + (2 * murin::htypes::MIN_ADA),
    ); // 10% Security for min utxo etc.
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
    let utxo_amt = utxo_amt - outputs.len() as u64;

    let out_value = murin::hfn::sum_output_values(&outputs);
    let in_value = used_input_utxos.calc_total_value()?;
    let change = change.checked_add(&in_value.checked_sub(&out_value)?)?;

    let txb =
        murin::clib::TransactionBody::new_tx_body(&inputs, &outputs, &transaction.body().fee());
    let transaction = murin::clib::Transaction::new(&txb, &transaction.witness_set(), None);

    let (tmp_tx, fee) = finalize_tx(
        &transaction,
        &inputs,
        &outputs,
        addr,
        &change,
        &transaction.body().fee(),
    )?;

    let txb = murin::clib::TransactionBody::new_tx_body(&inputs, &outputs, &fee);
    let mut transaction = murin::clib::Transaction::new(&txb, &transaction.witness_set(), None);

    match tmp_tx.to_bytes().len() {
        d if d > 15000 => Ok((
            tmp_tx,
            utxos.to_owned(),
            used_input_utxos.to_owned(),
            utxo_amt.to_owned(),
            std_value.to_owned(),
            murin::clib::utils::Value::new(&murin::clib::utils::to_bignum(0u64)),
            addr.to_owned(),
        )),
        _ => add_utxos(
            &mut transaction,
            utxos,
            used_input_utxos,
            &utxo_amt,
            std_value,
            &change,
            addr,
        ),
    }
}

fn finalize_tx(
    tx: &murin::clib::Transaction,
    inputs: &murin::clib::TransactionInputs,
    outputs: &murin::clib::TransactionOutputs,
    addr: &murin::clib::address::Address,
    change: &murin::clib::utils::Value,
    fee: &murin::clib::utils::BigNum,
) -> Result<(murin::clib::Transaction, murin::clib::utils::BigNum)> {
    let mut tmp_outputs = outputs.clone();
    tmp_outputs.add(&murin::clib::TransactionOutput::new(addr, change));
    let tmp_txb = murin::clib::TransactionBody::new_tx_body(inputs, &tmp_outputs, fee);
    let tmp_transaction = murin::clib::Transaction::new(&tmp_txb, &tx.witness_set(), None);

    let mem = murin::clib::utils::to_bignum(7000000u64); //cutils::to_bignum(7000000u64);
    let steps = murin::clib::utils::to_bignum(2500000000u64); //cutils::to_bignum(3000000000u64);
    let ex_unit_price: murin::htypes::ExUnitPrice = murin::ExUnitPrice {
        priceSteps: 7.21e-5,
        priceMemory: 5.77e-2,
    };
    let a = murin::clib::utils::to_bignum(44u64);
    let b = murin::clib::utils::to_bignum(155381u64);
    let fee = calc_txfee(&tmp_transaction, &a, &b, ex_unit_price, &steps, &mem, true);

    let mut tmp_outputs = outputs.clone();
    tmp_outputs.add(&murin::clib::TransactionOutput::new(
        addr,
        &change.checked_sub(&murin::clib::utils::Value::new(&fee))?,
    ));
    let tmp_txb = murin::clib::TransactionBody::new_tx_body(inputs, &tmp_outputs, &fee);
    Ok((
        murin::clib::Transaction::new(&tmp_txb, &tx.witness_set(), None),
        fee,
    ))
}
