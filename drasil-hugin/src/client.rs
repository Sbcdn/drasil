use crate::protocol::{Connection, Frame, IntoFrame};
use bc::Options;
use bincode as bc;
use std::io::{Error, ErrorKind};
use tokio::net::{TcpStream, ToSocketAddrs};

pub struct Client {
    pub connection: Connection,
}

pub async fn connect<T: ToSocketAddrs>(addr: T) -> crate::Result<Client> {
    let socket = TcpStream::connect(addr).await?;
    let connection = Connection::new(socket);

    Ok(Client { connection })
}

impl Client {
    pub async fn build_cmd<T: IntoFrame>(&mut self, cmd: T) -> crate::Result<String> {
        let frame = cmd.into_frame();
        log::debug!("Send frame: {:?}", frame);
        self.connection.write_frame(&frame).await?;
        log::debug!("Read response ...");
        match self.read_response().await? {
            Frame::Simple(response) => Ok(response),
            Frame::Bulk(data) => Ok(bc::DefaultOptions::new()
                .with_varint_encoding()
                .deserialize::<String>(&data)?),
            frame => Err(frame.to_error()),
        }
    }

    async fn read_response(&mut self) -> crate::Result<Frame> {
        let response = self.connection.read_frame().await?;

        log::debug!("{:?}", response);

        match response {
            Some(Frame::Error(msg)) => Err(msg.into()),
            Some(frame) => Ok(frame),
            None => {
                let err = Error::new(ErrorKind::ConnectionReset, "connection aborted by server");

                Err(err.into())
            }
        }
    }
}
