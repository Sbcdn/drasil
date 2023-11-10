//! This module defines the configuration data for WorldMobile
//! smart contracts.

use serde::Deserialize;

/// This type defines the staking smart contract configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct StakingConfig {
    /// Asset name.
    pub asset_name: String,
    /// This is the staking smart contract policy.
    pub policy: String,
}

impl StakingConfig {
    /// Load configuration.
    pub fn load() -> Self {
        let asset_name = String::from("776f726c646d6f62696c65746f6b656e");
        let policy = String::from("1d7f33bd23d85e1a25d87d86fac4f199c3197a2f7afeb662a0f34e1e");

        Self { asset_name, policy }
    }
}
