use serde::Serialize;
use thiserror::Error;

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum Error {
    #[error("internal error: {:?}", self)]
    Custom(String),
    #[error("rmq error: {0}")]
    RMQError(#[from] lapin::Error),
    #[error("rmq pool error: {0}")]
    RMQPoolError(#[from] deadpool_lapin::PoolError),
    #[error("Utf8Error")]
    UTF8Error(#[from] std::str::Utf8Error),
    #[error("JsonError")]
    JsonError(#[from] serde_json::Error),
    #[error("MintAPIError: {0}")]
    MintAPIError(#[from] drasil_gungnir::RWDError),
    #[error("SCLError: {0}")]
    CSLError(#[from] drasil_murin::clib::error::JsError),
    #[error("MurinError: {0}")]
    MurinError(#[from] drasil_murin::MurinError),
    #[error("MimirError: {0}")]
    MimirError(#[from] drasil_mimir::MimirError),
    #[error("HuginError: {0}")]
    HuginError(#[from] drasil_hugin::error::SystemDBError),
}

#[derive(Serialize, Debug)]
struct ErrorResponse {
    message: String,
    status: String,
}

impl From<std::string::String> for Error {
    fn from(err: std::string::String) -> Self {
        Error::Custom(err)
    }
}
