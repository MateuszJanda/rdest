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
        // let (read_stream, write_stream) = stream.split();
        Connection {
            addr,
            stream,
            // read_stream,
            // write_stream,
            buffer: BytesMut::with_capacity(BUFFER_SIZE),
        }
    }

    pub async fn init_frame<T: Serializer>(
        &mut self,
        msg: &T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // self.stream.read(&mut self.buffer[self.cursor..]).await?;

        self.stream.write_all(msg.data().as_slice()).await?;
        // self.stream.write_all(b"asdf").await?;

        println!("Handshake send");

        Ok(())
    }

    pub async fn write_frame(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.write_all(data).await?;

        Ok(())
    }

    pub async fn read_frame(&mut self) -> Result<Option<Frame>, Error> {
        // let n = self.stream.read_buf(&mut self.buffer).await?;
        // println!("read n {} {}", n, self.buffer.is_empty());
        // Ok(None)

        loop {
            println!("before check");
            if let Some(frame) = self.parse_frame()? {
                println!("is frame");
                return Ok(Some(frame));
            }

            println!("A before read");
            // let n = self.stream.read_buf(&mut self.buffer).await;
            // println!("po");
            // let n = n.unwrap();
            // println!("więc n {}", n);

            // let n = self.stream.read(&mut self.buffer).await?;
            let n = match self.stream.read_buf(&mut self.buffer).await {
                Err(e) => {
                    println!("tutaj");
                    println!("{:?}", e);
                    0
                }
                Ok(n) => {
                    println!("tutaj dobrze");
                    n
                }
            };
            println!("read n {} {}", n, self.buffer.is_empty());
            if n == 0 {
                return if self.buffer.is_empty() {
                    Ok(None)
                } else {
                    Err(Error::Peer("connection reset by peer".into()).into())
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

                // Return the frame to the caller.
                Ok(Some(frame))
            }
            // Not enough data has been buffered
            Err(Error::Incomplete) => {
                println!("Incomplete");
                Ok(None)
            }
            // An error was encountered
            Err(e) => Err(e.into()),
        }
    }
}
