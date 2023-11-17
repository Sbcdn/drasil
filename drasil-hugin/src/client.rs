use crate::protocol::{Connection, Frame, IntoFrame};
use bc::Options;
use bincode as bc;
use std::io::{Error, ErrorKind};
use tokio::net::{TcpStream, ToSocketAddrs};

/// Client connected to a server via TCP connection.
pub struct Client {
    pub connection: Connection,
}

/// Create a client connected to the given address
pub async fn connect<T: ToSocketAddrs>(addr: T) -> crate::Result<Client> {
    let socket = TcpStream::connect(addr).await?;
    let connection = Connection::new(socket);

    Ok(Client { connection })
}

impl Client {
    /// Sends command to server and retrieves response. 
    /// 
    /// If the response is serialized, then it will deserialize it first.
    /// If the server fails to give non-error response, then this method throws an error.
    pub async fn build_cmd<T: IntoFrame>(&mut self, cmd: T) -> crate::Result<String> {
        let frame = cmd.into_frame();
        self.connection.write_frame(&frame).await?;

        match self.read_response().await? {
            Frame::Simple(response) => Ok(response),
            Frame::Bulk(data) => Ok(bc::DefaultOptions::new()
                .with_varint_encoding()
                .deserialize::<String>(&data)?),
            frame => Err(frame.to_error()),
        }
    }

    /// Reads frame in TCP connection.
    /// 
    /// It throws error if the peer sent an error message or if the peer has reset connection.
    async fn read_response(&mut self) -> crate::Result<Frame> {
        let response = self.connection.read_frame().await?;

        log::debug!("{:?}", response);

        match response {
            Some(Frame::Error(msg)) => Err(msg.into()),
            Some(frame) => Ok(frame),
            None => {
                let err = Error::new(ErrorKind::ConnectionReset, "connection reset by server");

                Err(err.into())
            }
        }
    }
}
