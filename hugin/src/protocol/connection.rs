/// Hugin Connection
use super::frame::{self, Frame};
use async_recursion::async_recursion;
use bytes::{Buf, BytesMut};
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
            if let Some(frame) = self.parse_frame().await? {
                return Ok(Some(frame));
            }
            /*
            match self.parse_frame().await? {
                Ok(hf) => if let Some(frame) = hf {
                    return Ok(Some(frame));
                },
                Err(e) => Err(e.into())
            }
            */
            if 0 == self.stream.read_buf(&mut self.buffer).await? {
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err("connection reset by perr".into());
                }
            }
        }
    }

    async fn parse_frame(&mut self) -> crate::Result<Option<Frame>> {
        use frame::Error::Incomplete;
        let mut buf = Cursor::new(&self.buffer[..]);

        match Frame::check(&mut buf) {
            Ok(_) => {
                let len = buf.position() as usize;
                buf.set_position(0);

                let frame = Frame::parse(&mut buf)?;

                self.buffer.advance(len);
                Ok(Some(frame))
            }
            Err(Incomplete) => (Ok(None)),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn write_frame(&mut self, frame: &Frame) -> io::Result<()> {
        match frame {
            Frame::Array(val) => {
                self.stream.write_u8(b'*').await?;
                self.write_decimal(val.len() as u64).await?;
                for entry in &**val {
                    self.write_value(entry).await?;
                }
            }
            _ => self.write_value(frame).await?,
        }
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
            //  Frame::Header{method, uri, version, token} => {
            //      self.stream.write_u8(b'?').await?;
            //      let header_in_byte = 5; //get_header_bytes(method,uri,version,token);
            //      self.write_decimal(header_in_byte); //.len());
            //  },
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
        write!(&mut buf, "{}", val)?;

        let pos = buf.position() as usize;
        self.stream.write_all(&buf.get_ref()[..pos]).await?;
        self.stream.write_all(b"\r\n").await?;

        Ok(())
    }
}
