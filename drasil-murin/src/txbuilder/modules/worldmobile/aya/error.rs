use cardano_serialization_lib as csl;

use thiserror::Error;
use csl::error::JsError;
use csl::error::DeserializeError;

use super::enregistration;

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum CTSError {
    #[error("wrong credentials")]
    JWTTokenError,
    #[error("jwt token creation error")]
    NoAuthHeaderError,
    #[error("invalid auth header")]
    InvalidAuthHeaderError,
    #[error("no permission")]
    Custom(String),
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("wrong transaction pattern")]
    TxSchemaError,
    #[error("could not resolve standtand transaction")]
    StandardTransactionError,
    #[error("error during transaction building")]
    TransactionBuildingError(#[from] enregistration::error::TransactionBuildingError),
    #[error("no operation parameter provided")]
    NoOperation,
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    MurinError(#[from] crate::MurinError),
}

impl From<JsError> for CTSError {
    fn from(err: JsError) -> Self {
        CTSError::Custom(err.to_string())
    }
}
impl From<DeserializeError> for CTSError {
    fn from(err: DeserializeError) -> Self {
        CTSError::Custom(err.to_string())
    }
}
