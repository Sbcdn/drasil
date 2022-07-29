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
pub struct SleipnirError {
    details: String
}

impl SleipnirError {
    pub fn new(msg: &str) -> SleipnirError {
        SleipnirError {details : msg.to_string() }
    }
}

impl fmt::Display for SleipnirError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{}",self.details)
    }
}

impl Error for SleipnirError {
    fn description(&self) -> &str {
        &self.details
    }
}

unsafe impl Send for SleipnirError {}
unsafe impl Sync for SleipnirError {}


impl From<hex::FromHexError> for SleipnirError {
    fn from(err: hex::FromHexError) -> Self {
        SleipnirError::new(&err.to_string())
    }
}

impl From<serde_json::Error> for SleipnirError {
    fn from(err: serde_json::Error) -> Self {
        SleipnirError::new(&err.to_string())
    }
}

impl From<std::io::Error> for SleipnirError {
    fn from(err: std::io::Error) -> Self {
        SleipnirError::new(&err.to_string())
    }
}


impl From<std::env::VarError> for SleipnirError {
    fn from(err: std::env::VarError) -> Self {
        SleipnirError::new(&err.to_string())
    }
}


impl From<chrono::ParseError> for SleipnirError {
    fn from(err: chrono::ParseError) -> Self {
        SleipnirError::new(&err.to_string())
    }
}

#[cfg(feature = "mimir_bin")]
impl From<mimir::MurinError> for SleipnirError {
    fn from(err: mimir::MurinError) -> Self {
        SleipnirError::new(&err.to_string())
    }
}

impl From<std::string::FromUtf8Error> for SleipnirError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        SleipnirError::new(&err.to_string())
    }
}

impl From<murin::MurinError> for SleipnirError {
    fn from(err: murin::MurinError) -> Self {
        SleipnirError::new(&err.to_string())
    }
}

impl From<murin::clib::error::JsError> for SleipnirError {
    fn from(err: murin::clib::error::JsError) -> Self {
        SleipnirError::new(&err.to_string())
    }
}

impl From<gungnir::RWDError> for SleipnirError {
    fn from(err: gungnir::RWDError) -> Self {
        SleipnirError::new(&err.to_string())
    }
}


impl From<std::num::ParseIntError> for SleipnirError {
    fn from(err: std::num::ParseIntError) -> Self {
        SleipnirError::new(&err.to_string())
    }
}

impl From<std::num::ParseFloatError> for SleipnirError {
    fn from(err: std::num::ParseFloatError) -> Self {
        SleipnirError::new(&err.to_string())
    }
}

impl warp::reject::Reject for SleipnirError {}