use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr, crypto as ccrypto, utils as cutils};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::wallet;
use crate::txbuilder;
use crate::MurinError;

macro_rules! pub_struct {
    ($name:ident {$($field:ident: $t:ty,)*}) => {
        #[derive(Serialize,Deserialize,Debug, Clone, PartialEq)]
        pub struct $name {
            $(pub $field: $t),*
        }
    }
}

pub const DUMMY_VKEYWITNESS     : &str = "8258203818ad60f55faef4576ff88e9e7e1148fcb11d602ffa19def6e9c44b420fdaa25840751a9f1c01cf068e8b0becf3122832d13f8fc1dff74a43059b815e949442ad6b60c6a67d4b39e4a3271064665418960731280d0ef7ae5a471a98021cae074001";
pub const MIN_ADA: u64 = 1000000;

pub struct BuildOutput {
    pub tx_witness: String,
    pub metadata: String,
    pub tx_body: String,
    pub tx_unsigned: String,
    pub used_utxos: String,
    pub royalties: u64,
    pub internal_transfer: String,
}

impl BuildOutput {
    pub fn get_tx_unsigned(&self) -> String {
        self.tx_unsigned.clone()
    }

    pub fn get_tx_body(&self) -> String {
        self.tx_body.clone()
    }

    pub fn get_tx_body_typed(&self) -> clib::TransactionBody {
        clib::TransactionBody::from_bytes(hex::decode(self.tx_body.clone()).unwrap()).unwrap()
    }

    pub fn get_metadata(&self) -> String {
        self.metadata.clone()
    }

    pub fn get_metadata_typed(&self) -> clib::metadata::AuxiliaryData {
        clib::metadata::AuxiliaryData::from_bytes(hex::decode(self.metadata.clone()).unwrap())
            .unwrap()
    }

    pub fn get_txwitness(&self) -> String {
        self.tx_witness.clone()
    }

    pub fn get_txwitness_typed(&self) -> clib::TransactionWitnessSet {
        clib::TransactionWitnessSet::from_bytes(hex::decode(self.tx_witness.clone()).unwrap())
            .unwrap()
    }

    pub fn get_payed_royalties(&self) -> u64 {
        self.royalties
    }
    pub fn get_used_utxos(&self) -> String {
        self.used_utxos.clone()
    }
}

pub_struct!(SmartContract {
    r#type: String,
    description: String,
    cborHex: String,
});

pub_struct!(Assets {
    tokenName: String,
    amount: u64,
});

pub_struct!(Value {               // lovelace is empty String or "lovelace" in currency symbol
    currencySymbol : String, // Hexadecimal ByteString ; what type to make lib::Asset out of it?
    assets         : Vec<Assets>, // Hexadecimal ByteString ; what type to make lib:Asset out of it ?

});

pub_struct!(ScriptOutput {
    //address    : String,
    value      : Vec<Value>,
    txhash     : String,
    txinput    : u32,
});

pub_struct!(TxOutput {
    address : String,
    value   : Vec<Value>,
});

pub_struct!(TxInput {
    address : String,
    txhash  : String,
    txinput : u32,
    value   : Vec<Value>,
});

//#[derive(Serialize,Deserialize, Debug,Clone)]
pub_struct!(MpTxData {
 script_outputs     : Vec<ScriptOutput>,
 outputs            : Vec<TxOutput>,
 inputs             : Vec<TxInput>,
 collateral_input   : String,
 trade_owner        : String,
 senders_address    : String,
 selling_price      : String,
 change_address     : String,
 network_id         : String,
 current_slot       : u32,
 metadata           : Vec<String>,
 royalties_rate     : f64,
 royalties_addr     : String,
});

pub_struct!(TxMetadata {
    metadata : Vec<String>,
    network  : String,
});

pub_struct!(DecodedMetadata {
    trade_owner: String,
    policy_id: String,
    token_name: String,
    selling_price: String,
    royalties_pkey: String,
    royalties_rate: String,
    datumhash: String,
    smart_contract: String,
});

pub_struct!(ExUnitPrice {
    priceSteps: f64,
    priceMemory: f64,
});

pub_struct!(OutputSizeConstants {
    k0: usize,
    k1: usize,
    k2: usize,
    k3: usize,
    _k4: usize,
});

pub_struct!(TxHash {
    tx_hash: String,
    message: String,
});

// Traits

pub trait ARemove {
    fn aremove(&mut self, index: usize) -> clib::TransactionOutputs;
}

