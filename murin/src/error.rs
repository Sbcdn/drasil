/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use std::error::Error;
use std::fmt::{self};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MurinError {
    details: String,
}

impl MurinError {
    pub fn new(msg: &str) -> MurinError {
        MurinError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for MurinError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for MurinError {
    fn description(&self) -> &str {
        &self.details
    }
}

unsafe impl Send for MurinError {}
unsafe impl Sync for MurinError {}

impl From<hex::FromHexError> for MurinError {
    fn from(err: hex::FromHexError) -> Self {
        MurinError::new(&err.to_string())
    }
}

impl From<serde_json::Error> for MurinError {
    fn from(err: serde_json::Error) -> Self {
        MurinError::new(&err.to_string())
    }
}

impl From<std::io::Error> for MurinError {
    fn from(err: std::io::Error) -> Self {
        MurinError::new(&err.to_string())
    }
}

impl From<std::env::VarError> for MurinError {
    fn from(err: std::env::VarError) -> Self {
        MurinError::new(&err.to_string())
    }
}

/*
impl From<diesel::ConnectionError> for MurinError {
    fn from(err: diesel::ConnectionError) -> Self {
        MurinError::new(&err.to_string())
    }
}


impl From<diesel::result::Error> for MurinError {
    fn from(err: diesel::result::Error) -> Self {
        MurinError::new(&err.to_string())
    }
}
 */
impl From<std::num::ParseIntError> for MurinError {
    fn from(err: std::num::ParseIntError) -> Self {
        MurinError::new(&err.to_string())
    }
}

impl From<std::num::ParseFloatError> for MurinError {
    fn from(err: std::num::ParseFloatError) -> Self {
        MurinError::new(&err.to_string())
    }
}

impl From<std::str::ParseBoolError> for MurinError {
    fn from(err: std::str::ParseBoolError) -> Self {
        MurinError::new(&err.to_string())
    }
}

impl From<cardano_serialization_lib::error::DeserializeError> for MurinError {
    fn from(err: cardano_serialization_lib::error::DeserializeError) -> Self {
        MurinError::new(&err.to_string())
    }
}

impl From<redis::RedisError> for MurinError {
    fn from(err: redis::RedisError) -> Self {
        MurinError::new(&err.to_string())
    }
}

impl From<cbor_event::Error> for MurinError {
    fn from(err: cbor_event::Error) -> Self {
        MurinError::new(&err.to_string())
    }
}

impl From<cardano_serialization_lib::error::JsError> for MurinError {
    fn from(err: cardano_serialization_lib::error::JsError) -> Self {
        MurinError::new(&err.to_string())
    }
}

impl From<&MurinError> for MurinError {
    fn from(err: &MurinError) -> Self {
        MurinError::new(&err.to_string())
    }
}

impl From<reqwest::Error> for MurinError {
    fn from(err: reqwest::Error) -> Self {
        MurinError::new(&err.to_string())
    }
}

impl From<std::str::Utf8Error> for MurinError {
    fn from(err: std::str::Utf8Error) -> Self {
        MurinError::new(&err.to_string())
    }
}

impl From<std::string::FromUtf8Error> for MurinError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        MurinError::new(&err.to_string())
    }
}

impl From<argon2::password_hash::Error> for MurinError {
    fn from(err: argon2::password_hash::Error) -> Self {
        MurinError::new(&err.to_string())
    }
}
