use std::str;

use drasil_hugin::Signature;
use drasil_hugin::{OneShotMintPayload, TXPWrapper, TransactionPattern, WalletTransactionPattern};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use warp::http::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use warp::{reject, Rejection};

use crate::error::Error;

const BEARER: &str = "Bearer ";

#[derive(Debug, Deserialize, Serialize)]
struct ApiClaims {
    sub: String,
    exp: usize,
}

pub(crate) async fn authorize(
    headers: HeaderMap<HeaderValue>,
    body: bytes::Bytes,
) -> Result<(u64, TXPWrapper), Rejection> {
    let publ = std::env::var("JWT_PUB_KEY")
        .map_err(|_| Error::Custom("env jwt pub not existing".to_string()))?;
    let publ = publ.into_bytes();
    log::info!("checking login data ...");
    let b = Vec::<u8>::from(body);

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
    match jwt_from_header(&headers) {
        Ok(jwt) => {
            let decoded = decode::<ApiClaims>(
                &jwt,
                &DecodingKey::from_ec_pem(&publ).unwrap(),
                &Validation::new(Algorithm::ES256),
            )
            .map_err(|_| reject::custom(Error::JWTTokenError))?;
            log::info!("lookup user data ...");
            let user_id = decoded.claims.sub.parse::<u64>().map_err(|_| {
                reject::custom(Error::Custom("Could not parse customer id".to_string()))
            })?;
            // Deactivates User Identification, only API token validity checked
            //let mut client = connect(std::env::var("ODIN_URL").unwrap()).await.unwrap();
            //let cmd = VerifyUser::new(user_id, jwt);
            //log::info!("try to verify user ...");
            //match client.build_cmd::<VerifyUser>(cmd).await {
            //    Ok(_) => {}
            //    Err(_) => {
            //        return Err(reject::custom(Error::JWTTokenError));
            //    }
            //};
            log::debug!("Authentication successful: User_id: {user_id:?}; txp: {txp_out:?}");
            Ok((user_id, txp_out))
        }

        Err(e) => {
            println!("Authentication not successful");
            Err(reject::custom(e))
        }
    }
}

fn jwt_from_header(headers: &HeaderMap<HeaderValue>) -> Result<String, Error> {
    let header = match headers.get(AUTHORIZATION) {
        Some(v) => v,
        None => return Err(Error::NoAuthHeaderError),
    };
    let auth_header = match str::from_utf8(header.as_bytes()) {
        Ok(v) => v,
        Err(_) => return Err(Error::NoAuthHeaderError),
    };
    if !auth_header.starts_with(BEARER) {
        return Err(Error::InvalidAuthHeaderError);
    }
    Ok(auth_header.trim_start_matches(BEARER).to_owned())
}
