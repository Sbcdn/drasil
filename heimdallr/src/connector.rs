extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use async_stream::try_stream;
use bytes::Bytes;
use hugin::protocol::{commands, connection::Connection, frame::Frame};
use std::io::{Error, ErrorKind};
use std::time::Duration;
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio_stream::Stream;
use tracing::{debug, instrument};

pub mod client {}
