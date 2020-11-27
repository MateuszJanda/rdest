use crate::frame::{Frame, Serializer};
use crate::Error;
use bytes::{Buf, BytesMut};
use std::io::Cursor;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct Connection {
    pub addr: String,
    pub stream: TcpStream,
    pub buffer: BytesMut,
}

const BUFFER_SIZE: usize = 65536;

impl Connection {
    pub fn new(addr: String, stream: TcpStream) -> Connection {
        Connection {
            addr,
            stream,
            buffer: BytesMut::with_capacity(BUFFER_SIZE),
        }
    }

    pub async fn send_frame(&mut self, frame: &Frame) -> Result<(), Box<dyn std::error::Error>> {
        match frame {
            Frame::Handshake(msg) => self.send_msg(msg).await?,
            Frame::KeepAlive(msg) => self.send_msg(msg).await?,
            Frame::Choke(msg) => self.send_msg(msg).await?,
            Frame::Unchoke(msg) => self.send_msg(msg).await?,
            Frame::Interested(msg) => self.send_msg(msg).await?,
            Frame::NotInterested(msg) => self.send_msg(msg).await?,
            Frame::Have(msg) => self.send_msg(msg).await?,
            Frame::Bitfield(msg) => self.send_msg(msg).await?,
            Frame::Request(msg) => self.send_msg(msg).await?,
            Frame::Piece(msg) => self.send_msg(msg).await?,
            Frame::Cancel(msg) => self.send_msg(msg).await?,
        }

        Ok(())
    }

    pub async fn send_msg<T: Serializer>(
        &mut self,
        msg: &T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.write_all(msg.data().as_slice()).await?;

        Ok(())
    }

    pub async fn recv_frame(&mut self) -> Result<Option<Frame>, Error> {
        loop {
            if let Some(frame) = self.parse_frame()? {
                println!("Now ramka {:?}", frame);
                return Ok(Some(frame));
            }

            let n = match self.stream.read_buf(&mut self.buffer).await {
                Err(_) => return Err(Error::CantReadFromSocket),
                Ok(n) => n,
            };

            if n == 0 {
                return if self.buffer.is_empty() {
                    println!("Peer zamknął połączenie");
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
                // Discard the frame for unknown message from the buffer
                let len = crs.position() as usize;
                self.buffer.advance(len);

                Ok(None)
            }
            Err(Error::Incomplete(_)) => {
                // Not enough data has been buffered
                Ok(None)
            }
            Err(e) => Err(e.into()),
        }
    }
}
