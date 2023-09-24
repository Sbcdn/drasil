use drasil_murin::clib;
use thiserror::Error;
use std::string::String;
use clib::error::{DeserializeError, JsError};

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum MimirError {
    #[error("DBSync Error")]
    DBSyncError(String),
    #[error("Custom Error")]
    Custom(String),
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error(transparent)]
    DieselError(#[from] diesel::result::Error),
    #[error(transparent)]
    MurinError(#[from] drasil_murin::error::MurinError),
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

impl From<String> for MimirError {
    fn from(err: String) -> Self {
        MimirError::Custom(err)
    }
}

impl From<JsError> for MimirError {
    fn from(err: JsError) -> Self {
        MimirError::Custom(err.to_string())
    }
}

impl From<DeserializeError> for MimirError {
    fn from(err: DeserializeError) -> Self {
        MimirError::Custom(err.to_string())
    }
}