impl ARemove for clib::TransactionOutputs {
    fn aremove(&mut self, index: usize) -> clib::TransactionOutputs {
        let mut res = clib::TransactionOutputs::new();
        for tx in 0..self.len() {
            if tx != index {
                res.add(&self.get(tx));
            }
        }
        res
    }
}

pub type TransactionUnspentOutput = cutils::TransactionUnspentOutput;

#[derive(Clone, Debug)]
pub struct TransactionUnspentOutputs(pub(crate) Vec<TransactionUnspentOutput>);

impl Default for TransactionUnspentOutputs {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionUnspentOutputs {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn pop(&mut self) -> Option<cutils::TransactionUnspentOutput> {
        self.0.pop()
    }

    pub fn get(&self, index: usize) -> cutils::TransactionUnspentOutput {
        self.0[index].clone()
    }

    pub fn add(&mut self, elem: &cutils::TransactionUnspentOutput) {
        self.0.push(elem.clone());
    }

    pub fn insert(&mut self, pos: usize, elem: cutils::TransactionUnspentOutput) {
        self.0.insert(pos, elem);
    }

    pub fn find_utxo_index(&self, elem: &TransactionUnspentOutput) -> Option<usize> {
        let elem_hash = elem.input().transaction_id();
        let elem_index = elem.input().index();
        trace!("Elemhash: {:?}, ElemIndex: {:?}", elem_hash, elem_index);
        for i in 0..self.0.len() {
            let txi = self.0[i].input();
            trace!("Input: {:?}", txi);
            if txi.transaction_id().to_bytes() == elem_hash.to_bytes() && txi.index() == elem_index
            {
                debug!(
                    "Found utxo by txhash: {:?}, txix: {:?}",
                    elem_hash, elem_index
                );
                return Some(i);
            }
        }
        None
    }

    pub fn merge(&mut self, other: TransactionUnspentOutputs) {
        for elem in other {
            self.0.push(elem);
        }
    }

    pub fn sort_by_coin(&mut self) {
        self.0
            .sort_by_cached_key(|k| cutils::from_bignum(&k.output().amount().coin()));
    }

    pub fn sort_by_multi_amount(&mut self) {
        self.0.sort_by_cached_key(get_amount);

        fn get_amount(k: &TransactionUnspentOutput) -> usize {
            let mut acc = 0usize;
            let policies = k.output().amount().multiasset().unwrap().keys();
            let assets = k.output().amount().multiasset().unwrap();
            for policy in 0..policies.len() {
                acc += assets.get(&policies.get(policy)).unwrap().len();
            }
            acc += policies.len() * 1.5 as usize;
            acc
        }
    }

    /// sort by asset amount, largest first
    pub fn sort_by_asset_amount(&mut self, policy: &ccrypto::ScriptHash, asset: &clib::AssetName) {
        self.0.sort_by_cached_key(|k| get_amount(k, policy, asset));

        fn get_amount(
            k: &TransactionUnspentOutput,
            policy: &ccrypto::ScriptHash,
            asset: &clib::AssetName,
        ) -> usize {
            match k.output().amount().multiasset() {
                Some(ma) => match ma.get(policy) {
                    Some(assets) => match assets.get(asset) {
                        Some(a) => cutils::from_bignum(&a) as usize,
                        None => 100,
                    },
                    None => 1000,
                },
                None => 2000,
            }
        }

        self.0.reverse()
    }

    pub fn convert_to_csl(&self) -> cardano_serialization_lib::utils::TransactionUnspentOutputs {
        let mut out = cardano_serialization_lib::utils::TransactionUnspentOutputs::new();
        for elem in &self.0 {
            out.add(&elem.clone());
        }
        out
    }

    pub fn optimize_on_assets(&mut self, assets: Tokens) -> Result<(), MurinError> {
        let tot_val_utxos = self.calc_total_value()?;
        let needed_val = tokens_to_value(&assets);
        if let Ok(overhead_value) = tot_val_utxos.checked_sub(&needed_val) {
            if overhead_value.multiasset().is_some() {
                let mut overhead_tok = value_to_tokens(&overhead_value)?;
                let mut list_of_utxo_tokens = Vec::<(Tokens, TransactionUnspentOutput)>::new();
                for t in self.0.clone().iter() {
                    list_of_utxo_tokens.push((value_to_tokens(&t.output().amount())?, t.clone()));
                }

                for tokens in &list_of_utxo_tokens {
                    let diff = token_diff(&tokens.0, &overhead_tok);
                    trace!("Diff: {:?}", diff);
                    match diff.len() {
                        0 => {
                            continue;
                        }
                        _ => {
                            if let Some(k) = list_of_utxo_tokens.iter().find(|k| k.0 == diff) {
                                trace!("Found k: {:?}", k);
                                if let Some(pos) = self.find_utxo_index(&k.1) {
                                    self.swap_remove(pos);
                                    overhead_tok =
                                        value_to_tokens(&tot_val_utxos.clamped_sub(&needed_val))?;
                                }
                            }
                        }
                    }
                }
                return Ok(());
            }
        }
        Ok(())
    }

