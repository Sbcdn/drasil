use std::error::Error;
use std::fmt;

use drasil_murin::MurinError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SleipnirError {
    details: String,
}

impl SleipnirError {
    pub fn new(msg: &str) -> SleipnirError {
        SleipnirError {
            details: msg.to_string(),
        }
    }
}

impl Into<MurinError> for SleipnirError {
    fn into(self) -> MurinError {
        MurinError::Custom(self.to_string())
    }
}

impl fmt::Display for SleipnirError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
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
impl From<drasil_mimir::MurinError> for SleipnirError {
    fn from(err: drasil_mimir::MurinError) -> Self {
        SleipnirError::new(&err.to_string())
    }
}

impl From<std::string::FromUtf8Error> for SleipnirError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        SleipnirError::new(&err.to_string())
    }
}

impl From<drasil_murin::MurinError> for SleipnirError {
    fn from(err: drasil_murin::MurinError) -> Self {
        SleipnirError::new(&err.to_string())
    }
}

impl From<drasil_murin::clib::error::JsError> for SleipnirError {
    fn from(err: drasil_murin::clib::error::JsError) -> Self {
        SleipnirError::new(&err.to_string())
    }
}

impl From<drasil_gungnir::RWDError> for SleipnirError {
    fn from(err: drasil_gungnir::RWDError) -> Self {
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

impl From<drasil_mimir::MimirError> for SleipnirError {
    fn from(err: drasil_mimir::MimirError) -> Self {
        SleipnirError::new(&err.to_string())
    }
}

impl From<drasil_hugin::error::SystemDBError> for SleipnirError {
    fn from(err: drasil_hugin::error::SystemDBError) -> Self {
        SleipnirError::new(&err.to_string())
    }
}
