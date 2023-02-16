/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use crate::error::SleipnirError;
use chrono::prelude::*;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Deserialize, Serialize)]
pub enum Permissions {
    RewardClaimRequest,
    RewardClaimCreateContract,
    StandardTransactionsDeleigate,
    Marketplace,
    Standard,
}

impl Permissions {
    pub fn str_to_role(role: &str) -> Permissions {
        match role {
            "1" => Permissions::RewardClaimRequest,
            "2" => Permissions::RewardClaimCreateContract,
            "3" => Permissions::StandardTransactionsDeleigate,
            "4" => Permissions::Marketplace,
            _ => Permissions::Standard,
        }
    }
}

impl fmt::Display for Permissions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Permissions::RewardClaimRequest => write!(f, "1"),
            Permissions::RewardClaimCreateContract => write!(f, "2"),
            Permissions::StandardTransactionsDeleigate => write!(f, "3"),
            Permissions::Marketplace => write!(f, "3"),
            Permissions::Standard => write!(f, "0"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiClaims {
    sub: String,
    exp: usize,
}

pub fn create_jwt(uid: &i64, duration: Option<i64>) -> Result<String, SleipnirError> {
    let user = hugin::database::TBDrasilUser::get_user_by_user_id(uid)?;

    if !user.email_verified
    //&& check_identification(u.identification)
    {
        return Err(SleipnirError::new("invalid user"));
    }

    let mut dur = 317125598072; // 15552000;
    if let Some(t) = duration {
        dur = t
    };

    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::seconds(dur))
        .expect("valid timestamp")
        .timestamp();

    let claims = ApiClaims {
        sub: uid.to_string(),
        exp: expiration as usize,
    };
    let header = Header::new(Algorithm::ES256);
    let key = std::env::var("JWT_KEY").map_err(|_| SleipnirError::new("jwt key error"))?;
    let key = key.into_bytes();
    let token = encode(&header, &claims, &EncodingKey::from_ec_pem(&key).unwrap())
        .map_err(|_| SleipnirError::new("JWT Token could not been created"))?;

    hugin::database::TBDrasilUser::update_api_key(&user.id, &token)?;

    Ok(token)
}
