use crate::frame::Serializer;
use crate::{Error, Frame};
use bytes::{Buf, BytesMut};
use std::io::Cursor;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct Connection {
    pub addr: String,
    pub stream: TcpStream,
    pub buffer: BytesMut,
}

const BUFFER_SIZE: usize = 65536 + 2;

impl Connection {
    pub fn new(addr: String, stream: TcpStream) -> Connection {
        Connection {
            addr,
            stream,
            buffer: BytesMut::with_capacity(BUFFER_SIZE),
        }
    }

    pub async fn write_frame<T: Serializer>(
        &mut self,
        msg: &T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.write_all(msg.data().as_slice()).await?;

        Ok(())
    }

    pub async fn read_frame(&mut self) -> Result<Option<Frame>, Error> {
        loop {
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            let n = match self.stream.read_buf(&mut self.buffer).await {
                Err(_) => return Err(Error::SocketWrite),
                Ok(n) => n,
            };

            if n == 0 {
                return if self.buffer.is_empty() {
                    Ok(None)
                } else {
                    Err(Error::ConnectionReset)
                };
            }
        }
    }

    fn parse_frame(&mut self) -> Result<Option<Frame>, Error> {
        let mut crs = Cursor::new(&self.buffer[..]);

        // Check whether a full frame is available
        match Frame::parse(&mut crs) {
            Ok(frame) => {
                // Discard the frame from the buffer
                let len = crs.position() as usize;
                self.buffer.advance(len);

                Ok(Some(frame))
            }
            Err(Error::UnknownId(_)) => {
                // Discard the frame from the buffer
                let len = crs.position() as usize;
                self.buffer.advance(len);

                Ok(None)
            }
            Err(Error::Incomplete) => {
                // Not enough data has been buffered
                Ok(None)
            }
            Err(e) => Err(e.into()),
        }
    }
}
