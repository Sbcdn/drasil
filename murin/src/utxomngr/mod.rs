/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
extern crate dotenv;
extern crate redis;

pub mod txmind;
pub use txmind::*;

pub mod usedutxos;
pub use usedutxos::*;

use crate::MurinError;
use dotenv::dotenv;
use std::env;

pub fn redis_txmind_connection() -> Result<redis::cluster::ClusterConnection, MurinError> {
    dotenv().ok();

    let redis_db = env::var("REDIS_DB")?; // redis://[<username>][:<password>@]<hostname>[:port][/<db>]
    let client = redis::cluster::ClusterClient::open(vec![redis_db])?; // redis://127.0.0.1/
    Ok(client.get_connection()?)
}

pub fn redis_usedutxos_connection() -> Result<
    (
        Option<redis::cluster::ClusterConnection>,
        Option<redis::Connection>,
    ),
    MurinError,
> {
    dotenv().ok();

    let cluster = env::var("REDIS_CLUSTER")?.parse::<bool>()?;
    let redis_db = env::var("REDIS_DB_URL_UTXOMIND")?; // redis://[<username>][:<password>@]<hostname>[:port][/<db>]

    if !cluster {
        let scon = match redis::Client::open(redis_db.clone())?.get_connection() {
            Ok(c) => Some(c),
            Err(e) => {
                log::debug!(
                    "Error on trying to establish single redis connection; {:?}",
                    e.to_string()
                );
                None
            }
        };
        return Ok((None, scon));
    } else {
        let ccon =
            match redis::cluster::ClusterClient::open(vec![redis_db.clone()])?.get_connection() {
                Ok(c) => Some(c),
                Err(e) => {
                    log::debug!(
                        "Error on trying to establish redis cluster connection; {:?}",
                        e.to_string()
                    );
                    None
                }
            };
        return Ok((ccon, None));
    }

    Ok((None, None))
}

pub fn redis_replica_connection() -> Result<redis::cluster::ClusterConnection, MurinError> {
    dotenv().ok();

    let redis_db = env::var("REDIS_DB_URL_REPLICA")?; // redis://[<username>][:<password>@]<hostname>[:port][/<db>]
    let client = redis::cluster::ClusterClient::open(vec![redis_db])?; // redis://127.0.0.1/
    Ok(client.get_connection()?)
}
