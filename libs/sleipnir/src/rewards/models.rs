use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use strum_macros::{Display, EnumString};

#[derive(EnumString, Display, Serialize, Deserialize, Debug, Clone)]
pub enum CustomCalculationTypes {
    Freeloaderz,
    FixedAmountPerEpoch,
    FixedAmountPerEpochNonAcc,
    FixedAmountPerEpochCaped,
    Threshold,
    Airdrop,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CapedType {
    pub cap_value: i128,
    pub rwd: i128,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ThresholdType {
    pub stake_threshold: f64,
    pub lower_rwd: i128,
    pub upper_rwd: i128,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FreeloaderzType {
    pub min_stake: i32,
    pub min_earned: f64,
    pub flatten: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FixedAmountPerEpochType {
    pub min_stake: Option<f64>,
    pub amount: i128,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct WhitelistLink {
    pub id: i64,
}

impl fmt::Display for WhitelistLink {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Whitelist,")?;
        write!(f, "{}", self.id)
    }
}

impl std::str::FromStr for WhitelistLink {
    type Err = crate::error::SleipnirError;
    fn from_str(src: &str) -> Result<Self, crate::error::SleipnirError> {
        let split: Vec<&str> = src.split(':').collect();
        if split[0] == "Whitelist" && split[1].parse::<i64>().is_ok() {
            return Ok(WhitelistLink {
                id: split[1].parse::<i64>().unwrap(),
            });
        }
        Err(crate::error::SleipnirError::new(
            "Could not convert string to WhitelistLink",
        ))
    }
}

impl WhitelistLink {
    pub fn is_wl_link(str: &str) -> bool {
        matches!(WhitelistLink::from_str(str), Ok(_))
    }
}

#[derive(Debug, Clone)]
pub struct NewTWL {
    pub user_id: i64,
    pub contract_id: i64,
    pub fingerprint: String,
    pub vesting_period: Option<String>,
    pub pools: Option<Vec<String>>,
    pub mode: String,
    pub equation: String,
    pub start_epoch_in: i64,
    pub end_epoch: Option<i64>,
    pub modificator_equ: Option<String>,
}
