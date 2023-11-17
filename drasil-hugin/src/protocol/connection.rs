use super::frame::{self, Frame};
use async_recursion::async_recursion;
use bytes::{Buf, BytesMut};
use std::io::Cursor;
use tokio::io::BufWriter;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Buffered TCP connection.
#[derive(Debug)]
pub struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    /// Creates a buffered TCP connection from a TCP stream.
    pub fn new(stream: TcpStream) -> Connection {
        Connection {
            stream: BufWriter::new(stream),
            buffer: BytesMut::with_capacity(50 * 1024),
        }
    }

    /// Reads frame from the peer in the TCP connection.
    pub async fn read_frame(&mut self) -> crate::Result<Option<Frame>> {
        loop {
            if let Some(frame) = self.parse_frame().await? {
                return Ok(Some(frame));
            }
            if 0 == self.stream.read_buf(&mut self.buffer).await? {
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err("connection reset by perr".into());
                }
            }
        }
    }

    /// Tries to convert the TCP connection's internal buffer into a frame.
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
            Err(Incomplete) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Writes frame to the TCP connection.
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

    /// Writes an array `val` of frames to the TCP connection.
    /// 
    /// It writes the following:
    /// 1) `*` character 
    /// 2) array length
    /// 3) the array contents
    /// 
    /// To write the array contents, this method will loop through each array 
    /// element, calling `write_value(...)` each time. 
    #[async_recursion]
    async fn write_array(&mut self, val: &Vec<Frame>) -> io::Result<()> {
        self.stream.write_u8(b'*').await?;
        self.write_decimal(val.len() as u64).await?;
        for entry in &**val {
            self.write_value(entry).await?;
        }
        Ok(())
    }

    /// Writes the inner value of a frame to the TCP connection. 
    /// 
    /// It handles the frame differently depending on the frame variant:
    /// * `Simple`: 
    ///     1) write `+` character
    ///     2) write the inner value of frame
    ///     3) write `\r\n` (which starts a new line)
    /// * `Error`:
    ///     1) write `-` character
    ///     2) write the inner value of the frame
    ///     3) write `\r\n` (which starts a new line)
    /// * `Integer`:
    ///     1) write `:` character
    ///     2) write the inner value of the frame
    ///     3) write `\r\n` (which starts a new line)
    /// * `Null`:
    ///     1) write `$-1\r\n`
    /// * `Bulk`:
    ///     1) write `$`
    ///     2) write the number of bytes taken by frame's inner value
    ///     3) write the frame's inner value
    ///     4) write `\r\n` (which starts a new line)
    /// * `Array`:
    ///     1) write `*`
    ///     2) write array length
    ///     3) write inner content of array (recursively calling `write_value`)
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

    /// Writes an integer `val` into the TCP connection, and starts a new line. 
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
