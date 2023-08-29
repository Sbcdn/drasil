use cardano_serialization_lib as csl;
use csl::{
    address::{Address, BaseAddress},
    crypto::ScriptHash,
    utils::BigNum,
    AssetName,
};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use strum_macros::{Display, EnumString};

pub type Token = (ScriptHash, AssetName, BigNum);
pub type Tokens = Vec<Token>;

use crate::{
    address_from_string_non_async, reward_address_from_address,
    transaction_unspent_outputs_from_string_vec,
};

use super::error::CTSError;

#[derive(Serialize, Debug)]
pub(crate) struct ErrorResponse {
    pub message: String,
    pub status: String,
}

impl ErrorResponse {
    pub fn new(message: String, status: String) -> ErrorResponse {
        ErrorResponse { message, status }
    }
}

#[derive(EnumString, Display, Serialize, Deserialize, Debug, Clone)]

pub enum StandardTxType {
    SendValue,
    RequestValue,
    DelegateToStakePool,
}

#[derive(EnumString, Display, Serialize, Deserialize, Debug, Clone)]

pub enum SmartContractTxType {
    StakeToEarthNode,
    UnStakeFromEarthNode,
    RegisterEarthNode,
    UnRegisterEarthNode,
    RegisterAdmin,
}

#[derive(EnumString, Display, Serialize, Deserialize, Debug, Clone)]

pub enum NativeScriptTxType {
    Vesting,
    MultiSignWallet,
}

#[derive(EnumString, Display, Serialize, Deserialize, Debug, Clone)]

pub enum MintingTxType {
    OneShot,
}
#[derive(Display, Serialize, Deserialize, Debug, Clone)]
pub enum TxTypeWrapper {
    StandardTxType(StandardTxType),
    SmartContractTxType(SmartContractTxType),
    NativeScriptTxType(NativeScriptTxType),
    MintingTxType(MintingTxType),
}

#[derive(Display, Serialize, Deserialize, Debug, Clone)]
pub enum TxSchemaWrapper {
    TransactionPattern(Box<TransactionSchema>),
    Signature(Signature),
    None,
}

impl TxSchemaWrapper {
    pub fn unwrap_txschema(&self) -> Result<TransactionSchema, CTSError> {
        match self {
            TxSchemaWrapper::TransactionPattern(x) => Ok(*x.clone()),
            _ => Err(CTSError::TxSchemaError),
        }
    }
    pub fn is_txschema(&self) -> bool {
        matches!(self, TxSchemaWrapper::TransactionPattern(_))
    }
    pub fn unwrap_signature(&self) -> Result<Signature, CTSError> {
        match self {
            TxSchemaWrapper::Signature(x) => Ok(x.clone()),
            _ => Err(CTSError::TxSchemaError),
        }
    }
    pub fn is_signature(&self) -> bool {
        matches!(self, TxSchemaWrapper::Signature(_))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Signature {
    pub signature: String,
    pub tx: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransactionSchema {
    pub wallet_type: Option<WalletType>, // yoroi, ccvault, gero, flint, ... // or yoroi, cip30, typhon
    pub used_addresses: Vec<String>,
    pub unused_addresses: Vec<String>,
    pub stake_address: Option<Vec<String>>,
    pub change_address: Option<String>,
    pub utxos: Option<Vec<String>>,
    pub excludes: Option<Vec<String>>,
    pub collateral: Option<Vec<String>>,
    pub network: u64,
    pub operation: Option<serde_json::Value>,
}

impl fmt::Display for TransactionSchema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", serde_json::json!(self))
    }
}

impl TransactionSchema {
    pub fn check_txschema(&self) -> Result<(), CTSError> {
        log::debug!("Check operation available...");
        if self.operation.is_none() {
            return Err(CTSError::NoOperation);
        }
        log::debug!("Check used addresses...");
        let grwd: Address;
        if !self.used_addresses.is_empty() {
            // addresses need to all have the same stake address or non otherwise we have a frankenadress in the set,
            // here we can have also enterprise addresses, those we ignore as we cannot check to whom they belong
            let mut rewardaddr = reward_address_from_address(&address_from_string_non_async(
                &self.used_addresses[0],
            )?)?;
            for address in &self.used_addresses {
                let addr = address_from_string_non_async(address)?;
                if BaseAddress::from_address(&addr).is_some() {
                    let raddr = reward_address_from_address(&addr)?;
                    if raddr != rewardaddr {
                        return Err(CTSError::TxSchemaError);
                    }
                    rewardaddr = raddr
                }
            }
            grwd = rewardaddr;
        } else if self.unused_addresses.is_empty() {
            return Err(CTSError::TxSchemaError);
        } else {
            grwd = reward_address_from_address(&address_from_string_non_async(
                &self.unused_addresses[0],
            )?)?
        }
        log::debug!("Check unused addresses...");
        if !self.unused_addresses.is_empty() {
            // If unused addresses are provided all need to have the same stake address as the used addresses, otherwise we have a frankenadress in the set
            for address in &self.unused_addresses {
                let addr = address_from_string_non_async(address)?;
                let rwd = reward_address_from_address(&addr)?;
                if rwd != grwd {
                    return Err(CTSError::TxSchemaError);
                }
            }
        }
        log::debug!("Check netowork...");
        if self.network != 1 && self.network != 0 {
            return Err(CTSError::TxSchemaError);
        }
        log::debug!("Check reward address...");
        if let Some(reward_addresses) = &self.stake_address {
            let grwd_bech32 = hex::encode(grwd.to_bytes());
            if !reward_addresses.contains(&grwd_bech32) {
                return Err(CTSError::TxSchemaError);
            }
        }
        log::debug!("Check change address...");
        if let Some(change_address) = &self.change_address {
            let addr = address_from_string_non_async(change_address)?;
            let rwd = reward_address_from_address(&addr)?;
            if rwd != grwd {
                return Err(CTSError::TxSchemaError);
            }
        }
        log::debug!("Check utxos...");
        if let Some(utxos) = &self.utxos {
            transaction_unspent_outputs_from_string_vec(
                utxos,
                self.collateral.as_ref(),
                self.excludes.as_ref(),
            )?;
        }

        Ok(())
    }