    pub fn add_if_not_contained(&mut self, addons: &TransactionUnspentOutput) -> Option<()> {
        if self.0.iter().any(|n| n.to_bytes() == addons.to_bytes()) {
            self.add(addons);
            return Some(());
        };
        None
    }

    pub fn sum_avail_tokens(&self) -> Tokens {
        let mut out = Tokens::new();
        self.0.iter().for_each(|n| {
            if let Some(ma) = n.output().amount().multiasset() {
                let policies = ma.keys();
                for p in 0..policies.len() {
                    let assets = ma.get(&policies.get(p)).unwrap();
                    let ans = assets.keys();
                    for a in 0..assets.keys().len() {
                        let n = assets.get(&ans.get(a)).unwrap();
                        out.push((policies.get(p), ans.get(a), n))
                    }
                }
            }
        });
        sum_unique_tokens(&out)
    }

    pub fn coin_sum(&self) -> u64 {
        self.0
            .iter()
            .map(|utxo| cutils::from_bignum(&utxo.output().amount().coin()))
            .sum()
    }

    pub fn coin_sum_minutxo(&self) -> u64 {
        //cutils::BigNum {
        let coinsum: u64 = self
            .0
            .iter()
            .map(|utxo| cutils::from_bignum(&utxo.output().amount().coin()))
            .sum();

        let minutxo = cutils::from_bignum(&txbuilder::calc_min_ada_for_utxo(
            &self
                .calc_total_value()
                .expect("calc total value crashed in coin_sum_minutxo"),
            None,
        ));

        std::cmp::min(coinsum - minutxo, 0)
    }

    pub fn coin_value_subset(
        &self,
        ncoin: cutils::BigNum,
        already_in_use: Option<&TransactionUnspentOutputs>,
    ) -> TransactionUnspentOutputs {
        use itertools::FoldWhile::{Continue, Done};
        let i = self
            .0
            .iter()
            .fold_while(
                TransactionUnspentOutputs::new(),
                |mut acc: TransactionUnspentOutputs, x| {
                    if acc.coin_sum() >= cutils::from_bignum(&ncoin) {
                        Done(acc)
                    } else {
                        Continue({
                            if let Some(u) = already_in_use {
                                if !u.contains_tx(x) {
                                    acc.add(x);
                                }
                            } else {
                                acc.add(x);
                            }
                            acc
                        })
                    }
                },
            )
            .into_inner();
        i
    }

    pub fn coin_value_subset_minutxo(
        &self,
        ncoin: &cutils::BigNum,
        payaddr: &caddr::Address,
    ) -> TransactionUnspentOutputs {
        use itertools::FoldWhile::{Continue, Done};
        let i = self
            .0
            .iter()
            .fold_while(
                TransactionUnspentOutputs::new(),
                |mut acc: TransactionUnspentOutputs, x| {
                    if acc.coin_sum_minutxo() >= cutils::from_bignum(ncoin) {
                        Done(acc)
                    } else {
                        Continue({
                            let x_stake = wallet::stake_keyhash_from_address(
                                &x.output().address(),
                            )
                            .expect(
                                "Could not determine stake address in coin_value_subset_minutxo 1",
                            );
                            let payer_stake = wallet::stake_keyhash_from_address(payaddr).expect(
                                "Could not determine stake address in coin_value_subset_minutxo 2",
                            );
                            if x_stake == payer_stake {
                                acc.add(x);
                            }
                            acc
                        })
                    }
                },
            )
            .into_inner();
        i
    }

    pub fn subset_address(&self, payaddr: &caddr::Address) -> TransactionUnspentOutputs {
        let out = self.0.iter().fold(
            TransactionUnspentOutputs::new(),
            |mut acc: TransactionUnspentOutputs, x| {
                if x.output().address() == *payaddr {
                    acc.add(x)
                };
                acc
            },
        );
        out
    }

