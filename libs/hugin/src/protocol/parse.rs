use crate::Frame;

use bytes::Bytes;
use std::{fmt, str, vec};

#[derive(Debug)]
pub(crate) struct Parse {
    parts: vec::IntoIter<Frame>,
}

#[derive(Debug)]
pub(crate) enum CmdParseError {
    EndOfStream,
    Other(crate::Error),
}

impl Parse {
    pub(crate) fn new(frame: Frame) -> Result<Parse, CmdParseError> {
        let array = match frame {
            Frame::Array(array) => array,
            frame => {
                return Err(format!("protocol error; epected array type, got {:?}", frame).into())
            }
        };

        Ok(Parse {
            parts: array.into_iter(),
        })
    }

    fn next(&mut self) -> Result<Frame, CmdParseError> {
        self.parts.next().ok_or(CmdParseError::EndOfStream)
    }

    pub(crate) fn next_string(&mut self) -> Result<String, CmdParseError> {
        match self.next()? {
            Frame::Simple(s) => Ok(s),
            Frame::Bulk(data) => str::from_utf8(&data[..])
                .map(|s| s.to_string())
                .map_err(|_| "protocol error; invalid string".into()),
            frame => Err(format!(
                "protocol error; expected simple frame or bulk frame, got {:?}",
                frame
            )
            .into()),
        }
    }

    pub(crate) fn next_bytes(&mut self) -> Result<Bytes, CmdParseError> {
        match self.next()? {
            Frame::Simple(s) => Ok(Bytes::from(s.into_bytes())),
            Frame::Bulk(data) => Ok(data),
            frame => Err(format!(
                "protocol error; expected simple frame or bulk frame, got {:?}",
                frame
            )
            .into()),
        }
    }

    pub(crate) fn next_int(&mut self) -> Result<u64, CmdParseError> {
        use atoi::atoi;

        const MSG: &str = "protocol error; invalid number";

        match self.next()? {
            Frame::Integer(v) => Ok(v),
            Frame::Simple(data) => atoi::<u64>(data.as_bytes()).ok_or_else(|| MSG.into()),
            Frame::Bulk(data) => atoi::<u64>(&data).ok_or_else(|| MSG.into()),
            frame => Err(format!("protocol error; expected int frame but got {:?}", frame).into()),
        }
    }

    /// Ensure there are no more entries in the array
    pub(crate) fn finish(&mut self) -> Result<(), CmdParseError> {
        if self.parts.next().is_none() {
            Ok(())
        } else {
            Err("protocol error; expected end of frame".into())
        }
    }
}

impl From<String> for CmdParseError {
    fn from(src: String) -> CmdParseError {
        CmdParseError::Other(src.into())
    }
}

impl From<&str> for CmdParseError {
    fn from(src: &str) -> CmdParseError {
        src.to_string().into()
    }
}

impl fmt::Display for CmdParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CmdParseError::EndOfStream => "protocol error; unexpected end of stream".fmt(f),
            CmdParseError::Other(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for CmdParseError {}