    pub fn check_operation(&self, s: TxTypeWrapper) -> Result<(), CTSError> {
        let op = self.operation.clone();
        if op.is_none() {
            return Err(CTSError::NoOperation);
        }
        match s {
            TxTypeWrapper::StandardTxType(stx) => match stx {
                StandardTxType::SendValue => todo!(),
                StandardTxType::RequestValue => todo!(),
                StandardTxType::DelegateToStakePool => todo!(),
            },
            TxTypeWrapper::SmartContractTxType(sc) => match sc {
                SmartContractTxType::StakeToEarthNode => todo!(),
                SmartContractTxType::UnStakeFromEarthNode => todo!(),
                SmartContractTxType::RegisterEarthNode => {
                    serde_json::from_value::<RegisterEarthNode>(op.unwrap())?;
                }
                SmartContractTxType::UnRegisterEarthNode => {
                    serde_json::from_value::<RegisterEarthNode>(op.unwrap())?;
                }
                SmartContractTxType::RegisterAdmin => {
                    serde_json::from_value::<RegisterEarthNode>(op.unwrap())?;
                }
            },
            TxTypeWrapper::NativeScriptTxType(ns) => match ns {
                NativeScriptTxType::Vesting => todo!(),
                NativeScriptTxType::MultiSignWallet => todo!(),
            },
            TxTypeWrapper::MintingTxType(mint) => match mint {
                MintingTxType::OneShot => todo!(),
            },
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegisterEarthNode {
    // Earth Node Config in JSON format
    pub config: EarthNodeConfig,
    pub ennft_assetname: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct EarthNodeConfig {
    pub validator_address: String,
    pub operator_address: String,
    pub moniker: String,
}

#[derive(EnumString, Display, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum WalletType {
    #[strum(serialize = "nami", ascii_case_insensitive)]
    #[serde(rename = "nami")]
    Nami,
    #[strum(serialize = "eternl", ascii_case_insensitive)]
    #[serde(rename = "eternl")]
    Eternl,
    #[strum(serialize = "gero", ascii_case_insensitive)]
    #[serde(rename = "gero")]
    Gero,
    #[strum(serialize = "flint", ascii_case_insensitive)]
    #[serde(rename = "flint")]
    Flint,
    #[strum(serialize = "yoroi", ascii_case_insensitive)]
    #[serde(rename = "yoroi")]
    Yoroi,
    #[strum(serialize = "typhoon", ascii_case_insensitive)]
    #[serde(rename = "typhoon")]
    Typhon,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct UnsignedTransaction {
    pub id: String,
    pub tx: String,
}

impl UnsignedTransaction {
    pub fn new(tx: Option<&String>, id: &String) -> UnsignedTransaction {
        match tx {
            Some(s) => UnsignedTransaction {
                tx: s.to_string(),
                id: id.to_string(),
            },
            None => UnsignedTransaction {
                tx: "".to_string(),
                id: id.to_string(),
            },
        }
    }
}

impl ToString for UnsignedTransaction {
    fn to_string(&self) -> String {
        format!("{}|{}", self.id, self.tx)
    }
}

impl FromStr for UnsignedTransaction {
    type Err = CTSError;
    fn from_str(src: &str) -> Result<Self, CTSError> {
        let slice: Vec<&str> = src.split('|').collect();
        if slice.len() != 2 {
            Err(CTSError::TxSchemaError)
        } else {
            Ok(UnsignedTransaction {
                id: slice[0].to_string(),
                tx: slice[1].to_string(),
            })
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TxHash {
    pub hash: String,
}

impl ToString for TxHash {
    fn to_string(&self) -> String {
        self.hash.to_string()
    }
}

impl FromStr for TxHash {
    type Err = CTSError;
    fn from_str(src: &str) -> Result<Self, CTSError> {
        Ok(TxHash {
            hash: src.to_string(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TxbError {
    pub message: String,
}

impl ToString for TxbError {
    fn to_string(&self) -> String {
        self.message.to_string()
    }
}

impl FromStr for TxbError {
    type Err = CTSError;
    fn from_str(src: &str) -> Result<Self, CTSError> {
        Ok(TxbError {
            message: src.to_string(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BuilderResult {
    UnsignedTransaction(UnsignedTransaction),
    TxbError(TxbError),
}
