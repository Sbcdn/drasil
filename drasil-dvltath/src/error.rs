use serde::Serialize;
use thiserror::Error;
use std::string::String;
use std::num::ParseIntError;

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum Error {
    #[error("An error occured")]
    StdError,
    #[error("AN error occured")]
    Custom(String),
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
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
