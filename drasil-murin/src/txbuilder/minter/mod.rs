pub mod build_minttx;
pub mod build_oneshot_mint;
pub mod models;

use std::str;

use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr, utils as cutils};
use chrono::{DateTime, Utc};
use clib::crypto::{Ed25519KeyHash, ScriptHash};
use clib::metadata::{MetadataList, MetadataMap};
use clib::{NativeScript, NativeScripts, ScriptAll, ScriptPubkey};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::PerformTxb;
use crate::wallet;
use crate::MurinError;
use crate::{MintTokenAsset, TokenAsset};

#[derive(Debug, Clone)]
pub struct MinterTxData {
    mint_tokens: Vec<MintTokenAsset>,
    receiver_stake_addr: Option<caddr::Address>,
    receiver_payment_addr: caddr::Address,
    mint_metadata: Cip25Metadata,
    auto_mint: bool,
    fee_addr: Option<caddr::Address>,
    fee: Option<i64>,
    contract_id: i64,
}

impl MinterTxData {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mint_tokens: Vec<MintTokenAsset>,
        receiver_stake_addr: Option<caddr::Address>,
        receiver_payment_addr: caddr::Address,
        mint_metadata: Cip25Metadata,
        auto_mint: bool,
        fee_addr: Option<caddr::Address>,
        fee: Option<i64>,
        contract_id: i64,
    ) -> MinterTxData {
        MinterTxData {
            mint_tokens,
            receiver_stake_addr,
            receiver_payment_addr,
            mint_metadata,
            auto_mint,
            fee_addr,
            fee,
            contract_id,
        }
    }

    pub fn get_mint_tokens(&self) -> Vec<MintTokenAsset> {
        self.mint_tokens.clone()
    }

    pub fn get_stake_addr(&self) -> caddr::Address {
        match self.receiver_stake_addr.clone() {
            Some(addr) => addr,
            None => wallet::reward_address_from_address(&self.get_payment_addr()).unwrap(),
        }
    }

    pub fn get_stake_addr_bech32(&self) -> Result<Option<String>, MurinError> {
        if let Some(sa) = &self.receiver_stake_addr {
            return Ok(Some(sa.to_bech32(None)?));
        };

        Ok(None)
    }

    pub fn get_payment_addr(&self) -> caddr::Address {
        self.receiver_payment_addr.clone()
    }

    pub fn get_payment_addr_bech32(&self) -> Result<String, MurinError> {
        Ok(self.receiver_payment_addr.to_bech32(None)?)
    }

    pub fn get_metadata(&self) -> Cip25Metadata {
        self.mint_metadata.clone()
    }

    pub fn get_auto_mint(&self) -> bool {
        self.auto_mint
    }

    pub fn get_fee_addr(&self) -> Option<caddr::Address> {
        self.fee_addr.clone()
    }

    pub fn get_contract_id(&self) -> i64 {
        self.contract_id
    }

    pub fn get_fee(&self) -> Option<i64> {
        self.fee
    }

    pub fn set_claim_addr(&mut self, addr: caddr::Address) {
        self.receiver_payment_addr = addr;
    }

    pub fn set_mint_tokens(&mut self, mint_tokens: Vec<MintTokenAsset>) {
        self.mint_tokens = mint_tokens;
    }

    pub fn set_metadata(&mut self, metadata: Cip25Metadata) {
        self.mint_metadata = metadata;
    }

    pub fn set_fee_addr(&mut self, addr: caddr::Address) {
        self.fee_addr = Some(addr);
    }

    pub fn set_fee(&mut self, fee: i64) {
        self.fee = Some(fee);
    }
}