    pub fn delete_set(&mut self, set: &Self) {
        let b_set = set.to_bytes();
        self.0.retain(|n| !b_set.contains(&n.to_bytes()));
    }

    pub fn to_bytes(&self) -> Vec<Vec<u8>> {
        let mut out = Vec::<Vec<u8>>::new();
        out.extend(self.0.iter().map(|n| n.to_bytes()));
        out
    }

    pub fn swap_remove(&mut self, pos: usize) -> TransactionUnspentOutput {
        self.0.swap_remove(pos)
    }

    pub fn contains_tx(&self, elem: &TransactionUnspentOutput) -> bool {
        let elem_hash = elem.input().transaction_id();
        let elem_index = elem.input().index();
        debug!("Elemhash: {:?}, ElemIndex: {:?}", elem_hash, elem_index);
        for i in 0..self.0.len() {
            let txi = self.0[i].input();
            trace!("Input: {:?}", txi);
            if txi.transaction_id().to_bytes() == elem_hash.to_bytes() && txi.index() == elem_index
            {
                debug!(
                    "Found utxo by txhash: {:?}, txix: {:?}",
                    elem_hash, elem_index
                );
                return true;
            }
        }
        false
    }
    pub fn get_coin_only(&self) -> TransactionUnspentOutputs {
        let coin_only = self
            .0
            .iter()
            .filter(|n| n.to_owned().output().amount().multiasset().is_none())
            .collect();
        coin_only
    }

    pub fn get_token_only(&self) -> TransactionUnspentOutputs {
        let token_only = self
            .0
            .iter()
            .filter(|n| n.to_owned().output().amount().multiasset().is_some())
            .collect();
        token_only
    }

    pub fn contains_any(&self, elems: &TransactionUnspentOutputs) -> bool {
        for i in 0..self.0.len() {
            let txi_self = self.0[i].input();
            trace!("Input: {:?}", txi_self);
            for j in 0..elems.len() {
                let txi_other = elems.get(j).input();
                if txi_self.transaction_id().to_bytes() == txi_other.transaction_id().to_bytes()
                    && txi_self.index() == txi_other.index()
                {
                    trace!("Utxo set contains minimum one utxo of the other set");
                    return true;
                }
            }
        }
        false
    }

    pub fn contains_address(&self, addr: caddr::Address) -> bool {
        for i in 0..self.0.len() {
            if self.0[i].output().address().to_bytes() == addr.to_bytes() {
                return true;
            }
        }
        false
    }

    pub fn filter_values(
        &self,
        value: &cutils::Value,
        band: Option<i8>,
    ) -> Result<TransactionUnspentOutputs, crate::MurinError> {
        let mut min = value.clone();
        let mut max = value.clone();
        if let Some(b) = band {
            (min, max) = TransactionUnspentOutputs::band_value(value, b);
        }
        trace!("Band Values: Min: {:?}; Max: {:?}", min, max);
        let f: TransactionUnspentOutputs = self
            .0
            .iter()
            .filter(|n| {
                n.output().amount().compare(&min).unwrap_or(-1) >= 0
                    && n.output().amount().compare(&max).unwrap_or(1) <= 0
            })
            .collect();
        Ok(f)
    }

    fn band_value(value: &cutils::Value, band: i8) -> (cutils::Value, cutils::Value) {
        let coin = cutils::from_bignum(&value.coin());
        let mut min_val = cutils::Value::new(&cutils::to_bignum(coin - (coin / 100 * band as u64)));
        let mut max_val = cutils::Value::new(&cutils::to_bignum(coin + (coin / 100 * band as u64)));
        if value.multiasset().is_some() {
            let mut min_ma = clib::MultiAsset::new();
            let mut max_ma = clib::MultiAsset::new();
            let ma = value.multiasset().unwrap();
            for i in 0..ma.keys().len() {
                let policy = ma.keys().get(i);
                let assets = ma.get(&policy).unwrap();
                let mut min_assets = clib::Assets::new();
                let mut max_assets = clib::Assets::new();
                for j in 0..assets.keys().len() {
                    let asset = assets.keys().get(j);
                    let amt = cutils::from_bignum(&assets.get(&asset).unwrap());
                    let min_amt = cutils::to_bignum(amt - (amt / 100 * band as u64));
                    let max_amt = cutils::to_bignum(amt + (amt / 100 * band as u64));
                    min_assets.insert(&asset, &min_amt);
                    max_assets.insert(&asset, &max_amt);
                }
                min_ma.insert(&policy, &min_assets);
                max_ma.insert(&policy, &max_assets);
            }
            min_val.set_multiasset(&min_ma);
            max_val.set_multiasset(&max_ma);
        }
        (min_val, max_val)
    }

