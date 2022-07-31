/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
#[macro_use]
extern crate diesel;
pub mod schema;
use schema::*;

pub mod models;
pub use models::*;

pub mod api;
pub use api::*;
pub use murin::error::MurinError;


use diesel::prelude::*;
//use diesel::sql_types::{BigInt};
use diesel::pg::PgConnection;
use dotenv::dotenv;
use std::env;

//use chrono::{DateTime,Utc};
extern crate dotenv; 
extern crate pretty_env_logger;
//#[macro_use] extern crate log;

pub fn establish_connection() -> Result<PgConnection, murin::MurinError> {
    dotenv().ok();

    let database_url = env::var("DBSYNC_DB_URL")?;
    Ok(PgConnection::establish(&database_url)?)
            //.expect(&format!("Error connecting to {}", database_url))
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

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_db1() {
        dotenv().ok();
        println!("Starting test");

        let database_url = env::var("DBSYNC_DB_URL").expect("Could not find env-var 'DBSYNC_DB_URL'");
        println!("Found Database URL {}",database_url);
        
        let conn = PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url));
        println!("Connection established");

        let utxos = api::get_utxo_tokens(&conn, 7684383);
        println!("\nUtxos: {:?}",utxos);
        match utxos {
            Ok(_) => assert!(true),
            Err(e) => assert!(false, "{}", e.to_string()),
        }

    }

    #[tokio::test]
    async fn test_db2() {
        dotenv().ok();
        println!("Starting test");

        let database_url = env::var("DBSYNC_DB_URL").expect("Could not find env-var 'DBSYNC_DB_URL'");
        println!("Found Database URL {}",database_url);
        
        let conn = PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url));
        println!("Connection established");
       
        let addr_utxos = api::get_address_utxos(&conn, &"addr_test1vrmvx0x0c0ymxqy3pkffjqc5ckrk2tyry0va4sah3h7q0mqlqvuc8".to_string());
        println!("Address Utxos: {:?}",addr_utxos);
        match addr_utxos {
            Ok(_) => assert!(true, ),
            Err(e) => assert!(false, "{}", e.to_string()),
        }

    }

    #[tokio::test]
    async fn test_db3() {
        dotenv().ok();
        println!("Starting test");

        let database_url = env::var("DBSYNC_DB_URL").expect("Could not find env-var 'DBSYNC_DB_URL'");
        println!("Found Database URL {}",database_url);
        
        let conn = PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url));
        println!("Connection established");

        let addr_utxos = api::get_stake_address_utxos(&conn, &"stake_test1uzdfk4vexpw99fkva3p4z6w89uqshhlzndjend2mzy9y9qszkf4wy".to_string());
        println!("Stake Address Utxos: {:?}",addr_utxos);
        match addr_utxos {
            Ok(_) => assert!(true),
            Err(e) => assert!(false, "{}", e.to_string()),
        }
    }

    #[tokio::test]
    async fn test_slot() {
        dotenv().ok();
        println!("Starting test");

        let database_url = env::var("DBSYNC_DB_URL").expect("Could not find env-var 'DBSYNC_DB_URL'");
        println!("Found Database URL {}",database_url);
        
        let conn = PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url));
        println!("Connection established");
        
        match api::get_slot(&conn) {
            Ok(_) => assert!(true),
            Err(e) => assert!(false, "{}", e.to_string()),
        };
    }
}
*/