impl ToString for MinterTxData {
    fn to_string(&self) -> String {
        // prepare tokens vector
        let mut s_tokens = String::new();
        for ta in self.get_mint_tokens() {
            let mut subs = String::new();
            if let Some(p) = ta.0 {
                subs.push_str(&(hex::encode(p.to_bytes()) + "?"));
            } else {
                subs.push_str(&(("NoData".to_string()) + "?"));
            }

            subs.push_str(&(hex::encode(ta.1.to_bytes()) + "?"));
            subs.push_str(&(hex::encode(ta.2.to_bytes()) + "!"));
            s_tokens.push_str(&subs);
        }
        // erase last !
        s_tokens.pop();

        // prepare stake address
        let s_stake_addr = hex::encode(self.get_stake_addr().to_bytes());

        // prepare payment address
        let s_payment_addr = hex::encode(self.get_payment_addr().to_bytes());

        // prepare stake address
        let s_fee_addr = match self.get_fee_addr() {
            Some(addr) => hex::encode(addr.to_bytes()),
            None => "NoData".to_string(),
        };

        // prepare stake address
        let s_fee = match self.get_fee() {
            Some(fee) => fee.to_string(),
            None => "NoData".to_string(),
        };

        let mut ret = String::new();
        ret.push_str(&(s_tokens + "|"));
        ret.push_str(&(s_stake_addr + "|"));
        ret.push_str(&(s_payment_addr + "|"));
        ret.push_str(
            &(serde_json::to_string(&self.get_metadata())
                .expect("ERROR: Could not serialize metadata string")
                + "|"),
        );
        ret.push_str(&(self.get_auto_mint().to_string() + "|"));
        ret.push_str(&(s_fee_addr + "|"));
        ret.push_str(&(s_fee + "|"));
        ret.push_str(&self.contract_id.to_string());

        ret
    }
}

