use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::str;
use warp::http::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use warp::{reject, Rejection};

use super::error::Error;
use crate::{
    OneShotMintPayload, Signature, TXPWrapper, TransactionPattern, WalletTransactionPattern,
};

const BEARER: &str = "Bearer ";

#[derive(Debug, Deserialize, Serialize)]
struct ApiClaims {
    sub: String,
    exp: usize,
}

pub async fn authorize(
    headers: HeaderMap<HeaderValue>,
    body: bytes::Bytes,
) -> Result<(u64, TXPWrapper), Rejection> {
    let publ =
        std::env::var("JWT_PUB_KEY").map_err(|e| Error::ImproperlyConfigError(e.to_string()))?;
    log::info!("checking login data ...");
    let b = body.to_vec();

    let str_slice = str::from_utf8(&b).unwrap();
    log::debug!("str_slice: {}", &str_slice);
    let txp_out = if let Ok(txp) = serde_json::from_str::<TransactionPattern>(str_slice) {
        TXPWrapper::TransactionPattern(Box::new(txp))
    } else if let Ok(s) = serde_json::from_str::<Signature>(str_slice) {
        TXPWrapper::Signature(s)
    } else if let Ok(wal) = serde_json::from_str::<WalletTransactionPattern>(str_slice) {
        TXPWrapper::TransactionPattern(Box::new(wal.into_txp()))
    } else if let Ok(_wal) = serde_json::from_str::<OneShotMintPayload>(str_slice) {
        TXPWrapper::OneShotMinter(serde_json::from_str::<OneShotMintPayload>(str_slice).unwrap())
    } else {
        log::debug!(
            "txp_out: {:?}",
            &serde_json::from_str::<WalletTransactionPattern>(str_slice)
        );
        return Err(reject::not_found());
    };
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
            // Deactivates User Identification, only API token validity checked
            // This code will make sure that the API token is correctly registered with this user
            //
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
    let header = headers.get(AUTHORIZATION).ok_or(Error::NoAuthHeaderError)?;
    let header = str::from_utf8(header.as_bytes()).map_err(|_| Error::NoAuthHeaderError)?;
    if !header.starts_with(BEARER) {
        return Err(Error::InvalidAuthHeaderError);
    }
    Ok(header.trim_start_matches(BEARER).to_owned())
}
