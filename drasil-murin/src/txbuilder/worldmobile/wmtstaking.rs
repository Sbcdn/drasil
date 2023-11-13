//! This module defines the data structures for wmt staking
//!
pub mod stake;

use std::fmt;

use cardano_serialization_lib::address::Address;
use serde::{Deserialize, Serialize};

use super::enreg::EnRegistrationDatum;
use crate::{MurinError, TransactionUnspentOutput};

/// This type represents the staking transaction data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeTxData {
    /// The staking amount.
    pub staking_amount: u64,
    /// The transaction datum
    pub ennft: String,
    /// Wallet address.
    pub wallet_addr: Option<Address>,
    /// This is the registration UTXO for reference.
    /// This UTXO is not spent.
    pub registration_reference: Option<TransactionUnspentOutput>,
    /// The registration Datum of the referenced Registration UTXO.
    pub registration_datum: Option<EnRegistrationDatum>,
}

impl fmt::Display for StakeTxData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{},{}", self.staking_amount, self.ennft)?;
        if let Some(wa) = &self.wallet_addr {
            write!(f, "{}", wa.to_bech32(None).unwrap())?;
        } else {
            write!(f, "None")?;
        }

        if let Some(rr) = &self.registration_reference {
            write!(f, "{}", rr.to_hex())?;
        } else {
            write!(f, "None")?;
        }

        if let Some(rd) = &self.registration_datum {
            write!(f, "{}", serde_json::json!(rd))
        } else {
            write!(f, "None")
        }
    }
}

impl std::str::FromStr for StakeTxData {
    type Err = MurinError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        // Split the stringify stake data into parts.
        let parts: Vec<&str> = src.split(',').collect();
        if parts.len() < 4 {
            return Err(MurinError::new("invalid 'StakeTxData` string parts"));
        }
        // The first element is the staking amount
        let staking_amount = parts[0].parse::<u64>()?;
        // The second element is the earth node NFT
        let ennft = parts[1].to_string();
        // The third element is the wallet address.
        let wallet_addr = if parts[2] == "None" {
            None
        } else {
            Some(Address::from_bech32(parts[2])?)
        };
        // The fourth element is the registration reference.
        let registration_reference = if parts[3] == "None" {
            None
        } else {
            Some(TransactionUnspentOutput::from_hex(parts[3])?)
        };
        // The fifth element is the registration datum.
        let registration_datum = if parts[4] == "None" {
            None
        } else {
            Some(serde_json::from_str::<EnRegistrationDatum>(parts[4])?)
        };

        Ok(Self {
            staking_amount,
            ennft,
            wallet_addr,
            registration_reference,
            registration_datum,
        })
    }
}

impl StakeTxData {
    /// Creates new stake transaction data of the given amount
    /// and the earth node NFT token.
    pub fn new(staking_amount: u64, ennft: String) -> Self {
        Self {
            staking_amount,
            ennft,
            wallet_addr: None,
            registration_reference: None,
            registration_datum: None,
        }
    }
}
