#![allow(clippy::extra_unused_lifetimes)]

pub mod api;
pub mod error;
use crate::schema::{
    ca_payment, ca_payment_hash, contracts, drasil_user, email_verification_token, multisig_keyloc,
    multisigs,
};
use chrono::{DateTime, Utc};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use error::SystemDBError;
use gungnir::{BigDecimal, FromPrimitive, ToPrimitive};
use std::env;

pub fn establish_connection() -> Result<PgConnection, SystemDBError> {
    log::debug!("Establishing Drasil DB connection...");
    let database_url = env::var("PLATFORM_DB_URL")?;
    let dbcon = PgConnection::establish(&database_url)?;
    Ok(dbcon)
}

#[derive(Queryable, Identifiable, PartialEq, Debug, Clone, serde::Serialize)]
#[diesel(table_name = contracts)]
pub struct TBContracts {
    pub id: i64,
    pub user_id: i64,
    pub contract_id: i64,
    pub contract_type: String,
    pub description: Option<String>,
    pub version: f32,
    pub plutus: String,
    pub address: String,
    pub policy_id: Option<String>,
    pub depricated: bool,
    pub drasil_lqdty: Option<i64>,
    pub customer_lqdty: Option<i64>,
    pub external_lqdty: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, PartialEq, Debug, Clone)]
#[diesel(table_name = contracts)]
pub struct TBContractNew<'a> {
    pub user_id: &'a i64,
    pub contract_id: &'a i64,
    pub contract_type: &'a str,
    pub description: Option<&'a str>,
    pub version: &'a f32,
    pub plutus: &'a str,
    pub address: &'a str,
    pub policy_id: Option<&'a String>,
    pub depricated: &'a bool,
    pub drasil_lqdty: Option<&'a i64>,
    pub customer_lqdty: Option<&'a i64>,
    pub external_lqdty: Option<&'a i64>,
}

#[derive(Queryable, PartialEq, Eq, Debug, Clone)]
pub struct TBDrasilUser {
    pub id: i64,
    pub user_id: i64,
    pub api_pubkey: Option<String>,
    pub uname: String,
    pub email: String,
    pub pwd: String,
    pub role: String,
    pub permissions: Vec<String>,
    pub company_name: Option<String>,
    pub address: Option<String>,
    pub post_code: Option<String>,
    pub city: Option<String>,
    pub addional_addr: Option<String>,
    pub country: Option<String>,
    pub contact_p_fname: Option<String>,
    pub contact_p_sname: Option<String>,
    pub contact_p_tname: Option<String>,
    pub identification: Vec<String>,
    pub email_verified: bool,
    pub cardano_wallet: Option<String>,
    pub cwallet_verified: bool,
    pub drslpubkey: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, PartialEq, Eq, Debug, Clone)]
#[diesel(table_name = drasil_user)]
pub struct TBDrasilUserNew<'a> {
    pub user_id: &'a i64,
    pub api_pubkey: Option<&'a String>,
    pub uname: &'a String,
    pub email: &'a String,
    pub pwd: &'a String,
    pub role: &'a String,
    pub permissions: &'a Vec<String>,
    pub company_name: Option<&'a String>,
    pub address: Option<&'a String>,
    pub post_code: Option<&'a String>,
    pub city: Option<&'a String>,
    pub addional_addr: Option<&'a String>,
    pub country: Option<&'a String>,
    pub contact_p_fname: Option<&'a String>,
    pub contact_p_sname: Option<&'a String>,
    pub contact_p_tname: Option<&'a String>,
    pub identification: &'a Vec<String>,
    pub email_verified: &'a bool,
    pub cardano_wallet: Option<&'a String>,
    pub cwallet_verified: &'a bool,
    pub drslpubkey: &'a String,
}

#[derive(Queryable, PartialEq, Debug, Clone)]
pub struct TBMultiSigs {
    pub id: i64,
    pub user_id: i64,
    pub contract_id: i64,
    pub description: String,
    pub version: f32,
    pub multisig: String,
    pub depricated: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, PartialEq, Debug, Clone)]
