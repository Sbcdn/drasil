//! Staking data model.

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use strum_macros::Display;

use crate::error::SystemDBError as Error;

/// The `Action` type enumerates all the smart contract actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display)]
pub enum StakingAction {
    /// Staking a WMT
    Stake,
    /// Unstake a WMT
    UnStake,
}

impl FromStr for StakingAction {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.to_lowercase();
        let action = match value.as_str() {
            "stake" => Self::Stake,
            "unstake" => Self::UnStake,
            _ => return Err(Error::InvalidContractAction(value)),
        };
        Ok(action)
    }
}
