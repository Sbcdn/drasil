/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/

use gungnir::Discount;
use mimir::TokenInfoMint;
use murin::{make_fingerprint, Tokens, TransactionUnspentOutputs};

pub fn discount(utxos: TransactionUnspentOutputs, cid: i64, uid: i64) -> i64 {
    let discounts = Discount::get_discounts(cid, uid);

    if discounts.is_err() {
        return 0;
    }
    let mut discounts = discounts.unwrap();
    log::debug!("\nFound Discounts: {discounts:?}");
    let mut policys = discounts.iter().fold(Vec::<String>::new(), |mut acc, n| {
        acc.push(n.policy_id());
        acc
    });

    let wallet_tokens = utxos.sum_avail_tokens();
    log::debug!("\nWallet Tokens: {wallet_tokens:?}");
    let mut avail_token = Tokens::new();
    for p in policys {
        let t: Tokens = wallet_tokens
            .iter()
            .filter(|n| hex::encode(n.0.to_bytes()) == p)
            .cloned()
            .collect();
        avail_token.extend(t.iter().cloned());
    }
    log::debug!("\nAvail Tokens: {:?}", avail_token);
    let mut del = Vec::<usize>::new();
    for (i, d) in discounts.iter_mut().enumerate() {
        let b: Vec<_> = if let Some(f) = d.fingerprint.to_owned() {
            avail_token
                .iter()
                .filter(|n| {
                    f == make_fingerprint(&hex::encode(n.0.to_bytes()), &hex::encode(n.1.name()))
                        .unwrap()
                })
                .cloned()
                .collect()
        } else {
            Tokens::new()
        };

        if d.fingerprint.is_some() && b.is_empty() {
            del.push(i);
        }
    }
    for d in del.iter().enumerate() {
        log::debug!("\nDelete: {:?}", d.1 - d.0);
        discounts.remove(d.1 - d.0);
    }
    log::debug!("\nDiscounts2: {:?}", discounts);
    if !del.is_empty() {
        policys = discounts.iter().fold(Vec::<String>::new(), |mut acc, n| {
            acc.push(n.policy_id());
            acc
        });
        avail_token = Tokens::new();
        for p in policys {
            let t: Tokens = wallet_tokens
                .iter()
                .filter(|n| hex::encode(n.0.to_bytes()) == p)
                .cloned()
                .collect();
            avail_token.extend(t.iter().cloned());
        }
    }
    log::debug!("\nAvail Tokens2: {avail_token:?}");
    let mut metadata = Vec::<TokenInfoMint>::new();
    for t in avail_token.iter() {
        log::debug!("\nToken iter: {:?}\n", t);
        let m = mimir::get_mint_metadata(
            &make_fingerprint(&hex::encode(t.0.to_bytes()), &hex::encode(t.1.name())).unwrap(),
        )
        .unwrap();
        log::debug!("\nMetadata iter: {:?}\n", m);
        metadata.push(m);
    }
    log::debug!("Metadata: {:?}", metadata);
    let mut dvalues = Vec::<i64>::new();
    for meta in metadata {
        let d: Vec<_> = discounts
            .iter()
            .filter(|n| n.policy_id == meta.policy)
            .collect();
        let v = if let Some(x) = meta.json {
            let o = x.as_object().unwrap();
            log::debug!("metaobject: {:?}", o);
            let cs = o.get(&meta.policy).unwrap().as_object().unwrap();
            log::debug!("cs: {:?}", cs);
            let mut elem = cs.get(&meta.tokenname).unwrap();
            log::debug!("elem1: {:?}", elem);
            for n in &d[0].metadata_path {
                elem = match elem.as_object() {
                    Some(o) => match o.get(n) {
                        Some(x) => x,
                        None => break,
                    },
                    None => break,
                };
            }
            log::debug!("elem2: {:?}", elem);
            if let Some(x) = elem.as_i64() {
                x
            } else if let Some(x) = elem.as_str() {
                if let Ok(y) = x.parse::<i64>() {
                    y
                } else {
                    0
                }
            } else if let Some(x) = elem.as_array() {
                if let Some(y) = x[0].as_i64() {
                    y
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        };
        dvalues.push(v);
    }
    dvalues.sort();
    log::debug!("Dvalues sort: {:?}", dvalues);
    if dvalues.is_empty() {
        0
    } else {
        dvalues[dvalues.len() - 1]
    }
}
