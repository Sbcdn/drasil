/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/

use crate::{error::Error, Result, WebResult};
use chrono::prelude::*;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::fmt;
use warp::{
    filters::header::headers_cloned,
    http::header::{HeaderMap, HeaderValue, AUTHORIZATION},
    reject, Filter, Rejection,
};

const BEARER: &str = "Bearer ";

#[derive(Clone, PartialEq, Eq)]
pub enum Role {
    StandardUser,
    EnterpriseUser,
    Retailer,
    DrasilAdmin,
}

impl Role {
    pub fn from_str(role: &str) -> Role {
        match role {
            "0" => Role::DrasilAdmin,
            "1" => Role::Retailer,
            "2" => Role::EnterpriseUser,
            _ => Role::StandardUser,
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::DrasilAdmin => write!(f, "0"),
            Role::Retailer => write!(f, "1"),
            Role::EnterpriseUser => write!(f, "2"),
            Role::StandardUser => write!(f, "3"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Claims {
    sub: String,
    rpm: String,
    exp: usize,
}

pub fn with_auth(role: Role) -> impl Filter<Extract = (String,), Error = Rejection> + Clone {
    headers_cloned()
        .map(move |headers: HeaderMap<HeaderValue>| (role.clone(), headers))
        .and_then(authorize)
}

pub fn create_jwt(uid: &str, role: &Role) -> Result<String> {
    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::seconds(1800))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: uid.to_owned(),
        rpm: role.to_string(),
        exp: expiration as usize,
    };
    let header = Header::new(Algorithm::ES256);
    let key = std::env::var("JWT_KEY")
        .map_err(|_| Error::Custom("env jwt key path not existing".to_string()))?;
    let key = key.into_bytes(); //std::fs::read(key).expect("Could not read jwt key file");
    encode(&header, &claims, &EncodingKey::from_ec_pem(&key).unwrap())
        .map_err(|_| Error::JWTTokenCreationError)
}

async fn authorize((role, headers): (Role, HeaderMap<HeaderValue>)) -> WebResult<String> {
    let publ = std::env::var("JWT_PUB_KEY")
        .map_err(|_| Error::Custom("env jwt pub not existing".to_string()))?;
    let publ = publ.into_bytes();

    match jwt_from_header(&headers) {
        Ok(jwt) => {
            let decoded = decode::<Claims>(
                &jwt,
                &DecodingKey::from_ec_pem(&publ).unwrap(),
                &Validation::new(Algorithm::ES256),
            )
            .map_err(|_| reject::custom(Error::JWTTokenError))?;

            if role == Role::DrasilAdmin && Role::from_str(&decoded.claims.rpm) != Role::DrasilAdmin
            {
                println!("No Admin permission");
                return Err(reject::custom(Error::NoPermissionError));
            }
            if role == Role::Retailer
                && (Role::from_str(&decoded.claims.rpm) != Role::Retailer
                    && Role::from_str(&decoded.claims.rpm) != Role::DrasilAdmin)
            {
                println!("No Retailer permission");
                return Err(reject::custom(Error::NoPermissionError));
            }
            if role == Role::EnterpriseUser
                && (Role::from_str(&decoded.claims.rpm) != Role::EnterpriseUser
                    && Role::from_str(&decoded.claims.rpm) != Role::Retailer
                    && Role::from_str(&decoded.claims.rpm) != Role::DrasilAdmin)
            {
                println!("No Enterprise permission");
                return Err(reject::custom(Error::NoPermissionError));
            }

            Ok(decoded.claims.sub)
        }
        Err(e) => Err(reject::custom(e)),
    }
}

fn jwt_from_header(headers: &HeaderMap<HeaderValue>) -> Result<String> {
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
