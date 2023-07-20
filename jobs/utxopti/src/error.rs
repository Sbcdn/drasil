use serde::Serialize;
use thiserror::Error;

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum UOError {
    #[error("wrong credentials")]
    JWTTokenError,
    #[error("jwt token creation error")]
    NoAuthHeaderError,
    #[error("invalid auth header")]
    InvalidAuthHeaderError,
    #[error("no permission")]
    Custom(String),
    #[error("Error on Odin request")]
    OdinError(String),
    #[error(transparent)]
    ParseIntError(#[from] core::num::ParseIntError),
    #[error(transparent)]
    RWDError(#[from] gungnir::error::RWDError),
    #[error(transparent)]
    MurinError(#[from] murin::error::MurinError),
    #[error(transparent)]
    HexError(#[from] hex::FromHexError),
    #[error(transparent)]
    DBSyncError(#[from] mimir::MimirError),
    #[error(transparent)]
    HuginError(#[from] hugin::error::SystemDBError),
}

impl From<murin::clib::error::JsError> for UOError {
    fn from(err: murin::clib::error::JsError) -> Self {
        UOError::Custom(err.to_string())
    }
}
impl From<murin::clib::error::DeserializeError> for UOError {
    fn from(err: murin::clib::error::DeserializeError) -> Self {
        UOError::Custom(err.to_string())
    }
}

#[derive(Serialize, Debug)]
struct ErrorResponse {
    message: String,
    status: String,
}
