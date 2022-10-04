use std::fmt;

#[derive(Debug)]
pub enum CmdError {
    InvalidCmd,
    InvalidData,
    Custom { str: String },
    Other(crate::Error),
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
impl From<murin::clib::error::DeserializeError> for CmdError {
    fn from(src: murin::clib::error::DeserializeError) -> CmdError {
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
