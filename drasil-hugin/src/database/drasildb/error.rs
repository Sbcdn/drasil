use thiserror::Error;

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum SystemDBError {
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
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    FloatParseError(#[from] std::num::ParseFloatError),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    BoolParseError(#[from] std::str::ParseBoolError),
    #[error(transparent)]
    CmdError(#[from] crate::CmdError),
}

impl From<std::string::String> for SystemDBError {
    fn from(err: std::string::String) -> Self {
        SystemDBError::Custom(err)
    }
}

impl From<drasil_murin::clib::error::JsError> for SystemDBError {
    fn from(err: drasil_murin::clib::error::JsError) -> Self {
        SystemDBError::Custom(err.to_string())
    }
}

impl From<drasil_murin::clib::error::DeserializeError> for SystemDBError {
    fn from(err: drasil_murin::clib::error::DeserializeError) -> Self {
        SystemDBError::Custom(err.to_string())
    }
}

impl From<argon2::password_hash::Error> for SystemDBError {
    fn from(err: argon2::password_hash::Error) -> Self {
        SystemDBError::Custom(err.to_string())
    }
}
