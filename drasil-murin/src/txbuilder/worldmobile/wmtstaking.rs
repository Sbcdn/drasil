//! This module defines the data structures for wmt staking
//!
pub mod stake;
use std::fmt;

use cardano_serialization_lib::address::Address;
use serde::{Deserialize, Serialize};
use crate::{TransactionUnspentOutput, MurinError};
use super::enreg::EnRegistrationDatum;

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
        write!(f, "{},{},{},{},{}", 
            self.staking_amount,
            self.ennft,
            if let Some (wa) = &self.wallet_addr { wa.to_bech32(None).unwrap()} else { "None".to_string() },
            if let Some (rr) = &self.registration_reference { rr.to_hex()} else { "None".to_string() },
            if let Some (rd) = &self.registration_datum { serde_json::json!(rd).to_string() } else { "None".to_string() },
        )
    }
}

impl std::str::FromStr for StakeTxData {
    type Err = MurinError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let split: Vec<&str> = src.split(',').collect();
        Ok(StakeTxData {
            staking_amount: split[0].parse::<u64>()?,
            ennft: split[1].to_string(),
            wallet_addr: if split[2] != "None" { Some(Address::from_bech32(split[2])?) } else { None },
            registration_reference: if split[3] != "None" { Some(TransactionUnspentOutput::from_hex(split[3])?)} else { None },
            registration_datum: if split[4] != "None" { Some(serde_json::from_str::<EnRegistrationDatum>(&split[4])?) } else { None },
        })
    }
}

impl StakeTxData {
    /// Creates a new
    pub fn new(
        staking_amount: u64,
        ennft: String,
    ) -> Self { Self {staking_amount, ennft, wallet_addr: None , registration_reference: None, registration_datum: None} }

    pub fn set_wallet_addr(&mut self, wallet_addr: &Address) {
        self.wallet_addr = Some(wallet_addr.clone());
    }

    pub fn set_registration_reference (&mut self, registration_reference: &TransactionUnspentOutput) {
        self.registration_reference = Some(registration_reference.clone());
    }

    pub fn set_registration_datum (&mut self, registration_datum: &EnRegistrationDatum) {
        self.registration_datum = Some(registration_datum.clone());
    }
}


