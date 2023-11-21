use serde::Serialize;


#[allow(clippy::enum_variant_names)]
#[derive(thiserror::Error, Debug, Clone)]
pub enum MurinError {
    #[error("error: {:?}", self)]
    Custom(String),
    #[error("error: {:?}", self)]
    ProtocolCommandError(String),
    #[error("Invalid data")]
    ProtocolCommandErrorInvalidData,
    #[error("frame check: could not get decimal")]
    ProtocolCommandErrorCouldNotGetDecimal,
    //#[error("{:}}",)]
    //Error(#[from] &str),
}

#[derive(Serialize, Debug)]
struct ErrorResponse {
    message: String,
    status: String,
}

impl From<&str> for MurinError {
    fn from(err: &str) -> Self {
        MurinError::Custom(err.to_owned())
    }
}

impl From<std::string::String> for MurinError {
    fn from(err: std::string::String) -> Self {
        MurinError::Custom(err)
    }
}

impl MurinError {
    pub fn new(msg: &str) -> MurinError {
        MurinError::Custom(msg.to_owned())
    }
}

unsafe impl Send for MurinError {}
unsafe impl Sync for MurinError {}

impl From<crate::modules::transfer::error::TransferError> for MurinError {
    fn from(err: crate::modules::transfer::error::TransferError) -> Self {
        MurinError::new(&err.to_string())
    }
}

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

impl From<bip39::Error> for MurinError {
    fn from(err: bip39::Error) -> Self {
        MurinError::new(&err.to_string())
    }
}

impl From<Box<bincode::ErrorKind>> for MurinError {
    fn from(err: Box<bincode::ErrorKind>) -> Self {
        MurinError::new(&err.to_string())
    }
}



