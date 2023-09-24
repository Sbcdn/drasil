use thiserror::Error;
use std::{
    io,
    num::{ParseIntError, ParseFloatError},
    str::ParseBoolError,
    env::VarError,
    string::String
};
use drasil_murin::clib;

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum SystemDBError {
    #[error("DBSync Error")]
    DBSyncError(String),
    #[error("Custom Error")]
    Custom(String),
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
    #[error(transparent)]
    DieselError(#[from] diesel::result::Error),
    #[error(transparent)]
    MurinError(#[from] drasil_murin::error::MurinError),
    #[error(transparent)]
    VarError(#[from] VarError),
    #[error(transparent)]
    DieselConnectionError(#[from] diesel::ConnectionError),
    #[error(transparent)]
    HexError(#[from] hex::FromHexError),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    FloatParseError(#[from] ParseFloatError),
    #[error(transparent)]
    IOError(#[from] io::Error),
    #[error(transparent)]
    BoolParseError(#[from] ParseBoolError),
    #[error(transparent)]
    CmdError(#[from] crate::CmdError),
}

impl From<std::string::String> for SystemDBError {
    fn from(err: String) -> Self {
        SystemDBError::Custom(err)
    }
}

impl From<clib::error::JsError> for SystemDBError {
    fn from(err: clib::error::JsError) -> Self {
        SystemDBError::Custom(err.to_string())
    }
}

impl From<clib::error::DeserializeError> for SystemDBError {
    fn from(err: clib::error::DeserializeError) -> Self {
        SystemDBError::Custom(err.to_string())
    }
}

impl From<argon2::password_hash::Error> for SystemDBError {
    fn from(err: argon2::password_hash::Error) -> Self {
        SystemDBError::Custom(err.to_string())
    }
}
