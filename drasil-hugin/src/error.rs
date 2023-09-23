use serde::Serialize;
use thiserror::Error;
use std::string::String;

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum Error {
    #[error("An error occured")]
    StdError,
    #[error("An error occured")]
    Custom(String),
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Error::Custom(err)
    }
}

#[derive(Serialize, Debug)]
struct ErrorResponse {
    message: String,
    status: String,
}
