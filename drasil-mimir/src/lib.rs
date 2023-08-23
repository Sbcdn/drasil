#[macro_use]
extern crate diesel;
pub mod schema;
use schema::*;

pub mod models;
pub use models::*;
pub(crate) mod error;
pub use error::MimirError;

pub mod api;
pub use api::*;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;
use std::env;

extern crate dotenv;
extern crate pretty_env_logger;

pub fn establish_connection() -> Result<PgConnection, error::MimirError> {
    dotenv().ok();

    let database_url = env::var("DBSYNC_DB_URL")?;
    Ok(PgConnection::establish(&database_url)?)
}

/*
// Wenn rerunning diesel-cli you need to copy this into the schema!!!!
// Also add 'unspent_utxos' to : allow_tables_to_appear_in_same_query makro in schmea at the end of the file
// The missing types are in module and a 'crate::' in front of them.
// Diesel is not good here....

table! {
    unspent_utxos (id){
        id -> Int8,
        tx_id -> Int8,
        hash -> Bytea,
        index -> Int2,
        address -> Varchar,
        value -> Numeric,
        data_hash -> Nullable<Bytea>,
        stake_address -> Nullable<Varchar>,
    }
}
*/
