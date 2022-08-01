/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use crate::protocol::{Connection, Frame, IntoFrame};
use bc::Options;
use bincode as bc;
use std::io::{Error, ErrorKind};
use tokio::net::{TcpStream, ToSocketAddrs};
use tracing::debug;

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
        self.connection.write_frame(&frame).await?;

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

        debug!(?response);

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
