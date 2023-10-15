//! # Configuration
//!
//! This module defines various configuration data.

use cdp::{DBSyncProvider, DataProvider};
use serde::Deserialize;

/// This type represents the main configuration data.
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    /// Registration config.
    pub registration: RegistrationConfig,
    /// Database config.
    pub database: DatabaseConfig,
}

/// This type represents the database configuration data.
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// DBSync connection string
    pub dbsync: String,
}

/// This the registration smart contracts configuration data
#[derive(Debug, Clone, Deserialize)]
pub struct RegistrationConfig {
    /// This is the EN registration contract
    pub contract: String,

    /// This is the EN registration policy
    pub policy: String,
}

impl Config {
    pub fn new_cdp_provider(&self) -> DataProvider<DBSyncProvider> {
        let provider = cdp::DBSyncProvider::new(cdp::Config {
            db_path: self.database.dbsync.clone(),
        });
        cdp::DataProvider::new(provider)
    }
}
