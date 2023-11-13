//! This file checks whether a JWT bearer token was provided in the HTTP request to Heimdallr server, and whether that
//! bearer token is correct. HTTP requests to Heimdallr server must always contain a correct JWT bearer token, or else 
//! the request gets rejected.

use std::str;

use drasil_hugin::Signature;
use drasil_hugin::{OneShotMintPayload, TXPWrapper, TransactionPattern, WalletTransactionPattern};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use warp::http::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use warp::{reject, Rejection};

use crate::error::Error;

/// Prefix for bearer token in HTTP request header
const BEARER: &str = "Bearer ";

/// The required format of the JWT bearer token's payload
#[derive(Debug, Deserialize, Serialize)]
struct ApiClaims {
    sub: String,
    exp: usize,
}

/// This method authenticates and authorizes the client by using the given JWT token in the HTTP headers. 
/// It also tries to parse the request body into a transaction pattern.
pub(crate) async fn authorize(
    headers: HeaderMap<HeaderValue>,
    body: bytes::Bytes,
) -> Result<(u64, TXPWrapper), Rejection> {
    let publ =
        std::env::var("JWT_PUB_KEY").map_err(|e| Error::ImproperlyConfigError(e.to_string()))?;
    log::info!("checking login data ...");
    let b = body.to_vec();

    let str_slice = str::from_utf8(&b).unwrap();
    let txp_out = if let Ok(txp) = serde_json::from_str::<TransactionPattern>(str_slice) {
        TXPWrapper::TransactionPattern(Box::new(txp))
    } else if let Ok(s) = serde_json::from_str::<Signature>(str_slice) {
        TXPWrapper::Signature(s)
    } else if let Ok(wal) = serde_json::from_str::<WalletTransactionPattern>(str_slice) {
        TXPWrapper::TransactionPattern(Box::new(wal.into_txp()))
    } else {
        TXPWrapper::OneShotMinter(serde_json::from_str::<OneShotMintPayload>(str_slice).unwrap())
    };

    log::debug!("\n\nBody: {b:?}\n\n");
    let publ = publ.into_bytes();
    match jwt_from_header(&headers) {
        Ok(jwt) => {
            let decoded = decode::<ApiClaims>(
                &jwt,
                &DecodingKey::from_ec_pem(&publ).unwrap(),
                &Validation::new(Algorithm::ES256),
            )
            .map_err(Error::JWTTokenError)?;
            log::info!("lookup user data ...");
            let user_id: u64 = decoded.claims.sub.parse().map_err(Error::ParseIntError)?;
            log::debug!("Authentication successful: User_id: {user_id:?}; txp: {txp_out:?}");
            Ok((user_id, txp_out))
        }

        Err(e) => {
            println!("Authentication not successful");
            Err(reject::custom(e))
        }
    }
}

/// Find JWT among HTTP request's headers and format it as a string
fn jwt_from_header(headers: &HeaderMap<HeaderValue>) -> Result<String, Error> {
    let header = headers.get(AUTHORIZATION).ok_or(Error::NoAuthHeaderError)?;
    let header = str::from_utf8(header.as_bytes()).map_err(|_| Error::NoAuthHeaderError)?;
    if !header.starts_with(BEARER) {
        return Err(Error::InvalidAuthHeaderError);
    }
    Ok(header.trim_start_matches(BEARER).to_owned())
}
