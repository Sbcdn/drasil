pub(crate) mod error;

use std::ops::{Div, Rem, Sub};

use bech32::{self, ToBase32};
use cardano_serialization_lib as csl;
use cardano_serialization_lib::{
    address::{Address, BaseAddress, EnterpriseAddress, RewardAddress},
    crypto::{DataHash, ScriptHash, TransactionHash},
    utils::{from_bignum, to_bignum, BigNum, Value},
    AssetName, MultiAsset,
};
use cryptoxide::{blake2b::Blake2b, digest::Digest};
use csl::{PolicyID, TransactionInputs, TransactionOutput, TransactionOutputs};
use error::CSLCommonError;
use log::error;
use serde::{Deserialize, Serialize};

pub const DUMMY_VKEYWITNESS     : &str = "8258203818ad60f55faef4576ff88e9e7e1148fcb11d602ffa19def6e9c44b420fdaa25840751a9f1c01cf068e8b0becf3122832d13f8fc1dff74a43059b815e949442ad6b60c6a67d4b39e4a3271064665418960731280d0ef7ae5a471a98021cae074001";
pub const MIN_ADA: u64 = 1000000;

pub type TransactionUnspentOutput = csl::utils::TransactionUnspentOutput;
pub type Token = (ScriptHash, AssetName, BigNum);
pub type Tokens = Vec<Token>;

/// Create cardano address from string
pub fn addr_from_str(s: &str) -> Result<Address, CSLCommonError> {
    match hex::decode(s) {
        Ok(bytes) => Ok(Address::from_bytes(bytes)?),
        Err(_) => match Address::from_bech32(s) {
            Ok(addr) => Ok(addr),
            Err(_) => Err(CSLCommonError::CSLError),
        },
    }
}

/// Deserialize cbor encoded utxos into TransactionUnspentOutputs, filter collateral and excluded utxos if provided
pub fn decode_transaction_unspent_outputs(
    enc_txuos: &[String],
    col_utxo: Option<&Vec<String>>,
    enc_excl: Option<&Vec<String>>,
) -> Result<TransactionUnspentOutputs, CSLCommonError> {
    let mut txuos = TransactionUnspentOutputs::new();
    let mut utxos = enc_txuos.to_vec();

    // Filter exculdes if there are some
    if let Some(enc) = enc_excl {
        for e in enc {
            utxos = utxos.into_iter().filter(|utxo| *utxo != *e).collect();
        }
    }
    // filter collateral if there is some
    if let Some(col) = col_utxo {
        for c in col {
            utxos = utxos.into_iter().filter(|utxo| *utxo != *c).collect();
        }
    }
    // convert to TransactionunspentOutputs
    for utxo in utxos {
        txuos.add(&TransactionUnspentOutput::from_bytes(hex::decode(utxo)?)?);
    }
    Ok(txuos)
}

/// Deserialize a single cbor encoded UTxO into TransactionUnspentOutput
pub fn decode_transaction_unspent_output(
    encoded_utxo: &String,
) -> Result<TransactionUnspentOutput, CSLCommonError> {
    Ok(TransactionUnspentOutput::from_bytes(hex::decode(
        encoded_utxo,
    )?)?)
}

/// converts network id into NetworkIdKind
pub fn network_kind(net_id: u64) -> Result<csl::NetworkIdKind, CSLCommonError> {
    match net_id {
        0 => Ok(csl::NetworkIdKind::Testnet),
        1 => Ok(csl::NetworkIdKind::Mainnet),
        _ => Err(CSLCommonError::Custom("invalid network id".to_string())),
    }
}

/// determine reward adress from cardano address if existing
/// In the case of an enterprise address the enterprise address is returned,
pub fn get_stakeaddr_from_addr(addr: &Address) -> Result<Address, CSLCommonError> {
    match BaseAddress::from_address(addr) {
        Some(baseaddr) => {
            Ok(RewardAddress::new(addr.network_id()?, &baseaddr.stake_cred()).to_address())
        }
        None => match RewardAddress::from_address(addr) {
            Some(reward) => Ok(reward.to_address()),
            None => match EnterpriseAddress::from_address(addr) {
                Some(addr) => Ok(addr.to_address()),
                None => Err(CSLCommonError::CSLError),
            },
        },
    }
}

pub struct OutputSizeConstants {
    k0: usize,
    k1: usize,
    k2: usize,
    k3: usize,
    _k4: usize,
}

