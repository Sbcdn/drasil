use serde::Serialize;
use thiserror::Error;

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum Error {
    #[error("An error occured")]
    StdError,
    #[error("AN error occured")]
    Custom(String),
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
}

impl From<std::string::String> for Error {
    fn from(err: std::string::String) -> Self {
        Error::Custom(err)
    }
}

#[derive(Serialize, Debug)]
struct ErrorResponse {
    message: String,
    status: String,
}
