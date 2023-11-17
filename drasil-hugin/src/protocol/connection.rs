use super::frame::Frame;
use async_recursion::async_recursion;
use bytes::{Buf, BytesMut};
use drasil_murin::MurinError;
use std::io::Cursor;
use tokio::io::BufWriter;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug)]
pub struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Connection {
        Connection {
            stream: BufWriter::new(stream),
            buffer: BytesMut::with_capacity(50 * 1024),
        }
    }

    pub async fn read_frame(&mut self) -> crate::Result<Option<Frame>> {
        loop {
            log::trace!("new read loop cycle");
            if let Some(frame) = self.parse_frame().await? {
                log::trace!("found some frame: {:?}", &frame);
                return Ok(Some(frame));
            }
            if 0 == self.stream.read_buf(&mut self.buffer).await? {
                log::trace!("Connection::read frame {:?}", &self.buffer);
                if self.buffer.is_empty() {
                    log::trace!("Buffer is empty: Ok");
                    return Ok(None);
                } else {
                    return Err(format!("error connection reset by peer: {:?}", self.buffer).into());
                }
            }
            
        }
    }

    async fn parse_frame(&mut self) -> crate::Result<Option<Frame>> {
        let mut buf = Cursor::new(&self.buffer[..]);
        trace!("parse_frame: {:?}", &self.buffer[..]);
        match Frame::check(&mut buf) {
            Ok(_) => {
                let len = buf.position() as usize;
                buf.set_position(0);

                let frame = Frame::parse(&mut buf).map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;

                self.buffer.advance(len);
                Ok(Some(frame))
            }
            Err(super::frame::Error::Incomplete) => Ok(None),
            Err(e) => Err(e.to_string().into()),
        }
    }

    pub async fn write_frame(&mut self, frame: &Frame) -> io::Result<()> {
        log::trace!("Frame::write_frame: {:?}", frame);
        match frame {
            Frame::Array(val) => {
                log::trace!("write_frame ARRAY: {:?}", val.clone());
                self.stream.write_u8(b'*').await?;
                self.write_decimal(val.len() as u64).await?;
                for entry in val {
                    self.write_value(entry).await?;
                }
            }
            _ =>{ log::trace!("Frame::write_frame: OTHER!!! ");
                self.write_value(frame).await?
            },
        }
        log::trace!("Frame::write_frame: Flush self.stream");
        self.stream.flush().await
    }

    #[async_recursion]
    async fn write_array(&mut self, val: &Vec<Frame>) -> io::Result<()> {
        self.stream.write_u8(b'*').await?;
        self.write_decimal(val.len() as u64).await?;
        for entry in &**val {
            self.write_value(entry).await?;
        }
        Ok(())
    }

    async fn write_value(&mut self, frame: &Frame) -> io::Result<()> {
        match frame {
            Frame::Simple(val) => {
                self.stream.write_u8(b'+').await?;
                self.stream.write_all(val.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Error(val) => {
                self.stream.write_u8(b'-').await?;
                self.stream.write_all(val.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Integer(val) => {
                self.stream.write_u8(b':').await?;
                self.write_decimal(*val).await?;
            }
            Frame::Null => {
                self.stream.write_all(b"$-1\r\n").await?;
            }
            Frame::Bulk(val) => {
                let len = val.len();

                self.stream.write_u8(b'$').await?;
                self.write_decimal(len as u64).await?;
                self.stream.write_all(val).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Array(val) => self.write_array(val).await?,
        }

        Ok(())
    }

    async fn write_decimal(&mut self, val: u64) -> io::Result<()> {
        use std::io::Write;

        let mut buf = [0u8; 12];
        let mut buf = Cursor::new(&mut buf[..]);
        write!(&mut buf, "{val}")?;

        let pos = buf.position() as usize;
        self.stream.write_all(&buf.get_ref()[..pos]).await?;
        self.stream.write_all(b"\r\n").await?;

        Ok(())
    }
}