    pub fn reverse(&mut self) {
        self.0.reverse();
    }

    pub fn to_hex(&self) -> Result<String, crate::MurinError> {
        use cbor_event::{se::Serializer, Len};
        let mut serializer = Serializer::new_vec();
        serializer.write_array(Len::Len(self.len() as u64))?;

        for i in 0..self.len() {
            serializer.write_bytes(&self.0[i].to_bytes())?;
        }
        let bytes = serializer.finalize();
        Ok(hex::encode(bytes))
    }

    pub fn from_hex(str: &str) -> Result<TransactionUnspentOutputs, crate::MurinError> {
        use cbor_event::de::*;
        use std::io::Cursor;
        let vec = hex::decode(str)?;
        let mut raw = Deserializer::from(Cursor::new(vec));

        let mut v = TransactionUnspentOutputs::new();
        (|| -> Result<(), crate::MurinError> {
            let len = raw.array()?;
            while match len {
                cbor_event::Len::Len(n) => v.len() < n as usize,
                cbor_event::Len::Indefinite => true,
            } {
                if raw.cbor_type()? == cbor_event::Type::Special {
                    assert_eq!(raw.special()?, cbor_event::Special::Break);
                    break;
                }
                let u = raw.bytes()?;
                let t = TransactionUnspentOutput::from_bytes(u).unwrap();
                v.add(&t);
            }

            Ok(())
        })()?;
        Ok(v)
    }

    pub fn calc_total_value(&self) -> Result<cutils::Value, super::MurinError> {
        let mut tval = cutils::Value::new(&cutils::to_bignum(0u64));
        for i in 0..self.len() {
            tval = tval.checked_add(&self.get(i).output().amount())?;
        }
        Ok(tval)
    }

    pub fn find_utxo_by_txhash(&self, tx_hash: &String, tx_index: u32) -> Option<usize> {
        for (i, e) in self.0.clone().into_iter().enumerate() {
            if hex::encode(e.input().transaction_id().to_bytes()) == *tx_hash
                && e.input().index() == tx_index
            {
                return Some(i);
            }
        }
        None
    }

    pub fn find_utxo_containing_policy(
        &self,
        policy: &String,
    ) -> Result<TransactionUnspentOutputs, super::MurinError> {
        let policy = ccrypto::ScriptHash::from_bytes(hex::decode(policy)?)?;
        let mut out = TransactionUnspentOutputs::new();
        self.0.iter().for_each(|n| {
            let ma = n.output().amount().multiasset();
            if let Some(multi) = ma {
                let policies = multi.get(&policy);
                if policies.is_some() {
                    out.add(n)
                }
            }
        });
        Ok(out)
    }

    pub fn find_utxos_containing_asset(
        &self,
        policy: &ccrypto::ScriptHash,
        assetname: &clib::AssetName,
    ) -> Result<TransactionUnspentOutputs, super::MurinError> {
        let mut out = TransactionUnspentOutputs::new();
        self.0.iter().for_each(|n| {
            let ma = n.output().amount().multiasset();
            if let Some(multi) = ma {
                let asset = multi.get_asset(policy, assetname);
                if asset.compare(&cutils::to_bignum(0)) == 1 {
                    out.add(n)
                }
            }
        });
        Ok(out)
    }

    pub fn remove_used_utxos(&mut self, used: Vec<crate::utxomngr::usedutxos::UsedUtxo>) {
        for u in used {
            if let Some(i) = self.find_utxo_by_txhash(u.get_txhash(), u.get_index()) {
                let d = self.swap_remove(i);
                trace!("Deleted: {:?}", d);
            }
        }
    }
}

impl<'a> FromIterator<&'a cutils::TransactionUnspentOutput> for TransactionUnspentOutputs {
    fn from_iter<I: IntoIterator<Item = &'a cutils::TransactionUnspentOutput>>(iter: I) -> Self {
        let mut tuos = TransactionUnspentOutputs::new();
        for i in iter {
            tuos.add(i);
        }
        tuos
    }
}

