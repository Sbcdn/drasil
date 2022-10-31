/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/

use gungnir::{Discount, DiscountNew};
use mimir::TokenInfoMint;
use murin::clib::{address::Address, utils::from_bignum};
use murin::{make_fingerprint, MurinError, Tokens, TransactionUnspentOutputs};

use crate::MarketplaceActions;

pub fn discount(utxos: TransactionUnspentOutputs, cid: i64, uid: i64) -> i64 {
    let discounts = Discount::get_discounts(cid, uid);

    if let Err(_) = discounts {
        return 0;
    }
    let mut discounts = discounts.unwrap();

    let mut policys = discounts.iter().fold(Vec::<String>::new(), |mut acc, n| {
        acc.push(n.policy_id());
        acc
    });

    let wallet_tokens = utxos.sum_avail_tokens();
    let mut avail_token = Tokens::new();
    for p in policys {
        let t: Tokens = wallet_tokens
            .iter()
            .filter(|n| hex::encode(n.0.to_bytes()) == p)
            .cloned()
            .collect();
        avail_token.extend(t.iter().cloned());
    }
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

        if b.is_empty() {
            del.push(i);
        }
    }
    for d in del.iter().enumerate() {
        discounts.remove(d.1 - d.0);
    }
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

    let mut metadata = Vec::<TokenInfoMint>::new();
    for t in avail_token.iter() {
        let m = mimir::get_mint_metadata(
            &make_fingerprint(&hex::encode(t.0.to_bytes()), &hex::encode(t.1.name())).unwrap(),
        )
        .unwrap();
        metadata.push(m);
    }
    let mut dvalues = Vec::<i64>::new();
    for meta in metadata {
        let d: Vec<_> = discounts
            .iter()
            .filter(|n| n.policy_id == meta.policy)
            .collect();
        let v = if let Some(x) = meta.json {
            let o = x.as_object().unwrap();
            let cs = o.get(&meta.policy).unwrap().as_object().unwrap();
            let mut elem = cs.get(&meta.tokenname).unwrap();
            for n in &d[0].metadata_path {
                elem = elem.as_object().unwrap().get(n).unwrap();
            }
            elem.as_i64().unwrap()
        } else {
            0
        };
        dvalues.push(v);
    }
    dvalues.sort();
    dvalues[dvalues.len() - 1]
}
