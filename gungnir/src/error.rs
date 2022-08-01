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

#[derive(Debug, Clone, PartialEq)]
pub struct RWDError {
    details: String,
}

impl RWDError {
    pub fn new(msg: &str) -> RWDError {
        RWDError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for RWDError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for RWDError {
    fn description(&self) -> &str {
        &self.details
    }
}

unsafe impl Send for RWDError {}
unsafe impl Sync for RWDError {}

impl From<hex::FromHexError> for RWDError {
    fn from(err: hex::FromHexError) -> Self {
        RWDError::new(&err.to_string())
    }
}

impl From<serde_json::Error> for RWDError {
    fn from(err: serde_json::Error) -> Self {
        RWDError::new(&err.to_string())
    }
}

impl From<std::io::Error> for RWDError {
    fn from(err: std::io::Error) -> Self {
        RWDError::new(&err.to_string())
    }
}

impl From<std::env::VarError> for RWDError {
    fn from(err: std::env::VarError) -> Self {
        RWDError::new(&err.to_string())
    }
}

impl From<diesel::ConnectionError> for RWDError {
    fn from(err: diesel::ConnectionError) -> Self {
        RWDError::new(&err.to_string())
    }
}

impl From<diesel::result::Error> for RWDError {
    fn from(err: diesel::result::Error) -> Self {
        RWDError::new(&err.to_string())
    }
}

impl From<chrono::ParseError> for RWDError {
    fn from(err: chrono::ParseError) -> Self {
        RWDError::new(&err.to_string())
    }
}

#[cfg(feature = "mimir_bin")]
impl From<mimir::MurinError> for RWDError {
    fn from(err: mimir::MurinError) -> Self {
        RWDError::new(&err.to_string())
    }
}

impl From<std::string::FromUtf8Error> for RWDError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        RWDError::new(&err.to_string())
    }
}

impl From<std::num::ParseIntError> for RWDError {
    fn from(err: std::num::ParseIntError) -> Self {
        RWDError::new(&err.to_string())
    }
}
