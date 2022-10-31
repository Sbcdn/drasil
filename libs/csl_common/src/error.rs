use cardano_serialization_lib as csl;

use thiserror::Error;

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum CSLCommonError {
    #[error("general error")]
    General,
    #[error("error: {:?}", self)]
    Custom(String),
    #[error(transparent)]
    ParseIntError(#[from] core::num::ParseIntError),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("serialization lib error")]
    CSLError,
    #[error(transparent)]
    HexError(#[from] hex::FromHexError),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    EnvirontmentError(#[from] std::env::VarError),
    #[error(transparent)]
    CborError(#[from] cbor_event::Error),
}

impl From<csl::error::JsError> for CSLCommonError {
    fn from(err: csl::error::JsError) -> Self {
        CSLCommonError::Custom(err.to_string())
    }
}
impl From<csl::error::DeserializeError> for CSLCommonError {
    fn from(err: csl::error::DeserializeError) -> Self {
        CSLCommonError::Custom(err.to_string())
    }
}
