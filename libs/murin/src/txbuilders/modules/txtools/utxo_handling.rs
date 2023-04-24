use super::error::TxToolsError;
use super::models::TokenAsset;
use crate::clib;
use crate::clib::{
    address::Address,
    utils::{to_bignum, Value},
};
use crate::{TransactionUnspentOutput, TransactionUnspentOutputs};

pub async fn find_token_utxos(
    inputs: TransactionUnspentOutputs,
    assets: Vec<TokenAsset>,
) -> Result<TransactionUnspentOutputs, TxToolsError> {
    let mut out = TransactionUnspentOutputs::new();
    if !inputs.is_empty() && !assets.is_empty() {
        for asset in assets {
            let mut needed_amt = asset.2;
            for i in 0..inputs.len() {
                let unspent_output = inputs.get(i);
                let value = unspent_output.output().amount();
                if let Some(multi) = value.multiasset() {
                    if let Some(toks) = multi.get(&asset.0) {
                        if let Some(amt) = toks.get(&asset.1) {
                            // for tok in 0..toks.len()  {
                            if needed_amt >= asset.2 {
                                log::debug!(
                                    "Found a utxo containing {} tokens {}.{}!",
                                    asset.2.to_str(),
                                    hex::encode(asset.0.to_bytes()),
                                    hex::encode(asset.1.to_bytes())
                                );
                                if !out.contains_tx(&unspent_output) {
                                    out.add(&unspent_output);
                                    needed_amt = needed_amt.clamped_sub(&amt);
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        return Err(TxToolsError::EmptyInputs);
    }
    Ok(out)
}

pub fn find_token_utxos_v2(
    inputs: &TransactionUnspentOutputs,
    needed: &Value,
    assets: Vec<TokenAsset>,
    on_addr: Option<&Address>,
) -> Result<TransactionUnspentOutputs, TxToolsError> {
    let mut out = TransactionUnspentOutputs::new();
    let ins = inputs.clone();
    let mut needed_w = needed.clone();

    if !inputs.is_empty() && !needed.is_zero() {
        for i in 0..ins.len() {
            let unspent_output = ins.get(i);
            if let Some(addr) = on_addr {
                if unspent_output.output().address().to_bytes() != addr.to_bytes() {
                    continue;
                }
            };
            let value = unspent_output.output().amount();
            if value.multiasset().is_some() {
                match value.checked_sub(&needed_w) {
                    Ok(o) => {
                        if let Some(n) = needed_w.compare(&o) {
                            if n == 1 {
                                out.add(&unspent_output);
                                needed_w = needed_w.checked_sub(&value)?;
                                break;
                            }
                        }
                    }
                    Err(_) => match needed_w.checked_sub(&value) {
                        Ok(o) => {
                            if let Some(n) = needed_w.compare(&o) {
                                if n > 0 {
                                    out.add(&unspent_output);
                                    needed_w = needed_w.checked_sub(&value)?;
                                }
                            }
                        }
                        Err(_) => continue,
                    },
                }
            }
        }
    } else {
        return Err(TxToolsError::Custom(
            "ERROR: cannot find token utxos , one of the provided inputs is empty".to_string(),
        ));
    }

    if out.is_empty() || !needed_w.is_zero() {
        trace!("Inputs: {:?}\n\n", inputs);
        debug!("Selected UTxOs: {:?}\n\n", out);
        return Err(TxToolsError::Custom(
            "ERROR: The token is not available in the utxo set".to_string(),
        ));
    }

    out.optimize_on_assets(assets)?;
    Ok(out)
}

pub fn find_token_utxos_na(
    inputs: &TransactionUnspentOutputs,
    assets: Vec<TokenAsset>,
    on_addr: Option<&Address>,
) -> Result<TransactionUnspentOutputs, TxToolsError> {
    let mut out = TransactionUnspentOutputs::new();

    if !inputs.is_empty() && !assets.is_empty() {
        for asset in &assets {
            let ins = inputs.clone();
            let mut needed_amt = asset.2;
            info!("Set Needed Amount: {:?}", needed_amt);
            info!("Input UTxOs count: {:?}", ins.len());
            for i in 0..ins.len() {
                let unspent_output = ins.get(i);
                if let Some(addr) = on_addr {
                    if unspent_output.output().address().to_bytes() != addr.to_bytes() {
                        continue;
                    }
                };
                let value = unspent_output.output().amount();
                if let Some(multi) = value.multiasset() {
                    if let Some(toks) = multi.get(&asset.0) {
                        if let Some(amt) = toks.get(&asset.1) {
                            if needed_amt.compare(&to_bignum(0)) > 0 {
                                log::info!(
                                    "Found a utxo at index {} containing {} tokens of {}.{}!",
                                    i,
                                    amt.to_str(),
                                    hex::encode(asset.0.to_bytes()),
                                    hex::encode(asset.1.to_bytes())
                                );
                                if out.contains_tx(&unspent_output) {
                                    log::info!("Already contained in selection: {:?}", &amt);
                                    needed_amt = needed_amt.clamped_sub(&amt);
                                    log::info!("New needed amount1: {:?}", &needed_amt);
                                } else {
                                    log::info!(
                                        "Not contained in selection yet, tokens on utxo: {:?}",
                                        &amt
                                    );
                                    out.add(&unspent_output);
                                    needed_amt = needed_amt.clamped_sub(&amt);
                                    log::info!("New needed amount2: {:?}", &needed_amt);
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        return Err(TxToolsError::Custom(
            "ERROR: cannot find token utxos , one of the provided inputs is empty".to_string(),
        ));
    }

    if out.is_empty() {
        trace!("Inputs: {:?}", inputs);
        return Err(TxToolsError::Custom(
            "ERROR: The token is not available in the utxo set".to_string(),
        ));
    }

    out.optimize_on_assets(assets)?;
    Ok(out)
}

pub fn input_selection(
    specific_input_utxos: Option<&TransactionUnspentOutputs>,
    needed_value: &mut Value,
    txins: &TransactionUnspentOutputs,
    exclude: Option<TransactionUnspentOutput>,
    on_addr: Option<&Address>,
) -> Result<(clib::TransactionInputs, TransactionUnspentOutputs), TxToolsError> {
    //debug!("\n\nMULTIASSETS: {:?}\n\n", txins);

    let (mut purecoinassets, mut multiassets) = crate::chelper::hfn::splitt_coin_multi(txins);

    let mut nv = needed_value.clone();
    let mut selection = TransactionUnspentOutputs::new();
    let mut acc = Value::new(&to_bignum(0u64));
    let mut txins = clib::TransactionInputs::new();

    let overhead = 50u64;

    if let Some(token_utxos) = specific_input_utxos {
        for i in 0..token_utxos.len() {
            selection.add(&token_utxos.get(i));
            acc = acc
                .checked_add(&token_utxos.get(i).output().amount())
                .unwrap();
            nv = nv
                .checked_add(&token_utxos.get(i).output().amount())
                .unwrap();
            trace!("\n\nAdded Script Utxo to Acc Value : \n {:?}\n", acc);
            // Delete script input from multi assets
            if let Some(i) = multiassets.find_utxo_index(&token_utxos.get(i)) {
                let tutxo = multiassets.swap_remove(i);
                trace!(
                    "Deleted token utxo from multiasset inputs: \n {:?}\n",
                    tutxo
                );
            }
        }
    }

    if let Some(exclude_utxo) = exclude {
        //debug!("Exclude: {:?}", exclude_utxo);
        let c_index =
            crate::chelper::hfn::find_collateral_by_txhash_txix(&exclude_utxo, &purecoinassets);
        //debug!(
        //    "Some excludes to check for deletion found, Index: {:?}",
        //    c_index
        //);
        if let Some(index) = c_index {
            purecoinassets.swap_remove(index);
            //debug!("deleted exclude from inputs: {:?}\n", col);
            // Double check
            if crate::chelper::hfn::find_collateral_by_txhash_txix(&exclude_utxo, &purecoinassets)
                .is_some()
            {
                return Err(TxToolsError::Custom(
                    "Error: exclude of utxos was not possible".to_string(),
                ));
            }
        }
    }

    // lookup tokens from needed value
    let mut tokens_to_find = crate::chelper::htypes::Tokens::new();
    if needed_value.multiasset().is_some() {
        if needed_value.multiasset().unwrap().len() > 0 {
            let pids = needed_value.multiasset().unwrap().keys();
            for i in 0..pids.len() {
                let policy = pids.get(i);
                let assets = needed_value.multiasset().unwrap().get(&policy);
                if let Some(a) = assets {
                    let assetnames = a.keys();
                    for j in 0..assetnames.len() {
                        let assetname = assetnames.get(j);
                        if let Some(amount) = a.get(&assetname) {
                            tokens_to_find.push((policy.clone(), assetname, amount));
                        }
                    }
                }
            }
        }
        let token_selection = find_token_utxos_na(
            &multiassets.clone(),
            // &needed_value.clone(),
            tokens_to_find,
            on_addr,
        )?; //find_token_utxos_na(&multiassets.clone(), tokens_to_find, on_addr)?;
        if !token_selection.is_empty() {
            for i in 0..token_selection.len() {
                selection.add(&token_selection.get(i));
                acc = acc
                    .checked_add(&token_selection.get(i).output().amount())
                    .unwrap();
                //debug!("\n\nAdded Script Utxo to Acc Value : \n {:?}\n", acc);
                // Delete script input from multi assets
                if let Some(i) = multiassets.find_utxo_index(&token_selection.get(i)) {
                    multiassets.swap_remove(i);
                    //debug!(
                    //    "Deleted token utxo from multiasset inputs: \n {:?}\n",
                    //    tutxo
                    //);
                }
            }
        }
    }

    multiassets.sort_by_coin();
    purecoinassets.sort_by_coin();

    //debug!("\n\nMULTIASSETS: {:?}\n\n", multiassets);
    //debug!("\n\npurecoinassets: {:?}\n\n", purecoinassets);

    let utxo_count = multiassets.len() + purecoinassets.len();
    let mut max_run = 0;
    //debug!("\n\nNV: {:?}", nv);
    //debug!("\n\nNV: {:?}", acc);
    //debug!(
    //    "\nbefore while! Utxo Count: {:?}, {:?} \n",
    //    utxo_count,
    //    (nv.coin().compare(&acc.coin()) > 0)
    //);
    while nv.coin().compare(&acc.coin()) > 0 && max_run < utxo_count {
        nv = nv.checked_sub(&acc).unwrap();

        if purecoinassets.is_empty() {
            // Find the tokens we want in the multis
            trace!("\nWe look for multiassets!\n");
            let ret = crate::chelper::hfn::find_suitable_coins(&mut nv, &mut multiassets, overhead);
            match ret.0 {
                Some(utxos) => {
                    for u in utxos {
                        selection.add(&u);
                    }
                    acc.set_coin(&acc.coin().checked_add(&to_bignum(ret.1)).unwrap());
                }
                None => {
                    //ToDo: Do not panic -> Error
                    panic!("ERROR: Not enough input utxos available to balance the transaction");
                }
            }
            let _ = multiassets.pop();
        } else {
            // Fine enough Ada to pay the transaction
            let ret =
                crate::chelper::hfn::find_suitable_coins(&mut nv, &mut purecoinassets, overhead);
            trace!("Return coinassets: {:?}", ret);
            match ret.0 {
                Some(utxos) => {
                    for u in utxos {
                        selection.add(&u);
                    }
                    acc.set_coin(&acc.coin().checked_add(&to_bignum(ret.1)).unwrap());
                    trace!("\nSelection in coinassets: {:?}", selection);
                    trace!("\nAcc in coinassets: {:?}", acc);
                }
                None => {
                    return Err(TxToolsError::Custom(
                        "ERROR: Not enough input utxos available to balance the transaction"
                            .to_string(),
                    ));
                }
            }
            let _ = purecoinassets.pop();
        }
        max_run += 1;
    }
    for txuo in selection.clone() {
        txins.add(&txuo.input());
    }
    trace!("\n\nSelection: {:?}\n\n", selection);
    Ok((txins, selection))
}

pub fn combine_wallet_outputs(txos: &clib::TransactionOutputs) -> clib::TransactionOutputs {
    let mut _txos = Vec::<clib::TransactionOutput>::new();
    let mut out = clib::TransactionOutputs::new();
    for o in 0..txos.len() {
        _txos.push(txos.get(o))
    }
    /*
    let wallets: Vec<clib::TransactionOutput> = _txos
        .iter()
        .filter(
            |n| true, /*clib::address::BaseAddress::from_address(&n.address()).is_some()*/
        )
        .map(|n| n.to_owned())
        .collect();
    */

    let addresses = _txos
        .iter()
        .fold(Vec::<clib::address::Address>::new(), |mut acc, n| {
            if !acc.contains(&n.address()) {
                acc.push(n.address());
            }
            acc
        });
    addresses.iter().for_each(|a| {
        let os: Vec<clib::TransactionOutput> = _txos
            .iter()
            .filter(|n| n.address() == *a)
            .cloned()
            .collect();
        match os.len() {
            i if i > 1 => {
                let cv = os.iter().fold(Value::zero(), |mut acc, n| {
                    acc = acc.checked_add(&n.amount()).unwrap();
                    acc
                });
                out.add(&clib::TransactionOutput::new(a, &cv));
            }
            1 => {
                out.add(&os[0]);
            }
            _ => {}
        }
    });
    /*
    let r: Vec<clib::TransactionOutput> = _txos
        .iter()
        .filter(|n| clib::address::BaseAddress::from_address(&n.address()).is_none())
        .map(|n| n.to_owned())
        .collect();
    for e in r {
        out.add(&e);
    }*/
    out
}