impl Iterator for TransactionUnspentOutputs {
    type Item = TransactionUnspentOutput;

    fn next(&mut self) -> Option<Self::Item> {
        self.pop()
    }
}

pub type Tokens = Vec<(ccrypto::ScriptHash, clib::AssetName, cutils::BigNum)>;

pub fn sum_unique_tokens(tokens: &Tokens) -> Tokens {
    let mut out = Tokens::new();
    let mut shas = Vec::<(ccrypto::ScriptHash, clib::AssetName)>::new();
    for t in tokens {
        if !shas.contains(&(t.0.to_owned(), t.1.to_owned())) {
            shas.push((t.0.to_owned(), t.1.to_owned()))
        }
    }

    for t in shas {
        let mut tos = tokens.clone();

        tos.retain(|n| n.0 == t.0 && n.1 == t.1);
        let f = tos.iter().fold(0, |acc, n| acc + cutils::from_bignum(&n.2));

        out.push((t.0, t.1, cutils::to_bignum(f)))
    }
    out
}

pub fn tokens_to_value(tokens: &Tokens) -> cutils::Value {
    let mut val = clib::utils::Value::new(&cutils::to_bignum(0u64));
    let mut ma = clib::MultiAsset::new();
    for token in tokens {
        match ma.get(&token.0) {
            Some(mut assets) => {
                assets.insert(&token.1, &token.2);
                ma.insert(&token.0, &assets);
            }
            None => {
                let mut assets = clib::Assets::new();
                assets.insert(&token.1, &token.2);
                ma.insert(&token.0, &assets);
            }
        }
    }
    val.set_multiasset(&ma);

    debug!("Tokens to Val: {:?}", val);
    val
}

pub fn value_to_tokens(value: &cutils::Value) -> Result<Tokens, MurinError> {
    let ma = match value.multiasset() {
        Some(multiassets) => multiassets,
        None => {
            return Err(MurinError::new(
                "Value does not contain any multiassets cannot convert to 'Tokens'",
            ))
        }
    };
    let mut tokens = Tokens::new();
    let policies = ma.keys();
    for i in 0..policies.len() {
        let policy = policies.get(i);
        let assets = match ma.get(&policy) {
            Some(a) => a,
            None => continue,
        };
        let asset_keys = assets.keys();
        for j in 0..assets.len() {
            let asset = asset_keys.get(j);
            let amt = match assets.get(&asset) {
                Some(a) => a,
                None => continue,
            };
            tokens.push((policy.clone(), asset, amt))
        }
    }
    Ok(tokens)
}

pub fn find_tokenindex_by_policy_assetname(
    t: &Tokens,
    p: &ccrypto::ScriptHash,
    a: &clib::AssetName,
) -> Option<usize> {
    t.iter()
        .enumerate()
        .find(|k| k.1 .0 == *p && k.1 .1 == *a)
        .map(|k| k.0)
}

pub fn check_overhead_tokens(tok_l: &Tokens, tok_r: &Tokens) -> Tokens {
    let mut out = Tokens::new();
    for tok in tok_l {
        if let Some(s) = find_tokenindex_by_policy_assetname(tok_r, &tok.0, &tok.1) {
            if tok_r[s].2.compare(&tok.2) >= 0 {
                //cutils::from_bignum(&r) >= 0 {
                out.push((tok.0.clone(), tok.1.clone(), tok.2))
            }
        }
    }
    if tok_l.len() != out.len() {
        out = Tokens::new();
    }
    out
}

pub fn acc_tokens(tok_l: &Tokens, tok_r: &Tokens) -> Tokens {
    let mut out = Tokens::new();
    for tok in tok_l {
        if let Some(s) = find_tokenindex_by_policy_assetname(tok_r, &tok.0, &tok.1) {
            if let Ok(_bn) = tok.2.checked_sub(&tok_r[s].2) {
                debug!("Found utxo which can be kick optimized");
                out.push((tok.0.clone(), tok.1.clone(), tok.2))
            }
        }
    }

    out
}

pub fn token_diff(tok_l: &Tokens, tok_r: &Tokens) -> Tokens {
    let sub = check_overhead_tokens(tok_l, tok_r);
    acc_tokens(tok_l, &sub)
}

/// Create utxo from hex string slice
pub fn utxo_from_hex(hex: &str) -> Result<TransactionUnspentOutput, crate::MurinError> {
    let utxo = TransactionUnspentOutput::from_hex(hex)?;
    Ok(utxo)
}
