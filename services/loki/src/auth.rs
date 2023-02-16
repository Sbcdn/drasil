use crate::error::{self, Error};
//use chrono::prelude::*;
use hugin::client::connect;
use hugin::VerifyUser;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation}; //encode , EncodingKey, Header
use serde::{Deserialize, Serialize};

use warp::{
    http::header::{HeaderMap, HeaderValue, AUTHORIZATION},
    reject, Rejection,
};

const BEARER: &str = "Bearer ";

#[derive(Debug, Deserialize, Serialize)]
struct ApiClaims {
    sub: String,
    exp: usize,
}

pub(crate) async fn authorize(
    headers: HeaderMap<HeaderValue>,
    //body: bytes::Bytes,
) -> Result<u64, Rejection> {
    let publ = std::env::var("JWT_PUB_KEY")
        .map_err(|_| Error::Custom("env jwt pub not existing".to_string()))?;
    let publ = publ.into_bytes();
    log::info!("checking login data ...");

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
            let mut client = connect(std::env::var("ODIN_URL").unwrap()).await.unwrap();
            let cmd = VerifyUser::new(user_id, jwt);
            log::info!("try to verify user ...");
            match client.build_cmd::<VerifyUser>(cmd).await {
                Ok(_) => {}
                Err(_) => {
                    return Err(reject::custom(Error::JWTTokenError));
                }
            };
            log::info!("Authentication successful: User_id: {:?}", user_id);
            Ok(user_id)
        }

        Err(e) => {
            log::info!("Authentication not successful");
            Err(reject::custom(e))
        }
    }
}

fn jwt_from_header(headers: &HeaderMap<HeaderValue>) -> Result<String, error::Error> {
    let header = match headers.get(AUTHORIZATION) {
        Some(v) => v,
        None => return Err(Error::NoAuthHeaderError),
    };
    let auth_header = match std::str::from_utf8(header.as_bytes()) {
        Ok(v) => v,
        Err(_) => return Err(Error::NoAuthHeaderError),
    };
    if !auth_header.starts_with(BEARER) {
        return Err(Error::InvalidAuthHeaderError);
    }
    Ok(auth_header.trim_start_matches(BEARER).to_owned())
}

/*
pub fn create_jwt(uid: &str) -> Result<String, error::Error> {
    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::seconds(1800))
        .expect("valid timestamp")
        .timestamp();

    let claims = ApiClaims {
        sub: uid.to_owned(),
        exp: expiration as usize,
    };
    let header = Header::new(Algorithm::ES256);
    let key = std::env::var("JWT_KEY")
        .map_err(|_| Error::Custom("env jwt key path not existing".to_string()))?;
    let key = key.into_bytes(); //std::fs::read(key).expect("Could not read jwt key file");
    encode(&header, &claims, &EncodingKey::from_ec_pem(&key).unwrap())
        .map_err(|_| Error::JWTTokenCreationError)
}
*/
