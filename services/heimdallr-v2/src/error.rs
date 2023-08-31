//! This module defines the error types.

use std::io;

use thiserror;

/// Result an alias for the `Result` type with `self::Error` as error
pub type Result<T> = std::result::Result<T, Error>;

/// Error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// I/O related errors
    #[error(transparent)]
    IoError(#[from] io::Error),

    /// Improperly configuration errors
    #[error(transparent)]
    ConfigError(#[from] config::ConfigError),
}