impl std::str::FromStr for MinterTxData {
    type Err = MurinError;
    fn from_str(src: &str) -> std::result::Result<Self, Self::Err> {
        let slice: Vec<&str> = src.split('|').collect();
        if slice.len() == 8 {
            // restore token vector
            let mut tokens = Vec::<MintTokenAsset>::new();
            let tokens_vec: Vec<&str> = slice[0].split('!').collect();
            for token in tokens_vec {
                let token_slice: Vec<&str> = token.split('?').collect();
                let p = match token_slice[0] {
                    "NoData" => None,
                    _ => Some(clib::PolicyID::from_bytes(hex::decode(token_slice[0])?)?),
                };
                tokens.push((
                    p,
                    clib::AssetName::from_bytes(hex::decode(token_slice[1])?)?,
                    cutils::BigNum::from_bytes(hex::decode(token_slice[2])?)?,
                ))
            }

            let stake_address = match slice[1] {
                "NoData" => None,
                _ => Some(caddr::Address::from_bytes(hex::decode(slice[1])?)?),
            };

            // restore payment address
            let payment_address = caddr::Address::from_bytes(hex::decode(slice[2])?)?;

            // restore fee
            debug!("restore fee");
            let metadata: Cip25Metadata = serde_json::from_str(slice[3])
                .expect("ERORR: Could not deserialize metadata from string");

            let auto_mint = bool::from_str(slice[4])?;

            let fee_addr = match slice[5] {
                "NoData" => None,
                _ => Some(caddr::Address::from_bytes(hex::decode(slice[5])?)?),
            };

            let fee = match slice[6] {
                "NoData" => None,
                _ => Some(slice[6].parse::<i64>()?),
            };

            let contract_id = slice[7].parse::<i64>()?;

            Ok(MinterTxData {
                mint_tokens: tokens,
                receiver_stake_addr: stake_address,
                receiver_payment_addr: payment_address,
                mint_metadata: metadata,
                auto_mint,
                fee_addr,
                fee,
                contract_id,
            })
        } else {
            Err(MurinError::new(&format!(
                "Error the provided string '{src}' cannot be parsed into 'RWDTxData' "
            )))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataOther {
    pub key: String,
    pub value: String, //serde_json::Value,
}

impl MetadataOther {
    pub fn from_json(json: &serde_json::Value, key: &str) -> Vec<MetadataOther> {
        vec![MetadataOther {
            key: key.to_string(),
            value: json.to_string(),
        }]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Source {
    String(String),
    Vec(Vec<String>),
}

impl Source {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataFile {
    pub name: String,
    #[serde(rename(serialize = "mediaType", deserialize = "mediaType"))]
    pub media_type: String,
    pub src: serde_json::Value,
    pub other: Option<Vec<MetadataOther>>,
}

impl MetadataFile {
    pub fn from_json(json: &serde_json::Value) -> Result<Vec<MetadataFile>, MurinError> {
        match json {
            serde_json::Value::Object(_) => {
                let n: MetadataFile = serde_json::from_value(json.clone())?;
                Ok(vec![n])
            }
            serde_json::Value::Array(arr) => {
                Ok(arr.iter().fold(Vec::<MetadataFile>::new(), |mut acc, n| {
                    log::debug!(
                        "Value to hand over recursivley to Metadatafile::from_json:\n {:?}",
                        n
                    );
                    acc.push(serde_json::from_value::<MetadataFile>(n.clone()).unwrap());
                    acc
                }))
            }
            _ => Err(MurinError::new("Not a valid cip25 file format")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMetadata {
    pub name: Option<String>,
    pub tokenname: String,
    #[serde(rename(serialize = "mediaType", deserialize = "mediaType"))]
    pub media_type: Option<String>,
    pub description: Option<Vec<String>>,
    pub image_url: Option<String>,
    pub files: Option<Vec<MetadataFile>>,
    pub other: Option<Vec<MetadataOther>>,
}

impl AssetMetadata {
    pub fn from_json(str: &str) -> Result<Vec<AssetMetadata>, MurinError> {
        let raw: serde_json::Value = serde_json::from_str(str)?;
        log::debug!("Read Raw Json: {:?}", raw);
        let mut assets = Vec::<AssetMetadata>::new();
        if let Some(o) = raw.as_object() {
            log::debug!("Some Assets found: {:?}", o);
            let mut a = AssetMetadata {
                tokenname: "".to_owned(),
                name: None,
                media_type: None,
                description: None,
                image_url: None,
                files: None,
                other: None,
            };
            let m = o
                .iter()
                .fold(Vec::<(&String, &Value)>::new(), |mut acc, n| {
                    acc.push(n);
                    acc
                });
            log::debug!("Key Values: {:?}", m);
            for elem in m.iter() {
                log::debug!("Element: {:?}", elem);
                match &elem.0[..] {
                    "name" => match elem.1 {
                        Value::String(s) => match hex::decode(s) {
                            Ok(v) => {
                                a.tokenname = hex::encode(v.clone());
                                a.name =
                                    Some(String::from_utf8(v.clone()).unwrap_or(hex::encode(v)));
                            }
                            Err(_e) => {
                                a.tokenname = hex::encode(s.as_bytes());
                                a.name = Some(s.to_string());
                            }
                        },
                        _ => {
                            return Err(MurinError::new("name is not a string"));
                        }
                    },
                    "image" => {
                        a.image_url = match elem.1 {
                            Value::String(s) => Some(s.to_string()),
                            Value::Array(s) => match s.len() {
                                0 => None,
                                // ToDo: allow also arrays and transform them correctly
                                _ => Some(s[0].to_string()),
                            },
                            _ => None,
                        }
                    }
                    "mediaType" => {
                        a.media_type = match elem.1 {
                            Value::String(s) => Some(s.to_string()),
                            _ => {
                                return Err(MurinError::new("mediaType value is not a string"));
                            }
                        }
                    }
                    "description" => {
                        //ToDO: Make also string descriptions possible without vec
                        a.description = {
                            match elem.1 {
                                Value::String(s) => Some(vec![s.to_string()]),
                                Value::Array(s) => match s.len() {
                                    0 => None,
                                    _ => Some(s.iter().map(|s| s.to_string()).collect()),
                                },
                                _ => None,
                            }
                        };
                    }
                    "files" => a.files = Some(MetadataFile::from_json(elem.1)?),
                    _ => {
                        log::debug!("SubElement Other: {:?}", elem);
                        if let Some(other) = a.clone().other {
                            let mut w = other.clone();
                            w.extend(MetadataOther::from_json(elem.1, elem.0).into_iter());
                            a.other = Some(w);
                        } else {
                            a.other = Some(MetadataOther::from_json(elem.1, elem.0));
                        }
                    }
                }
            }
            assets.push(a);
        }
        Ok(assets)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cip25Metadata {
    pub assets: Vec<AssetMetadata>,
    pub other: Option<Vec<MetadataOther>>,
    pub version: String,
}

impl Default for Cip25Metadata {
    fn default() -> Self {
        Self::new()
    }
}

impl Cip25Metadata {
    pub fn new() -> Self {
        Cip25Metadata {
            assets: Vec::<AssetMetadata>::new(),
            other: None,
            version: "2.0".to_string(),
        }
    }
}

pub fn mintasset_into_tokenasset(m: Vec<MintTokenAsset>, p: clib::PolicyID) -> Vec<TokenAsset> {
    let mut out = Vec::<TokenAsset>::new();
    for mta in m {
        let mut policy = p.clone();
        if let Some(cs) = mta.0 {
            policy = cs;
        }
        out.push((policy, mta.1, mta.2))
    }
    out.reverse();
    out
}

pub fn make_mint_metadata(
    raw_metadata: &Cip25Metadata,
    policy_id: clib::PolicyID,
) -> std::result::Result<clib::metadata::GeneralTransactionMetadata, MurinError> {
    pub use clib::metadata::*;

    let policy_str = hex::encode(policy_id.to_bytes());
    let mut toplevel_metadata = clib::metadata::GeneralTransactionMetadata::new();

    let mut metamap = clib::metadata::MetadataMap::new();
    let mut assetmap = MetadataMap::new();

    for asset in &raw_metadata.assets {
        make_721_asset_entry(asset, &mut assetmap)?;
    }
    log::debug!("\nAssetmap: {:?}\n", assetmap);
    let metadatum = clib::metadata::TransactionMetadatum::new_map(&assetmap);
    metamap.insert_str(&policy_str, &metadatum)?;
    metamap.insert_str(
        "version",
        &clib::metadata::TransactionMetadatum::new_text(raw_metadata.version.clone())?,
    )?;

    // Other
    if let Some(other) = &raw_metadata.other {
        encode_other_metadata(&mut metamap, other)?
    }

    let metadata = clib::metadata::TransactionMetadatum::new_map(&metamap);
    toplevel_metadata.insert(&cutils::to_bignum(721u64), &metadata);

    Ok(toplevel_metadata)
}

fn chunk_string(metamap: &mut MetadataMap, key: &str, s: &String) -> Result<(), MurinError> {
    if s.len() > 64 {
        let chunks = s
            .as_bytes()
            .chunks(64)
            .map(|c| String::from(str::from_utf8(c).unwrap_or(&hex::encode(c))))
            .collect::<Vec<String>>();

        log::debug!("Chunks: {:?}", chunks);
        let mut list = MetadataList::new();
        for s in chunks {
            list.add(&clib::metadata::TransactionMetadatum::new_text(s)?)
        }
        metamap.insert_str(key, &clib::metadata::TransactionMetadatum::new_list(&list))?;
    } else {
        metamap.insert_str(
            key,
            &clib::metadata::TransactionMetadatum::new_text(s.to_string())?,
        )?;
    }
    Ok(())
}

fn encode_object(v: &serde_json::Value) -> Result<MetadataMap, MurinError> {
    let mut map = MetadataMap::new();
    let o = v.as_object().unwrap();
    for e in o.iter() {
        match e.1 {
            Value::Null => {
                map.insert_str(
                    e.0,
                    &clib::metadata::TransactionMetadatum::new_text("".to_string())?,
                )?;
            }
            Value::Bool(b) => {
                map.insert_str(
                    e.0,
                    &clib::metadata::TransactionMetadatum::new_int(&clib::utils::Int::new_i32(
                        *b as i32,
                    )),
                )?;
            }
            Value::Number(b) => {
                map.insert_str(
                    e.0,
                    &clib::metadata::TransactionMetadatum::new_int(&clib::utils::Int::new_i32(
                        b.as_i64().unwrap() as i32,
                    )),
                )?;
            }
            Value::String(b) => {
                map.insert_str(
                    e.0,
                    &clib::metadata::TransactionMetadatum::new_text(b.to_string())?,
                )?;
            }
            Value::Array(_) => {
                let array = encode_array(e.1)?;
                map.insert_str(e.0, &clib::metadata::TransactionMetadatum::new_list(&array))?;
            }
            Value::Object(_) => {
                let object = encode_object(e.1)?;
                map.insert_str(e.0, &clib::metadata::TransactionMetadatum::new_map(&object))?;
            }
        }
    }
    Ok(map)
}

fn encode_array(v: &serde_json::Value) -> Result<MetadataList, MurinError> {
    let mut olist = MetadataList::new();
    for e in v.as_array().unwrap() {
        match e {
            Value::Null => {
                olist.add(&clib::metadata::TransactionMetadatum::new_text(
                    "".to_string(),
                )?);
            }
            Value::Bool(b) => {
                olist.add(&clib::metadata::TransactionMetadatum::new_int(
                    &clib::utils::Int::new_i32(*b as i32),
                ));
            }
            Value::Number(n) => {
                olist.add(&clib::metadata::TransactionMetadatum::new_int(
                    &clib::utils::Int::new_i32(n.as_i64().unwrap() as i32),
                ));
            }
            Value::String(s) => {
                olist.add(&clib::metadata::TransactionMetadatum::new_text(
                    s.to_string(),
                )?);
            }
            Value::Array(_) => {
                let array = encode_array(e)?;
                olist.add(&clib::metadata::TransactionMetadatum::new_list(&array))
            }
            Value::Object(_) => {
                let object = encode_object(e)?;
                olist.add(&clib::metadata::TransactionMetadatum::new_map(&object));
            }
        }
    }
    Ok(olist)
}

fn encode_other_metadata(
    metamap: &mut MetadataMap,
    other: &[MetadataOther],
) -> Result<(), MurinError> {
    for o in other {
        let v: serde_json::Value = serde_json::from_str(&o.value)?;
        if !v.is_null() {
            if v.is_i64() {
                metamap.insert_str(
                    &o.key,
                    &clib::metadata::TransactionMetadatum::new_int(&clib::utils::Int::new_i32(
                        v.as_i64().unwrap() as i32,
                    )),
                )?;
            }

            if v.is_string() {
                let s = o.value.to_string();
                chunk_string(metamap, &o.key, &s)?
            }

            if v.is_array() {
                let array = encode_array(&v)?;
                metamap.insert_str(
                    &o.key,
                    &clib::metadata::TransactionMetadatum::new_list(&array),
                )?;
            }

            if v.is_boolean() {
                metamap.insert_str(
                    &o.key,
                    &clib::metadata::TransactionMetadatum::new_int(&clib::utils::Int::new_i32(
                        v.as_i64().unwrap() as i32,
                    )),
                )?;
            }

            if v.is_object() {
                chunk_string(metamap, &o.key, &v.to_string())?;
            }

            if v.is_number() {
                metamap.insert_str(
                    &o.key,
                    &clib::metadata::TransactionMetadatum::new_int(&clib::utils::Int::new_i32(
                        v.as_i64().unwrap() as i32,
                    )),
                )?;
            }
        } else {
            chunk_string(metamap, &o.key, &o.value.to_string())?
        }
    }
    Ok(())
}

pub fn make_mint_metadata_from_json(
    raw_metadata: &Cip25Metadata,
    tokens: Vec<TokenAsset>,
    policy_id: clib::PolicyID,
) -> std::result::Result<clib::metadata::GeneralTransactionMetadata, MurinError> {
    pub use clib::metadata::*;

    let policy_str = hex::encode(policy_id.to_bytes());
    let mut toplevel_metadata = clib::metadata::GeneralTransactionMetadata::new();

    debug!("RawMetadata: {:?}", raw_metadata);

    // Check if all tokens have metadata available
    let mut i = 0;
    'avail_tok: for token in tokens.clone() {
        let t_name = str::from_utf8(&token.1.name())
            .unwrap_or(&hex::encode(token.1.name()))
            .to_string();
        debug!("TName: {}", t_name);
        for asset in raw_metadata.assets.clone() {
            if asset.tokenname == t_name {
                i += 1;
                continue 'avail_tok;
            }
        }
    }
    if tokens.len() != i {
        return Err(MurinError::new(&format!("Error provided metadata and tokens to mint are not fitting, please provide correct metadata: \n {raw_metadata:?}")));
    }

    let mut metamap = clib::metadata::MetadataMap::new();
    let mut assetmap = MetadataMap::new();
    for asset in &raw_metadata.assets {
        make_721_asset_entry(asset, &mut assetmap)?;
    }
    let metadatum = clib::metadata::TransactionMetadatum::new_map(&assetmap);
    metamap.insert_str(&policy_str, &metadatum)?;
    metamap.insert_str(
        "version",
        &clib::metadata::TransactionMetadatum::new_text(raw_metadata.version.clone())?,
    )?;

    // Other
    if let Some(other) = &raw_metadata.other {
        encode_other_metadata(&mut metamap, other)?
    }

    let metadata = clib::metadata::TransactionMetadatum::new_map(&metamap);
    toplevel_metadata.insert(&cutils::to_bignum(721u64), &metadata);

    Ok(toplevel_metadata)
}

pub fn make_721_asset_entry(
    asset: &AssetMetadata,
    assetmap: &mut clib::metadata::MetadataMap,
) -> std::result::Result<(), MurinError> {
    pub use clib::metadata::*;

    let mut asset_metadata = MetadataMap::new();

    if let Some(name) = &asset.name {
        asset_metadata.insert_str(
            "name",
            &clib::metadata::TransactionMetadatum::new_text(name.clone())?,
        )?;
    }
    if let Some(image_url) = &asset.image_url {
        asset_metadata.insert_str(
            "image",
            &clib::metadata::TransactionMetadatum::new_text(image_url.clone())?,
        )?;
        asset_metadata.insert_str(
            "mediaType",
            &clib::metadata::TransactionMetadatum::new_text(
                asset
                    .media_type
                    .clone()
                    .expect("If an image url is provided a mediaType is mandatory"),
            )?,
        )?;
    }

    //Description
    let mut desc_array = MetadataList::new();
    if let Some(descritpion) = &asset.description {
        if descritpion.len() == 1 {
            chunk_string(&mut asset_metadata, "description", &descritpion[0])?
        } else {
            for line in descritpion.clone() {
                desc_array.add(&clib::metadata::TransactionMetadatum::new_text(line)?);
            }
            asset_metadata.insert_str(
                "description",
                &clib::metadata::TransactionMetadatum::new_list(&desc_array),
            )?;
        }
    }

    //Files
    if let Some(files) = &asset.files {
        log::debug!("Found some files: {:?}", files);
        let mut mfiles = MetadataList::new();
        for f in files.clone() {
            let mut filemap = MetadataMap::new();
            filemap.insert_str(
                "name",
                &clib::metadata::TransactionMetadatum::new_text(f.name)?,
            )?;
            filemap.insert_str(
                "mediaType",
                &clib::metadata::TransactionMetadatum::new_text(f.media_type)?,
            )?;
            let mut filelist = MetadataList::new();
            match f.src {
                Value::String(s) => {
                    filelist.add(&clib::metadata::TransactionMetadatum::new_text(s)?)
                }
                Value::Array(v) => {
                    for s in v {
                        if let Ok(t) = serde_json::from_value::<String>(s) {
                            filelist.add(&clib::metadata::TransactionMetadatum::new_text(t)?)
                        }
                    }
                }
                Value::Null => {
                    return Err(MurinError::new(
                        "Null not implemented to restore metadata file source",
                    ))
                }
                Value::Bool(_) => panic!("Bool not implemented to restore metadata file source"),
                Value::Number(_) => {
                    panic!("Number not implemented to restore metadata file source")
                }
                Value::Object(_) => {
                    panic!("Object not implemented to restore metadata file source")
                }
            }

            filemap.insert_str(
                "src",
                &clib::metadata::TransactionMetadatum::new_list(&filelist),
            )?;
            //Other
            if let Some(other) = f.other {
                log::debug!("Found some key / values in other: {:?}", other);
                encode_other_metadata(&mut filemap, &other)?
            }

            mfiles.add(&clib::metadata::TransactionMetadatum::new_map(&filemap));
        }
        asset_metadata.insert_str(
            "files",
            &clib::metadata::TransactionMetadatum::new_list(&mfiles),
        )?;
    }

    // Other
    if let Some(other) = &asset.other {
        encode_other_metadata(&mut asset_metadata, other)?
    }
    let metadatum = clib::metadata::TransactionMetadatum::new_map(&asset_metadata);
    assetmap.insert_str(&asset.tokenname, &metadatum)?;

    Ok(())
}

pub fn create_onshot_policy(
    pub_key_hash: &Ed25519KeyHash,
    current_slot: u64,
) -> (NativeScript, ScriptHash) {
    let mut native_scripts = NativeScripts::new();
    native_scripts.add(&NativeScript::new_script_pubkey(&ScriptPubkey::new(
        pub_key_hash,
    )));

    let slot = cutils::to_bignum(current_slot + 9000);
    native_scripts.add(&NativeScript::new_timelock_expiry(
        &clib::TimelockExpiry::new_timelockexpiry(&slot),
    ));

    let mint_script = NativeScript::new_script_all(&ScriptAll::new(&native_scripts));
    let policy_id = mint_script.hash(); //policyId

    (mint_script, policy_id)
}

pub fn calculate_slot_from_date(date: DateTime<Utc>) -> u32 {
    let ms = date.timestamp();
    let slot = ms - 1596491091 + 4924800;
    slot as u32
}