#[diesel(table_name = multisigs)]
pub struct TBMultiSigsNew {
    pub user_id: i64,
    pub contract_id: i64,
    pub description: String,
    pub version: f32,
    pub multisig: String,
    pub depricated: bool,
}

#[derive(Queryable, Debug, Clone)]
pub struct TBMultiSigLoc {
    pub id: i64,
    pub user_id: i64,
    pub contract_id: i64,
    pub version: f32,
    pub fee_wallet_addr: Option<String>,
    pub fee: Option<i64>,
    pub pvks: Vec<String>,
    pub depricated: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = multisig_keyloc)]
pub struct TBMultiSigLocNew<'a> {
    pub user_id: &'a i64,
    pub contract_id: &'a i64,
    pub version: &'a f32,
    pub fee_wallet_addr: Option<&'a String>,
    pub fee: Option<&'a i64>,
    pub pvks: &'a Vec<String>,
    pub depricated: &'a bool,
}

#[derive(serde::Deserialize, Clone)]
pub struct TBEmailVerificationTokenMessage {
    pub id: Option<String>,
    pub email: String,
}

impl TBEmailVerificationTokenMessage {
    pub fn new(id: Option<String>, email: &str) -> Self {
        TBEmailVerificationTokenMessage {
            id,
            email: email.to_owned(),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Queryable, Insertable)]
#[diesel(table_name = email_verification_token)]
pub struct TBEmailVerificationToken {
    pub id: Vec<u8>,
    pub email: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct CaValue {
    pub ada_amount: gungnir::BigDecimal,
    pub token: Vec<crate::Token>,
}

impl CaValue {
    pub fn new(ada_amount: u64, token: Vec<crate::Token>) -> Self {
        let ada_amount = BigDecimal::from_u64(ada_amount).unwrap();
        CaValue { ada_amount, token }
    }

    pub fn into_cvalue(&self) -> Result<murin::clib::utils::Value, SystemDBError> {
        let coin = murin::clib::utils::to_bignum(self.ada_amount.to_u64().unwrap());
        let mut value = murin::clib::utils::Value::new(&coin);
        let mut ma = murin::clib::MultiAsset::new();
        if !self.token.is_empty() {
            for t in &self.token {
                let mut asset = murin::clib::Assets::new();
                let tok = t.into_asset()?;
                asset.insert(&tok.1, &tok.2);
                ma.insert(&tok.0, &asset);
            }
        }
        if ma.len() > 0 {
            value.set_multiasset(&ma);
        }
        Ok(value)
    }
}

#[derive(serde::Deserialize, serde::Serialize, Queryable, PartialEq, Eq, Debug)]
pub struct TBCaPayment {
    pub id: i64,
    pub user_id: i64,
    pub contract_id: i64,
    pub value: String, // String of CA Value json encoded
    pub tx_hash: Option<String>,
    pub user_appr: Option<String>,
    pub drasil_appr: Option<String>,
    pub stauts_bl: Option<String>,
    pub stauts_pa: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = ca_payment)]
pub struct TBCaPaymentNew<'a> {
    pub user_id: &'a i64,
    pub contract_id: &'a i64,
    pub value: &'a String, // String of CA Value json encoded
    pub tx_hash: Option<&'a str>,
    pub user_appr: Option<&'a str>,
    pub drasil_appr: Option<&'a str>,
    pub status_bl: Option<&'a str>,
    pub status_pa: &'a str,
}

#[derive(serde::Deserialize, serde::Serialize, Queryable, PartialEq, Eq, Debug)]
pub struct TBCaPaymentHash {
    pub id: i64,
    pub payment_id: i64,
    pub payment_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = ca_payment_hash)]
pub struct TBCaPaymentHashNew<'a> {
    pub payment_id: &'a i64,
    pub payment_hash: &'a str,
}
