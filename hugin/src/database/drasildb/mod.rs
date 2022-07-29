/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
pub mod api;

use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use std::env;
use chrono::{DateTime,Utc};
use crate::schema::{contracts,drasil_user,multisigs,multisig_keyloc,email_verification_token};

pub fn establish_connection() -> Result<PgConnection, murin::MurinError> {
    dotenv().ok();
    log::debug!("Establishing Drasil DB connection...");
    let database_url = env::var("PLATFORM_DB_URL")?;
    let dbcon = PgConnection::establish(&database_url)?;
    Ok(dbcon)
}


#[derive(Queryable, Identifiable, PartialEq, Debug, Clone, serde::Serialize)]
#[table_name="contracts"]
pub struct TBContracts {
    pub id              : i64,
    pub user_id         : i64,
    pub contract_id     : i64,
    pub contract_type   : String,
    pub description     : Option<String>,
    pub version         : f32,
    pub plutus          : String,
    pub address         : String,
    pub policy_id       : Option<String>,
    pub depricated      : bool,
    pub drasil_lqdty    : Option<i64>,
    pub customer_lqdty  : Option<i64>,
    pub external_lqdty  : Option<i64>,
    pub created_at      : DateTime<Utc>,
    pub updated_at      : DateTime<Utc>
}


#[derive(Insertable, PartialEq, Debug, Clone)]
#[table_name="contracts"]
pub struct TBContractNew<'a> {
    pub user_id         : &'a i64,
    pub contract_id     : &'a i64,
    pub contract_type   : &'a str,
    pub description     : Option<&'a str>,
    pub version         : &'a f32,
    pub plutus          : &'a str,
    pub address         : &'a str,
    pub policy_id       : Option<&'a String>,
    pub depricated      : &'a bool,
    pub drasil_lqdty    : Option<&'a i64>,
    pub customer_lqdty  : Option<&'a i64>,
    pub external_lqdty  : Option<&'a i64>,
}



#[derive(Queryable, PartialEq, Debug, Clone)]
pub struct TBDrasilUser {
    pub id              : i64,
    pub user_id         : i64,
    pub api_pubkey      : Option<String>,
    pub uname           : String,
    pub email           : String,
    pub pwd             : String,
    pub role            : String,
    pub permissions     : Vec::<String>,
    pub company_name    : Option<String>,
    pub address         : Option<String>,
    pub post_code       : Option<String>,
    pub city            : Option<String>, 
    pub addional_addr   : Option<String>, 
    pub country         : Option<String>, 
    pub contact_p_fname : Option<String>,
    pub contact_p_sname : Option<String>,
    pub contact_p_tname : Option<String>,
    pub identification  : Vec::<String>,
    pub email_verified  : bool,
    pub cardano_wallet  : Option<String>,
    pub cwallet_verified : bool,
    pub created_at      : DateTime<Utc>,
    pub updated_at      : DateTime<Utc>
}

#[derive(Insertable, PartialEq, Debug, Clone)]
#[table_name="drasil_user"]
pub struct TBDrasilUserNew<'a> {
    pub user_id         : &'a i64,
    pub api_pubkey      : Option<&'a String>,
    pub uname           : &'a String,
    pub email           : &'a String,
    pub pwd             : &'a String,
    pub role            : &'a String,
    pub permissions     : &'a Vec::<String>,
    pub company_name    : Option<&'a String>,
    pub address         : Option<&'a String>,
    pub post_code       : Option<&'a String>,
    pub city            : Option<&'a String>,  
    pub addional_addr   : Option<&'a String>, 
    pub country         : Option<&'a String>, 
    pub contact_p_fname : Option<&'a String>,
    pub contact_p_sname : Option<&'a String>,
    pub contact_p_tname : Option<&'a String>,
    pub identification  : &'a Vec::<String>,
    pub email_verified  : &'a bool,
    pub cardano_wallet  : Option<&'a String>,
    pub cwallet_verified : &'a bool,
}

#[derive(Queryable, PartialEq, Debug, Clone)]
pub struct TBMultiSigs {
    pub id              : i64,
    pub user_id         : i64,
    pub contract_id     : i64,
    pub description     : String,
    pub version         : f32,
    pub multisig        : String, 
    pub depricated      : bool,
    pub created_at      : DateTime<Utc>,
    pub updated_at      : DateTime<Utc>
}

#[derive(Insertable, PartialEq, Debug, Clone)]
#[table_name="multisigs"]
pub struct TBMultiSigsNew {
    pub user_id         : i64,
    pub contract_id     : i64,
    pub description     : String,
    pub version         : f32,
    pub multisig        : String, 
    pub depricated      : bool
}

#[derive(Queryable, Debug, Clone)]
pub struct TBMultiSigLoc {
    pub id : i64,
    pub user_id : i64,
    pub contract_id : i64,
    pub version : f32,
    pub fee_wallet_addr : Option<String>,
    pub fee : Option<i64>,
    pub pvks : Vec::<String>,
    pub depricated : bool,
    pub created_at : DateTime<Utc>,
    pub updated_at : DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[table_name="multisig_keyloc"]
pub struct TBMultiSigLocNew<'a> {
    pub user_id : &'a i64,
    pub contract_id : &'a i64,
    pub version : &'a f32,
    pub fee_wallet_addr : Option<&'a String>,
    pub fee : Option<&'a i64>,
    pub pvks : &'a Vec::<String>,
    pub depricated : &'a bool,
}

#[derive(serde::Deserialize, Clone)]
pub struct TBEmailVerificationTokenMessage {
    pub id: Option<String>,
    pub email: String,
}

impl TBEmailVerificationTokenMessage {
    pub fn new(id : Option<String>, email: &String) -> Self {
        TBEmailVerificationTokenMessage {
            id: id,
            email: email.clone(),
        }
    }
}


#[derive(serde::Deserialize, serde::Serialize, Queryable, Insertable)]
#[table_name = "email_verification_token"]
pub struct TBEmailVerificationToken {
    pub id : Vec<u8>,
    pub email : String, 
    pub expires_at : DateTime<Utc>,
    pub created_at : DateTime<Utc>, 
}