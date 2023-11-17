//! Helper functions for connecting to Redis database. 

extern crate redis;

pub mod txmind;
pub use txmind::*;

pub mod usedutxos;
pub use usedutxos::*;

use crate::MurinError;
use std::env;

/// Connects to Redis database whose address is given 
/// by environment variable `REDIS_DB`.
pub fn redis_txmind_connection() -> Result<
    (
        Option<redis::cluster::ClusterConnection>,
        Option<redis::Connection>,
    ),
    MurinError,
> {
    let redis_db = env::var("REDIS_DB")?;
    redis_connection(&redis_db)
}

/// Connects to Redis database whose address is given 
/// by environment variable `REDIS_DB_URL_UTXOMIND`.
pub fn redis_usedutxos_connection() -> Result<
    (
        Option<redis::cluster::ClusterConnection>,
        Option<redis::Connection>,
    ),
    MurinError,
> {
    let redis_db = env::var("REDIS_DB_URL_UTXOMIND")?;
    redis_connection(&redis_db)
}

/// Connects to Redis database whose address is given 
/// by environment variable `REDIS_DB_URL_REPLICA`.
pub fn redis_replica_connection() -> Result<
    (
        Option<redis::cluster::ClusterConnection>,
        Option<redis::Connection>,
    ),
    MurinError,
> {
    let redis_db = env::var("REDIS_DB_URL_REPLICA")?;
    redis_connection(&redis_db)
}

/// Connects to Redis database. 
/// 
/// `redis_db` is the address of the Redis database.
/// 
/// Environment variable `REDIS_CLUSTER`: 
/// * `true` => Establish a Redis cluster connection.
/// * `false` => Establish a single Redis connection.
fn redis_connection(
    redis_db: &str,
) -> Result<
    (
        Option<redis::cluster::ClusterConnection>,
        Option<redis::Connection>,
    ),
    MurinError,
> {
    let cluster = env::var("REDIS_CLUSTER")?.parse::<bool>()?;
    if !cluster {
        let scon = match redis::Client::open(redis_db)?.get_connection() {
            Ok(c) => Some(c),
            Err(e) => {
                log::debug!(
                    "Error on trying to establish single redis connection; {:?}",
                    e.to_string()
                );
                None
            }
        };

        Ok((None, scon))
    } else {
        let ccon = match redis::cluster::ClusterClient::open(vec![redis_db])?.get_connection() {
            Ok(c) => Some(c),
            Err(e) => {
                log::debug!(
                    "Error on trying to establish redis cluster connection; {:?}",
                    e.to_string()
                );
                None
            }
        };

        Ok((ccon, None))
    }
}
