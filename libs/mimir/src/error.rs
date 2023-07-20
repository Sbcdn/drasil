use thiserror::Error;

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum MimirError {
    #[error("DBSync Error")]
    DBSyncError(String),
    #[error("Custom Error")]
    Custom(String),
    #[error(transparent)]
    ParseIntError(#[from] core::num::ParseIntError),
    #[error(transparent)]
    DieselError(#[from] diesel::result::Error),
    #[error(transparent)]
    MurinError(#[from] murin::error::MurinError),
    #[error(transparent)]
    VarError(#[from] std::env::VarError),
    #[error(transparent)]
    DieselConnectionError(#[from] diesel::ConnectionError),
    #[error(transparent)]
    HexError(#[from] hex::FromHexError),
    #[error(transparent)]
    UTF8Error(#[from] std::string::FromUtf8Error),
    #[error("Could not find metadata for token")]
    NotOnChainMetadataFound,
}

impl From<std::string::String> for MimirError {
    fn from(err: std::string::String) -> Self {
        MimirError::Custom(err)
    }
}

impl From<murin::clib::error::JsError> for MimirError {
    fn from(err: murin::clib::error::JsError) -> Self {
        MimirError::Custom(err.to_string())
    }
}

impl From<murin::clib::error::DeserializeError> for MimirError {
    fn from(err: murin::clib::error::DeserializeError) -> Self {
        MimirError::Custom(err.to_string())
    }
}
