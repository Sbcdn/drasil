use std::fmt;

use drasil_murin::MurinError;

#[derive(Debug)]
pub enum CmdError {
    InvalidCmd,
    InvalidData,
    Custom { str: String },
    Other(MurinError),
}

impl From<String> for CmdError {
    fn from(src: String) -> CmdError {
        CmdError::Other(src.into())
    }
}

impl From<&str> for CmdError {
    fn from(src: &str) -> CmdError {
        src.to_string().into()
    }
}

impl From<drasil_murin::clib::error::DeserializeError> for CmdError {
    fn from(src: drasil_murin::clib::error::DeserializeError) -> CmdError {
        src.to_string().into()
    }
}

impl From<drasil_murin::error::MurinError> for CmdError {
    fn from(src: drasil_murin::error::MurinError) -> CmdError {
        src.to_string().into()
    }
}

impl From<drasil_mimir::MimirError> for CmdError {
    fn from(src: drasil_mimir::MimirError) -> CmdError {
        src.to_string().into()
    }
}

impl std::error::Error for CmdError {}

impl fmt::Display for CmdError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CmdError::InvalidCmd => "Invalid command provided".fmt(fmt),
            CmdError::InvalidData => "Invalid data provided".fmt(fmt),
            CmdError::Custom { str } => str.fmt(fmt),
            CmdError::Other(err) => err.fmt(fmt),
        }
    }
}
