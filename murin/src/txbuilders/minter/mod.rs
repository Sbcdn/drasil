/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
pub mod build_minttx;
pub mod build_oneshot_mint;
use crate::MurinError;
use crate::{MintTokenAsset, TokenAsset};
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr, utils as cutils};
use chrono::{DateTime, Utc};
use clib::crypto::{Ed25519KeyHash, ScriptHash};
use clib::{NativeScript, NativeScripts, ScriptAll, ScriptPubkey};
use serde::{Deserialize, Serialize};
use std::str;

pub use build_minttx::build_mint_multisig;

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

    pub fn get_stake_addr(&self) -> Option<caddr::Address> {
        self.receiver_stake_addr.clone()
    }

    pub fn get_stake_addr_bech32(&self) -> Result<Option<String>, MurinError> {
        if let Some(sa) = &self.receiver_stake_addr {
            return Ok(Some(sa.to_bech32(None)?.clone()));
        };

        Ok(None)
    }

    pub fn get_payment_addr(&self) -> caddr::Address {
        self.receiver_payment_addr.clone()
    }

    pub fn get_payment_addr_bech32(&self) -> Result<String, MurinError> {
        Ok(self.receiver_payment_addr.to_bech32(None)?.clone())
    }

    pub fn get_metadata(&self) -> Cip25Metadata {
        self.mint_metadata.clone()
    }

    pub fn get_auto_mint(&self) -> bool {
        self.auto_mint.clone()
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

    pub fn set_mint_tokens(&mut self, mint_tokens: Vec<MintTokenAsset>) -> () {
        self.mint_tokens = mint_tokens;
    }

    pub fn set_metadata(&mut self, metadata: Cip25Metadata) -> () {
        self.mint_metadata = metadata;
    }

    pub fn set_fee_addr(&mut self, addr: caddr::Address) -> () {
        self.fee_addr = Some(addr);
    }

    pub fn set_fee(&mut self, fee: i64) -> () {
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
        let s_stake_addr = match self.get_stake_addr() {
            Some(addr) => hex::encode(addr.to_bytes()),
            None => "NoData".to_string(),
        };

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

impl core::str::FromStr for MinterTxData {
    type Err = MurinError;
    fn from_str(src: &str) -> std::result::Result<Self, Self::Err> {
        let slice: Vec<&str> = src.split("|").collect();
        //debug!("Slice: {:?}",slice);
        if slice.len() == 8 {
            // restore token vector
            let mut tokens = Vec::<MintTokenAsset>::new();
            let tokens_vec: Vec<&str> = slice[0].split("!").collect();
            for token in tokens_vec {
                let token_slice: Vec<&str> = token.split("?").collect();
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
                auto_mint: auto_mint,
                fee_addr: fee_addr,
                fee: fee,
                contract_id: contract_id,
            })
        } else {
            Err(MurinError::new(
                &format!(
                    "Error the provided string '{}' cannot be parsed into 'RWDTxData' ",
                    src
                )
                .to_string(),
            ))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataOther {
    pub key: String,
    pub value: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataFile {
    pub name: String,
    #[serde(rename(serialize = "mediaType", deserialize = "mediaType"))]
    pub media_type: String,
    pub src: Vec<String>,
    pub other: Option<Vec<MetadataOther>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMetadata {
    pub name: Option<String>,
    pub tokenname: String,
    #[serde(rename(serialize = "mediaType", deserialize = "mediaType"))]
    pub media_type: Option<String>,
    pub descritpion: Option<Vec<String>>,
    pub image_url: Option<String>,
    pub files: Option<Vec<MetadataFile>>,
    pub other: Option<Vec<MetadataOther>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cip25Metadata {
    pub assets: Vec<AssetMetadata>,
    pub other: Option<Vec<MetadataOther>>,
    pub version: String,
}

impl Cip25Metadata {
    pub fn new() -> Self {
        Cip25Metadata {
            assets: Vec::<AssetMetadata>::new(),
            other: None,
            version: "1.0".to_string(),
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

pub fn make_mint_metadata_from_json(
    raw_metadata: &Cip25Metadata,
    tokens: Vec<TokenAsset>,
    policy_id: clib::PolicyID,
) -> std::result::Result<clib::metadata::GeneralTransactionMetadata, MurinError> {
    pub use clib::metadata::*;
    /////////////////////////////////////////////////////////////////////////////////////////////////////
    //
    //Auxiliary Data
    //  Plutus Script and Metadata
    /////////////////////////////////////////////////////////////////////////////////////////////////////
    let policy_str = hex::encode(policy_id.to_bytes());
    let mut toplevel_metadata = clib::metadata::GeneralTransactionMetadata::new();
    //let mut raw_metadata =  Vec::<String>::new();

    debug!("RawMetadata: {:?}", raw_metadata);

    // Check if all tokens have metadata available
    let mut i = 0;
    'avail_tok: for token in tokens.clone() {
        let t_name = str::from_utf8(&token.1.name())?.to_string();
        debug!("TName: {}", t_name);
        for asset in raw_metadata.assets.clone() {
            if asset.tokenname == t_name {
                i += 1;
                continue 'avail_tok;
            }
        }
    }
    if tokens.len() != i {
        return Err(MurinError::new(&format!("Error provided metadata and tokens to mint are not fitting, please provide correct metadata: \n {:?}",raw_metadata)));
    }

    let mut metamap = clib::metadata::MetadataMap::new();
    let mut assetmap = MetadataMap::new();
    for asset in &raw_metadata.assets {
        make_721_asset_entry(&asset, &mut assetmap)?;
    }
    let metadatum = clib::metadata::TransactionMetadatum::new_map(&assetmap);
    metamap.insert_str(&policy_str, &metadatum)?;
    metamap.insert_str(
        "version",
        &clib::metadata::TransactionMetadatum::new_text(raw_metadata.version.clone())?,
    )?;

    // Other
    if let Some(other) = &raw_metadata.other {
        for o in other.clone() {
            if o.value.len() >= 1 {
                let mut olist = MetadataList::new();
                for l in o.value {
                    olist.add(&clib::metadata::TransactionMetadatum::new_text(l)?)
                }
                metamap.insert_str(
                    &o.key,
                    &clib::metadata::TransactionMetadatum::new_list(&olist),
                )?;
            } else {
                metamap.insert_str(
                    &o.key,
                    &clib::metadata::TransactionMetadatum::new_text(o.value[0].clone())?,
                )?;
            }
        }
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
                    .expect("If an image url is provided a mediaType is mandatory")
                    .clone(),
            )?,
        )?;
    }

    //Description
    let mut desc_array = MetadataList::new();
    if let Some(descritpion) = &asset.descritpion {
        for line in descritpion.clone() {
            desc_array.add(&clib::metadata::TransactionMetadatum::new_text(line)?);
        }
        asset_metadata.insert_str(
            "description",
            &clib::metadata::TransactionMetadatum::new_list(&desc_array),
        )?;
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
            for s in f.src {
                filelist.add(&clib::metadata::TransactionMetadatum::new_text(s)?)
            }
            filemap.insert_str(
                "src",
                &clib::metadata::TransactionMetadatum::new_list(&filelist),
            )?;
            //Other
            if let Some(other) = f.other {
                log::debug!("Found some key / values in other: {:?}", other);
                for o in other {
                    if o.value.len() >= 1 {
                        let mut filelist = MetadataList::new();
                        for l in o.value {
                            filelist.add(&clib::metadata::TransactionMetadatum::new_text(l)?)
                        }
                        filemap.insert_str(
                            &o.key,
                            &clib::metadata::TransactionMetadatum::new_list(&filelist),
                        )?;
                    } else {
                        filemap.insert_str(
                            &o.key,
                            &clib::metadata::TransactionMetadatum::new_text(o.value[0].clone())?,
                        )?;
                    }
                }
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
        for o in other.clone() {
            if o.value.len() >= 1 {
                let mut olist = MetadataList::new();
                for l in o.value {
                    olist.add(&clib::metadata::TransactionMetadatum::new_text(l)?)
                }
                asset_metadata.insert_str(
                    &o.key,
                    &clib::metadata::TransactionMetadatum::new_list(&olist),
                )?;
            } else {
                asset_metadata.insert_str(
                    &o.key,
                    &clib::metadata::TransactionMetadatum::new_text(o.value[0].clone())?,
                )?;
            }
        }
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

    let slot = cutils::to_bignum(current_slot + 1800);
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