/// Overwrite TransactionUnspentOutputs of CSL
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

    pub fn pop(&mut self) -> Option<TransactionUnspentOutput> {
        self.0.pop()
    }

    pub fn get(&self, index: usize) -> TransactionUnspentOutput {
        self.0[index].clone()
    }

    pub fn add(&mut self, elem: &TransactionUnspentOutput) {
        self.0.push(elem.clone());
    }

    pub fn insert(&mut self, pos: usize, elem: TransactionUnspentOutput) {
        self.0.insert(pos, elem);
    }

    pub fn find_utxo_index(&self, elem: &TransactionUnspentOutput) -> Option<usize> {
        let elem_hash = elem.input().transaction_id();
        let elem_index = elem.input().index();
        for i in 0..self.0.len() {
            let txi = self.0[i].input();
            if txi.transaction_id().to_bytes() == elem_hash.to_bytes() && txi.index() == elem_index
            {
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
            .sort_by_cached_key(|k| from_bignum(&k.output().amount().coin()));
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
    pub fn sort_by_asset_amount(&mut self, policy: &ScriptHash, asset: &AssetName) {
        self.0.sort_by_cached_key(|k| get_amount(k, policy, asset));

        fn get_amount(
            k: &TransactionUnspentOutput,
            policy: &ScriptHash,
            asset: &AssetName,
        ) -> usize {
            match k.output().amount().multiasset() {
                Some(ma) => match ma.get(policy) {
                    Some(assets) => match assets.get(asset) {
                        Some(a) => from_bignum(&a) as usize,
                        None => 100,
                    },
                    None => 1000,
                },
                None => 2000,
            }
        }

        self.0.reverse()
    }

    pub fn optimize_on_assets(&mut self, assets: Tokens) -> Result<(), CSLCommonError> {
        let tot_val_utxos = self.calc_total_value()?;
        let needed_val = tokens_to_value(&assets);
        let overhead_value = tot_val_utxos.clamped_sub(&needed_val);
        if overhead_value.multiasset().is_some() {
            let mut overhead_tok = value_to_tokens(&overhead_value)?;
            let mut list_of_utxo_tokens = Vec::<(Tokens, TransactionUnspentOutput)>::new();
            for t in self.0.clone().iter() {
                list_of_utxo_tokens.push((value_to_tokens(&t.output().amount())?, t.clone()));
            }

            for tokens in &list_of_utxo_tokens {
                let diff = token_diff(&tokens.0, &overhead_tok);
                match diff.len() {
                    0 => {
                        continue;
                    }
                    _ => {
                        if let Some(k) = list_of_utxo_tokens.iter().find(|k| k.0 == diff) {
                            if let Some(pos) = self.find_utxo_index(&k.1) {
                                self.swap_remove(pos);
                                overhead_tok =
                                    value_to_tokens(&tot_val_utxos.clamped_sub(&needed_val))?;
                            }
                        }
                    }
                }
            }
            Ok(())
        } else {
            Ok(())
        }
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
            let ma = n.output().amount().multiasset().unwrap();
            let policies = ma.keys();
            for p in 0..policies.len() {
                let assets = ma.get(&policies.get(p)).unwrap();
                let ans = assets.keys();
                for a in 0..assets.keys().len() {
                    let n = assets.get(&ans.get(a)).unwrap();
                    out.push((policies.get(p), ans.get(a), n))
                }
            }
        });
        sum_unique_tokens(&out)
    }

    pub fn coin_sum(&self) -> u64 {
        //BigNum {
        let coinsum = self.0.iter().fold(0u64, |acc: u64, x| {
            acc + from_bignum(&x.output().amount().coin())
        });
        //to_bignum(coinsum)
        coinsum
    }

    pub fn coin_sum_minutxo(&self) -> u64 {
        //BigNum {
        let coinsum = self.0.iter().fold(0u64, |acc: u64, x| {
            acc + from_bignum(&x.output().amount().coin())
        });
        let minutxo = from_bignum(
            &calc_min_ada_for_utxo(
                &self
                    .calc_total_value()
                    .expect("calc total value crashed in coin_sum_minutxo"),
                None,
            )
            .unwrap(),
        );

        std::cmp::min(coinsum as i64 - minutxo as i64, 0) as u64
    }

    pub fn coin_value_subset(
        &self,
        ncoin: BigNum,
        already_in_use: Option<&TransactionUnspentOutputs>,
    ) -> TransactionUnspentOutputs {
        use itertools::FoldWhile::{Continue, Done};
        let i = itertools::Itertools::fold_while(
            &mut self.0.iter(),
            TransactionUnspentOutputs::new(),
            |mut acc: TransactionUnspentOutputs, x| {
                if acc.coin_sum() >= from_bignum(&ncoin) {
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
        ncoin: &BigNum,
        payaddr: &Address,
    ) -> TransactionUnspentOutputs {
        use itertools::FoldWhile::{Continue, Done};
        let i = itertools::Itertools::fold_while(
            &mut self.0.iter(),
            TransactionUnspentOutputs::new(),
            |mut acc: TransactionUnspentOutputs, x| {
                if acc.coin_sum_minutxo() >= from_bignum(ncoin) {
                    Done(acc)
                } else {
                    Continue({
                        let x_stake = get_stakeaddr_from_addr(&x.output().address()).expect(
                            "Could not determine stake address in coin_value_subset_minutxo 1",
                        );
                        let payer_stake = get_stakeaddr_from_addr(payaddr).expect(
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

    pub fn subset_address(&self, payaddr: &Address) -> TransactionUnspentOutputs {
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
        for i in 0..self.0.len() {
            let txi = self.0[i].input();
            if txi.transaction_id().to_bytes() == elem_hash.to_bytes() && txi.index() == elem_index
            {
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
            for j in 0..elems.len() {
                let txi_other = elems.get(j).input();
                if txi_self.transaction_id().to_bytes() == txi_other.transaction_id().to_bytes()
                    && txi_self.index() == txi_other.index()
                {
                    return true;
                }
            }
        }
        false
    }

    pub fn contains_address(&self, addr: Address) -> bool {
        for i in 0..self.0.len() {
            if self.0[i].output().address().to_bytes() == addr.to_bytes() {
                return true;
            }
        }
        false
    }

    pub fn filter_values(
        &self,
        value: &Value,
        band: Option<i8>,
    ) -> Result<TransactionUnspentOutputs, CSLCommonError> {
        let mut min = value.clone();
        let mut max = value.clone();
        if let Some(b) = band {
            (min, max) = TransactionUnspentOutputs::band_value(value, b);
        }
        println!("Band Values: Min: {:?}; Max: {:?}", min, max);
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

    fn band_value(value: &Value, band: i8) -> (Value, Value) {
        let coin = from_bignum(&value.coin());
        let mut min_val = Value::new(&to_bignum(coin - (coin / 100 * band as u64)));
        let mut max_val = Value::new(&to_bignum(coin + (coin / 100 * band as u64)));
        if value.multiasset().is_some() {
            let mut min_ma = csl::MultiAsset::new();
            let mut max_ma = csl::MultiAsset::new();
            let ma = value.multiasset().unwrap();
            for i in 0..ma.keys().len() {
                let policy = ma.keys().get(i);
                let assets = ma.get(&policy).unwrap();
                let mut min_assets = csl::Assets::new();
                let mut max_assets = csl::Assets::new();
                for j in 0..assets.keys().len() {
                    let asset = assets.keys().get(j);
                    let amt = from_bignum(&assets.get(&asset).unwrap());
                    let min_amt = to_bignum(amt - (amt / 100 * band as u64));
                    let max_amt = to_bignum(amt + (amt / 100 * band as u64));
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

    pub fn to_hex(&self) -> Result<String, CSLCommonError> {
        use cbor_event::{se::Serializer, Len};
        let mut serializer = Serializer::new_vec();
        serializer.write_array(Len::Len(self.len() as u64))?;

        for i in 0..self.len() {
            serializer.write_bytes(&self.0[i].to_bytes())?;
        }
        let bytes = serializer.finalize();
        Ok(hex::encode(bytes))
    }

    pub fn from_hex(str: &str) -> Result<TransactionUnspentOutputs, CSLCommonError> {
        use cbor_event::de::*;
        use std::io::Cursor;
        let vec = hex::decode(str)?;
        let mut raw = Deserializer::from(Cursor::new(vec));

        let mut v = TransactionUnspentOutputs::new();
        (|| -> Result<(), crate::CSLCommonError> {
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

    pub fn calc_total_value(&self) -> Result<Value, CSLCommonError> {
        let mut tval = Value::new(&to_bignum(0u64));
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
    ) -> Result<TransactionUnspentOutputs, CSLCommonError> {
        let policy = ScriptHash::from_bytes(hex::decode(&policy)?)?;
        let mut out = TransactionUnspentOutputs::new();
        self.0.iter().for_each(|n| {
            let ma = n.output().amount().multiasset();
            if let Some(multi) = ma {
                let policies = multi.get(&policy);
                match policies {
                    Some(_) => out.add(n),
                    None => {}
                }
            }
        });
        Ok(out)
    }
}
impl<'a> FromIterator<&'a TransactionUnspentOutput> for TransactionUnspentOutputs {
    fn from_iter<I: IntoIterator<Item = &'a TransactionUnspentOutput>>(iter: I) -> Self {
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

pub trait ARemove {
    fn aremove(&mut self, index: usize) -> csl::TransactionOutputs;
}

impl ARemove for csl::TransactionOutputs {
    fn aremove(&mut self, index: usize) -> csl::TransactionOutputs {
        let mut res = csl::TransactionOutputs::new();
        for tx in 0..self.len() {
            if tx != index {
                res.add(&self.get(tx));
            }
        }
        res
    }
}

pub fn sum_unique_tokens(tokens: &Tokens) -> Tokens {
    let mut out = Tokens::new();
    let mut shas = Vec::<(ScriptHash, AssetName)>::new();
    for t in tokens {
        if !shas.contains(&(t.0.to_owned(), t.1.to_owned())) {
            shas.push((t.0.to_owned(), t.1.to_owned()))
        }
    }

    for t in shas {
        let mut tos = tokens.clone();

        tos.retain(|n| n.0 == t.0 && n.1 == t.1);
        let f = tos.iter().fold(0, |acc, n| acc + from_bignum(&n.2));

        out.push((t.0, t.1, to_bignum(f)))
    }
    out
}

pub fn tokens_to_value(tokens: &Tokens) -> Value {
    let mut val = Value::new(&to_bignum(0u64));
    let mut ma = csl::MultiAsset::new();
    for token in tokens {
        match ma.get(&token.0) {
            Some(mut assets) => {
                assets.insert(&token.1, &token.2);
                ma.insert(&token.0, &assets);
            }
            None => {
                let mut assets = csl::Assets::new();
                assets.insert(&token.1, &token.2);
                ma.insert(&token.0, &assets);
            }
        }
    }
    val.set_multiasset(&ma);
    val
}

pub fn value_to_tokens(value: &Value) -> Result<Tokens, CSLCommonError> {
    let ma = match value.multiasset() {
        Some(multiassets) => multiassets,
        None => {
            return Err(CSLCommonError::Custom(
                "Value does not contain any multiassets cannot convert to 'Tokens'".to_string(),
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
    p: &ScriptHash,
    a: &AssetName,
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
                //from_bignum(&r) >= 0 {
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
        match find_tokenindex_by_policy_assetname(tok_r, &tok.0, &tok.1) {
            Some(s) => {
                let r = tok.2.clamped_sub(&tok_r[s].2);
                if from_bignum(&r) == 0 {
                    out.push((tok.0.clone(), tok.1.clone(), tok.2))
                }
            }
            None => {}
        }
    }

    out
}

pub fn token_diff(tok_l: &Tokens, tok_r: &Tokens) -> Tokens {
    let sub = check_overhead_tokens(tok_l, tok_r);
    acc_tokens(tok_l, &sub)
}

pub fn extract_assets(
    utxo: &TransactionUnspentOutput,
    policy: &String,
) -> Result<MultiAsset, CSLCommonError> {
    let mut out = MultiAsset::new();

    if let Some(multiassets) = utxo.output().amount().multiasset() {
        let sh = ScriptHash::from_bytes(hex::decode(policy)?)?;
        if let Some(assets) = multiassets.get(&sh) {
            out.insert(&sh, &assets);
        }
    }

    Ok(out)
}

pub type TokenAsset = (PolicyID, AssetName, BigNum);
pub type MintTokenAsset = (Option<PolicyID>, AssetName, BigNum);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CBORTransaction {
    r#type: String,
    description: String,
    #[serde(rename = "camelCase")]
    cbor_hex: String,
}

// Customer Payout
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CPO {
    user_id: i64,
    contract_id: i64,
    security_code: String,
}

impl CPO {
    pub fn new(user_id: &i64, contract_id: &i64, security_code: &str) -> Self {
        CPO {
            user_id: *user_id,
            contract_id: *contract_id,
            security_code: security_code.to_string(),
        }
    }

    pub fn get_user_id(&self) -> i64 {
        self.user_id
    }

    pub fn get_contract_id(&self) -> i64 {
        self.contract_id
    }

    pub fn get_security_code(&self) -> String {
        self.security_code.clone()
    }
}

pub async fn find_token_utxos(
    inputs: TransactionUnspentOutputs,
    assets: Vec<TokenAsset>,
) -> Result<TransactionUnspentOutputs, CSLCommonError> {
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
                            if needed_amt >= asset.2 && !out.contains_tx(&unspent_output) {
                                out.add(&unspent_output);
                                needed_amt = needed_amt.clamped_sub(&amt);
                            }
                        }
                    }
                }
            }
        }
    } else {
        return Err(CSLCommonError::Custom(
            "cannot find token utxos , one of the provided inputs is empty".to_owned(),
        ));
    }
    Ok(out)
}

pub fn find_token_utxos_na(
    inputs: &TransactionUnspentOutputs,
    assets: Vec<TokenAsset>,
    on_addr: Option<&Address>,
) -> Result<TransactionUnspentOutputs, CSLCommonError> {
    let mut out = TransactionUnspentOutputs::new();
    let ins = inputs.clone();
    if !inputs.is_empty() && !assets.is_empty() {
        for asset in assets.clone() {
            let mut needed_amt = asset.2;
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
                                if !out.contains_tx(&unspent_output) {
                                    out.add(&unspent_output);
                                    needed_amt = needed_amt.clamped_sub(&amt);
                                } else {
                                    needed_amt = needed_amt.clamped_sub(&amt);
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        return Err(CSLCommonError::Custom(
            "ERROR: cannot find token utxos , one of the provided inputs is empty".to_owned(),
        ));
    }

    if out.is_empty() {
        return Err(CSLCommonError::Custom(
            "ERROR: The token is not available in the utxo set".to_owned(),
        ));
    }

    out.optimize_on_assets(assets)?;
    Ok(out)
}

pub fn input_selection(
    token_utxos: Option<&TransactionUnspentOutputs>,
    needed_value: &mut Value,
    txins: &TransactionUnspentOutputs,
    collateral: Option<TransactionUnspentOutput>,
    on_addr: Option<&Address>,
) -> Result<(TransactionInputs, TransactionUnspentOutputs), CSLCommonError> {
    let (mut purecoinassets, mut multiassets) = splitt_coin_multi(txins);

    let mut nv = needed_value.clone();
    let mut selection = TransactionUnspentOutputs::new();
    let mut acc = Value::new(&to_bignum(0u64));
    let mut txins = TransactionInputs::new();

    let overhead = 50u64;

    if let Some(token_utxos) = token_utxos {
        for i in 0..token_utxos.len() {
            selection.add(&token_utxos.get(i));
            acc = acc
                .checked_add(&token_utxos.get(i).output().amount())
                .unwrap();
            nv = nv
                .checked_add(&token_utxos.get(i).output().amount())
                .unwrap();
            // Delete script input from multi assets
            if let Some(i) = multiassets.find_utxo_index(&token_utxos.get(i)) {
                multiassets.swap_remove(i);
            }
        }
    }

    if let Some(cutxo) = collateral {
        let c_index = find_collateral_by_txhash_txix(&cutxo, &purecoinassets);
        if let Some(index) = c_index {
            purecoinassets.swap_remove(index);
        }
    }

    // lookup tokens from needed value
    let mut tokens_to_find = Tokens::new();
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
        let token_selection = find_token_utxos_na(&multiassets.clone(), tokens_to_find, on_addr)?;
        if !token_selection.is_empty() {
            for i in 0..token_selection.len() {
                selection.add(&token_selection.get(i));
                acc = acc
                    .checked_add(&token_selection.get(i).output().amount())
                    .unwrap();
                // Delete script input from multi assets
                if let Some(i) = multiassets.find_utxo_index(&token_selection.get(i)) {
                    multiassets.swap_remove(i);
                }
            }
        }
    }

    multiassets.sort_by_coin();
    purecoinassets.sort_by_coin();

    let utxo_count = multiassets.len() + purecoinassets.len();
    let mut max_run = 0;

    while nv.coin().compare(&acc.coin()) > 0 && max_run < utxo_count {
        nv = nv.checked_sub(&acc).unwrap();

        if purecoinassets.is_empty() {
            // Find the tokens we want in the multis
            let ret = find_suitable_coins(&mut nv, &mut multiassets, overhead);
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
            let ret = find_suitable_coins(&mut nv, &mut purecoinassets, overhead);
            match ret.0 {
                Some(utxos) => {
                    for u in utxos {
                        selection.add(&u);
                    }
                    acc.set_coin(&acc.coin().checked_add(&to_bignum(ret.1)).unwrap());
                }
                None => {
                    return Err(CSLCommonError::Custom(
                        "ERROR: Not enough input utxos available to balance the transaction"
                            .to_owned(),
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
    Ok((txins, selection))
}

// Helper function for select coins
// Recursive apply minutxo until enough Ada is available
fn select_min_utxo_input_coins(
    paying_address: &Address,
    needed: &Value,
    selected_value: &mut Value,
    selected_inputs: &mut TransactionUnspentOutputs,
    avail_input_utxos: &mut TransactionUnspentOutputs,
) -> Result<(), CSLCommonError> {
    let diff = selected_value.checked_sub(needed)?;

    let change_min_utxo = calc_min_ada_for_utxo(&diff, None)?;

    if diff.coin().compare(&change_min_utxo) == -1 {
        let missing_coins = change_min_utxo.checked_sub(&diff.coin())?;
        let additional_utxos =
            avail_input_utxos.coin_value_subset_minutxo(&missing_coins, paying_address);
        avail_input_utxos.delete_set(&additional_utxos);
        selected_inputs.merge(additional_utxos);
        *selected_value = selected_inputs.calc_total_value()?;
        return select_min_utxo_input_coins(
            paying_address,
            needed,
            selected_value,
            selected_inputs,
            avail_input_utxos,
        );
    }
    Ok(())
}

pub fn select_coins(
    utxo_selection: &mut TransactionUnspentOutputs,
    input_utxos: &mut TransactionUnspentOutputs,
    needed: &Value,
    payer: &Address,
    own_address: &Address,
) -> Result<(), CSLCommonError> {
    let mut selected_value = utxo_selection.calc_total_value()?;

    if selected_value.coin().compare(&needed.coin()) == -1 {
        // Not enough Ada we need more
        let missing_coins = needed.coin().checked_sub(&selected_value.coin())?;
        let additional_inputs = input_utxos.coin_value_subset_minutxo(&missing_coins, payer);
        input_utxos.delete_set(&additional_inputs);
        utxo_selection.merge(additional_inputs);

        // make sure enough Ada is available
        select_min_utxo_input_coins(
            own_address,
            needed,
            &mut selected_value,
            utxo_selection,
            input_utxos,
        )?;
    }
    Ok(())
}

pub fn half_utxo(
    v: &TransactionOutput,
    inputs: &mut TransactionUnspentOutputs,
    paying_address: &Address,
) -> (TransactionOutputs, TransactionUnspentOutputs) {
    let mut one = Value::new(&to_bignum(0));
    let mut two = v.amount();
    let mut out = TransactionOutputs::new();
    let mut used_inputs = TransactionUnspentOutputs::new();

    let size_two = two.to_bytes().len();
    let mut multiasset_one = csl::MultiAsset::new();
    let mut multiasset_two = two
        .multiasset()
        .expect("Error: The TxOut to be halfed has no multiassets!");
    while one.to_bytes().len() < size_two / 2 {
        let mut temp = csl::MultiAsset::new();
        let policys = multiasset_two.keys();
        if let Some(assets) = multiasset_two.get(&policys.get(0)) {
            multiasset_one.insert(&policys.get(0), &assets);
            temp.insert(&policys.get(0), &assets);
        };
        multiasset_two = multiasset_two.sub(&temp);
    }
    one.set_multiasset(&multiasset_one);
    let min_utxo_one = calc_min_ada_for_utxo(&one, None).unwrap();
    two.set_multiasset(&multiasset_two);
    let min_utxo_two = calc_min_ada_for_utxo(&two, None).unwrap();

    let total_min_ada = min_utxo_one.checked_add(&min_utxo_two).unwrap();
    match two.coin().compare(&total_min_ada) {
        -1 => {
            // Not enough Ada for both min Utxos
            let missing_coins = total_min_ada.clamped_sub(&two.coin());
            let additional_inputs =
                inputs.coin_value_subset_minutxo(&missing_coins, paying_address);
            inputs.delete_set(&additional_inputs);
            used_inputs.merge(additional_inputs.clone());

            two.set_coin(&min_utxo_two);
            one.set_coin(&min_utxo_one);
            out.add(&TransactionOutput::new(&v.address(), &one));
            out.add(&TransactionOutput::new(&v.address(), &two));

            let mut tot_val = additional_inputs.calc_total_value().unwrap();
            tot_val = tot_val.checked_sub(&Value::new(&total_min_ada)).unwrap();
            let new_change = TransactionOutput::new(paying_address, &tot_val);
            if new_change.to_bytes().len() > 5000 {
                let next = half_utxo(&new_change, inputs, paying_address);
                for i in 0..next.0.len() {
                    out.add(&next.0.get(i));
                }
                inputs.merge(next.1);
            } else {
                out.add(&new_change);
            }
        }
        _ => {
            // Enough Ada
            two.set_coin(&two.coin().clamped_sub(&min_utxo_one));
            one.set_coin(&min_utxo_one);

            out.add(&TransactionOutput::new(&v.address(), &one));
            out.add(&TransactionOutput::new(&v.address(), &two));
        }
    }

    (out, used_inputs)
}

pub fn find_assets_in_value(v: &Value, a: &Vec<TokenAsset>) -> (bool, Value, Value) {
    let mut new_ma = csl::MultiAsset::new();
    let mut rest = csl::MultiAsset::new();
    let coin = v.coin();

    let mut new_val = Value::new(&to_bignum(0));
    let mut rest_val = Value::new(&to_bignum(0));

    let mut flag = false;

    if v.multiasset().is_none() {
        return (flag, new_val, rest_val);
    }
    let ma = v.multiasset().unwrap();
    for t in a {
        let amt = &ma.get_asset(&t.0, &t.1);
        if from_bignum(amt) > 0 {
            flag = true;
            let mut ramt = *amt;
            if amt.compare(&t.2) > 0 {
                ramt = amt.clamped_sub(&t.2);
            }

            let mut assets = csl::Assets::new();
            assets.insert(&t.1, &t.2);
            new_ma.insert(&t.0, &assets);

            let mut rassets = csl::Assets::new();
            rassets.insert(&t.1, &ramt);
            rest.insert(&t.0, &rassets);
        } else {
            let mut assets = csl::Assets::new();
            assets.insert(&t.1, &t.2);
            rest.insert(&t.0, &assets);
        }
    }

    new_val.set_multiasset(&new_ma);
    rest_val.set_multiasset(&rest);

    rest_val.set_coin(&coin);

    (flag, new_val, rest_val)
}

pub fn calc_min_ada_for_utxo(
    value: &Value,
    dh: Option<DataHash>,
) -> Result<BigNum, CSLCommonError> {
    //utxoEntrySize (txout) * coinsPerUTxOWord
    //utxoEntrySize (txout) = utxoEntrySizeWithoutVal + size (v) + dataHashSize (dh)
    let dhsize: u64 = match dh {
        Some(_) => 10u64, //(datumhash.to_bytes().len())  as u64
        None => 0u64,
    };

    let size = bundle_size(
        value,
        &OutputSizeConstants {
            k0: 2,
            k1: 6,
            k2: 12,
            k3: 1,
            _k4: 8,
        },
    );

    let utxo_entry_size_without_val = 27u64; //29
    let min_ada = to_bignum(dhsize + utxo_entry_size_without_val + size as u64)
        .checked_mul(&to_bignum(34482u64))
        .unwrap();
    //Limit max Val size
    let val_size = value.to_bytes().len();
    if val_size > 5000 {
        return Err(CSLCommonError::Custom("exceeded max value size".to_owned()));
    }

    Ok(min_ada)
}

pub fn bundle_size(value: &Value, osc: &OutputSizeConstants) -> usize {
    match &value.multiasset() {
        Some(assets) => {
            //Anzahl Tokens
            let mut num_assets: usize = 0;
            //AssetName Length
            let mut anl: usize = 0;
            // PolicyId Length
            let mut pil: usize = 0;

            let policy_ids = assets.keys();

            for policy in 0..policy_ids.len() {
                let pid = &policy_ids.get(policy);
                pil += pid.to_bytes().len();
                num_assets += assets.get(pid).unwrap().len();
                let tns = assets.get(&policy_ids.get(policy)).unwrap().keys();
                for tn in 0..tns.len() {
                    anl += tns.get(tn).name().len();
                }
            }

            fn roundup_bytes_to_words(b: usize) -> usize {
                quot(b + 7, 8)
            }

            osc.k1 + (roundup_bytes_to_words((num_assets * osc.k2) + anl + (osc.k3 * pil)))
        }

        None => osc.k0,
    }
}

pub fn quot<T>(a: T, b: T) -> T
where
    T: Sub<Output = T> + Rem<Output = T> + Div<Output = T> + Copy + Clone + std::fmt::Display,
{
    (a - (a % b)) / b
}

pub async fn create_and_submit_cbor_tx(
    tx: String,
    tx_hash: String,
) -> Result<String, CSLCommonError> {
    let cli_tx = CBORTransaction {
        r#type: "Tx BabbageEra".to_string(),
        description: "drasil transaction".to_string(),
        cbor_hex: tx,
    };
    submit_tx(&cli_tx, &tx_hash).await
}

pub async fn submit_endpoint(
    tx: &[u8],
    endpoint: String,
    own_tx_hash: &String,
    client: &reqwest::Client,
) -> Result<(String, String, bool), CSLCommonError> {
    use futures::pin_mut;

    let response = client
        .post(endpoint.clone())
        .header("Content-Type", "application/cbor")
        .body(tx.to_owned())
        .send();
    pin_mut!(response);

    match tokio::time::timeout(std::time::Duration::from_secs(5), &mut response).await {
        Err(_) => Ok((
            "".to_string(),
            format!("ERROR: '{:?}' is not available", endpoint),
            false,
        )),
        Ok(no_timeout) => match no_timeout {
            Ok(resp) => {
                let mut err = String::new();
                let mut txhash = String::new();
                let r_status = resp.status();
                let resp_text = resp.text().await?;
                if r_status != http::StatusCode::ACCEPTED {
                    err = format!("ERROR on tx submission: {:?}", resp_text);
                } else {
                    txhash = resp_text.replace('\"', "");
                }
                let assert = *own_tx_hash == txhash;

                Ok((txhash, err, assert))
            }
            Err(e) => Ok((
                "".to_string(),
                format!("ERROR: '{:?}' is not available", e),
                false,
            )),
        },
    }
}

pub async fn submit_tx(
    tx: &CBORTransaction,
    own_tx_hash: &String,
) -> Result<String, CSLCommonError> {
    let submit1 = std::env::var("TX_SUBMIT_ENDPOINT1")?;
    let submit2 = std::env::var("TX_SUBMIT_ENDPOINT2")?;
    let submit3 = std::env::var("TX_SUBMIT_ENDPOINT3")?;

    let client = reqwest::Client::new();
    let tx = hex::decode(tx.cbor_hex.clone())?;
    let mut response1 = (String::new(), String::new(), false);
    match submit_endpoint(&tx, submit1, own_tx_hash, &client).await {
        Ok(x) => response1 = x,
        Err(e) => {
            error!("Error: '{}'", e.to_string())
        }
    };

    let mut response2 = (String::new(), String::new(), false);
    match submit_endpoint(&tx, submit2, own_tx_hash, &client).await {
        Ok(x) => response2 = x,
        Err(e) => {
            error!("Error: '{}'", e.to_string())
        }
    };

    let mut response3 = (String::new(), String::new(), false);
    match submit_endpoint(&tx, submit3, own_tx_hash, &client).await {
        Ok(x) => response3 = x,
        Err(e) => {
            error!("Error: '{}'", e.to_string())
        }
    };

    if response1.2 || response2.2 || response3.2 {
        Ok(own_tx_hash.clone())
    } else {
        Err(CSLCommonError::Custom(
            response1.1 + &response2.1 + &response3.1,
        ))
    }
}

pub fn harden(num: u32) -> u32 {
    0x80000000 + num
}

pub fn get_input_position(
    inputs: csl::TransactionInputs,
    elem: TransactionUnspentOutput,
) -> (usize, Vec<TransactionHash>) {
    let mut my_index = Vec::<TransactionHash>::new();
    for i in 0..inputs.len() {
        my_index.push(inputs.get(i).transaction_id());
    }

    my_index.sort();
    let index = my_index
        .iter()
        .enumerate()
        .find(|&r| r.1 == &elem.input().transaction_id())
        .unwrap()
        .0;

    (index, my_index)
}

pub fn split_value(value: Value) -> Result<(Vec<Value>, Option<BigNum>), CSLCommonError> {
    let coins = value.coin();
    let mut val_coins = to_bignum(0);
    let val_tok = value_to_tokens(&value)?;
    let mut values = Vec::<Value>::new();
    for tok in val_tok {
        let mut value = tokens_to_value(&[tok].to_vec());
        let min_utxo_val = calc_min_ada_for_utxo(&value, None)?;
        val_coins = val_coins.checked_add(&min_utxo_val)?;
        value.set_coin(&min_utxo_val);
        values.push(value);
    }

    match coins.compare(&val_coins) {
        k if k > 0 => {
            let c = values[0].coin();
            let diff = coins.checked_sub(&val_coins)?;
            values[0].set_coin(&c.checked_add(&diff)?);
            Ok((values, None))
        }
        k if k < 0 => {
            // More Ada needed
            let diff = val_coins.checked_sub(&coins)?;
            Ok((values, Some(diff)))
        }
        _ => Ok((values, None)),
    }
}

pub fn minimize_coins_on_values(values: Vec<Value>) -> Result<Vec<Value>, CSLCommonError> {
    let mut out = Vec::<Value>::new();
    let ada = values
        .iter()
        .fold(Value::new(&to_bignum(0)), |mut acc: Value, x: &Value| {
            let c = x.coin();
            let mut xc = x.clone();
            let muv = calc_min_ada_for_utxo(x, None).unwrap();
            if c.compare(&muv) > 0 {
                let diff = c.checked_sub(&muv).unwrap();
                xc.set_coin(&muv);
                acc.set_coin(&acc.coin().checked_add(&diff).unwrap());
                out.push(xc);
                acc
            } else {
                out.push(xc);
                acc
            }
        });
    if from_bignum(&ada.coin()) > 0 {
        out.push(ada);
    }
    Ok(out)
}

pub fn splitt_coin_multi(
    txins: &TransactionUnspentOutputs,
) -> (TransactionUnspentOutputs, TransactionUnspentOutputs) {
    let mut ada_only = TransactionUnspentOutputs::new();
    let mut multi = TransactionUnspentOutputs::new();

    for tx in txins.clone() {
        match tx.output().amount().multiasset() {
            Some(x) => {
                if x.len() > 0 {
                    multi.add(&tx)
                } else {
                    ada_only.add(&tx)
                }
            }
            None => ada_only.add(&tx),
        }
    }
    ada_only.sort_by_coin();
    multi.sort_by_multi_amount();

    (ada_only, multi)
}

pub fn find_collateral_by_txhash_txix(
    elem: &TransactionUnspentOutput,
    txuos: &TransactionUnspentOutputs,
) -> Option<usize> {
    let col_max = to_bignum(20000000u64);
    let elem_hash = elem.input().transaction_id();
    let elem_index = elem.input().index();
    for i in 0..txuos.len() {
        let txi = txuos.get(i).input();
        if txi.transaction_id().to_bytes() == elem_hash.to_bytes() && txi.index() == elem_index {
            if txuos.get(i).output().amount().coin().compare(&col_max) <= 0 {
                return Some(i);
            } else {
                return None;
            }
        }
    }
    None
}

pub fn find_suitable_coins(
    nv: &mut Value,
    inputs: &mut TransactionUnspentOutputs,
    overhead: u64,
) -> (Option<TransactionUnspentOutputs>, u64) {
    let coins = from_bignum(&nv.coin());
    let max_coins = coins + (coins / 100 * overhead as u64); // Coins + Overhead in %

    let mut acc = 0u64;
    let mut selection = TransactionUnspentOutputs::new();
    let mut multi_storage = TransactionUnspentOutputs::new();
    let mut coin_storage = TransactionUnspentOutputs::new();

    'outer: for tx in inputs.clone() {
        let lc = from_bignum(&tx.output().amount().coin());
        if lc > coins {
            match tx.output().amount().multiasset() {
                Some(multi) => match multi.len() {
                    0 => {
                        if lc < max_coins {
                            selection.add(&tx);
                            return (Some(selection), lc);
                        } else {
                            coin_storage.add(&tx);
                        }
                    }

                    1..=21 => {
                        selection.add(&tx);
                        return (Some(selection), lc);
                    }
                    _ => {
                        multi_storage.add(&tx);
                    }
                },

                None => {
                    if lc < max_coins {
                        selection.add(&tx);
                        return (Some(selection), lc);
                    } else {
                        coin_storage.add(&tx);
                    }
                }
            }
        }
        if lc <= coins {
            if !coin_storage.is_empty() {
                coin_storage.sort_by_coin();
                let tx = coin_storage.get(0);
                selection.add(&tx);
                acc = from_bignum(&tx.output().amount().coin());
                return (Some(selection), acc);
            }
            break 'outer;
        }
    }
    if !coin_storage.is_empty() {
        coin_storage.sort_by_coin();
        let tx = coin_storage.get(0);
        selection.add(&tx);
        acc = from_bignum(&tx.output().amount().coin());
        return (Some(selection), acc);
    } else {
        for tx in inputs {
            let lc = from_bignum(&tx.output().amount().coin());
            acc += lc;
            selection.add(&tx);
            if acc > coins + MIN_ADA {
                return (Some(selection), acc);
            }
        }
    }

    if selection.is_empty() {
        (None, 0)
    } else {
        if !multi_storage.is_empty() {
            let mut selection = TransactionUnspentOutputs::new();
            multi_storage.sort_by_multi_amount();
            let tx = multi_storage.get(0);
            selection.add(&tx);
            acc = from_bignum(&tx.output().amount().coin());
        }
        (Some(selection), acc)
    }
}

pub fn blake2b160(data: &[u8]) -> [u8; 20] {
    //Vec::<u8> {
    let mut out = [0u8; 20];
    let mut context = Blake2b::new(20);
    context.input(data);
    context.result(&mut out);
    Blake2b::blake2b(&mut out, data, &[]);
    out
}

pub fn make_fingerprint(p: &String, a: &String) -> Result<String, CSLCommonError> {
    let policy = hex::decode(p)?;
    let tn = hex::decode(a)?;
    let data = [&policy[..], &tn[..]].concat();
    let hash = blake2b160(&data);
    let fingerprint = bech32::Bech32::new("asset".to_string(), hash.to_base32()).unwrap();
    Ok(fingerprint.to_string())
}

pub fn get_network_from_address(address: &str) -> Result<csl::NetworkIdKind, CSLCommonError> {
    let addr: Address = addr_from_str(address)?;
    match addr.network_id()? {
        1 => Ok(csl::NetworkIdKind::Mainnet),
        _ => Ok(csl::NetworkIdKind::Testnet),
    }
}

pub fn get_vkey_count(
    txuos: &TransactionUnspentOutputs,
    col: Option<&TransactionUnspentOutput>,
) -> usize {
    // Check for Number of Vkeys in the signature
    let mut vkey_counter = 0usize;
    let mut addresses = Vec::<std::vec::Vec<u8>>::new();
    for txi in 0..txuos.len() {
        if !addresses.contains(&txuos.get(txi).output().address().to_bytes()) {
            vkey_counter += 1;
            addresses.push(txuos.get(txi).output().address().to_bytes());
        }
    }
    match col {
        Some(c) => {
            if !txuos.contains_address(c.output().address()) {
                vkey_counter += 1;
            }
        }
        None => {}
    }
    vkey_counter
}

pub fn make_dummy_vkeywitnesses(vkey_count: usize) -> csl::crypto::Vkeywitnesses {
    let mut dummy_vkeywitnesses = csl::crypto::Vkeywitnesses::new();
    let vkeywitness =
        csl::crypto::Vkeywitness::from_bytes(hex::decode(DUMMY_VKEYWITNESS).unwrap()).unwrap();
    for _ in 0..vkey_count {
        dummy_vkeywitnesses.add(&vkeywitness);
    }

    dummy_vkeywitnesses
}

pub fn find_utxos_by_address(
    addr: Address,
    txuos: &TransactionUnspentOutputs,
) -> (TransactionUnspentOutputs, TransactionUnspentOutputs) {
    let mut addr_utxos = TransactionUnspentOutputs::new();
    let mut other_utxos = TransactionUnspentOutputs::new();

    for tx in txuos.clone() {
        if tx.output().address().to_bytes() == addr.to_bytes() {
            addr_utxos.add(&tx);
        } else {
            other_utxos.add(&tx);
        }
    }
    addr_utxos.sort_by_multi_amount();
    other_utxos.sort_by_coin();

    (addr_utxos, other_utxos)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExUnitPrice {
    #[serde(rename = "priceSteps")]
    pub price_steps: f64,
    #[serde(rename = "priceMemory")]
    pub price_memory: f64,
}

pub fn calc_txfee(
    tx: &csl::Transaction,
    a: &BigNum,
    b: &BigNum,
    ex_unit_price: ExUnitPrice,
    steps: &BigNum,
    mem: &BigNum,
    no_sc: bool,
) -> BigNum {
    let txsfee = tx_script_fee(ex_unit_price, from_bignum(steps), from_bignum(mem));
    let linearfee = csl::fees::LinearFee::new(a, b);
    let base_fee = csl::fees::min_fee(&tx.clone(), &linearfee).unwrap();
    let mut calculated_fee = base_fee.checked_add(&to_bignum(txsfee)).unwrap();

    if no_sc {
        calculated_fee = base_fee;
    }

    calculated_fee
}

pub fn tx_script_fee(ex_unit_price: ExUnitPrice, steps: u64, mem: u64) -> u64 {
    let tx_script_fee =
        (ex_unit_price.price_memory * mem as f64) + (ex_unit_price.price_steps * steps as f64);
    tx_script_fee.ceil() as u64
}
