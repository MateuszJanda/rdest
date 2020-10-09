// use rdest::BValue;
use rdest::{Torrent, ResponseParser};
// use rdest::TrackerClient;
// use hex_literal::hex;
use tokio;
use tokio::net::{TcpListener, TcpStream};
use std::io::Cursor;

// fn main() {
//     println!("Hello, world!");
//     // let b = BValue::parse(b"i4e").unwrap();
//     let _t = Torrent::from_file(String::from("ubuntu-20.04.1-desktop-amd64.iso.torrent"));
//     // println!("{:?}", t);
//
//     // match TrackerClient::connect1(&t.unwrap()) {
//     //     Ok(_) => println!("Http Ok"),
//     //     Err(e) => println!("Http Problem {:?}", e),
//     // }
//
//     println!("{:?}", ResponseParser::from_file("response.data".to_string()));
// }


#[tokio::main]
async fn main() {
    println!("Hello, world!");

    let mut listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    loop {
        // The second item contains the IP and port of the new connection.
        let (socket, _) = listener.accept().await.unwrap();
        println!("accept");
        // process(socket).await;
    }

}

enum Frame {
    Handshake,
    KeepAlive,
}

struct Connection {
    stream: TcpStream,
    buffer: Vec<u8>,
    cursor: usize,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Connection {
        Connection {
            stream,
            buffer: vec![0; 65536],
            cursor: 0,
        }
    }

    pub async fn read_frame(&mut self)
                            -> Result<Option<Frame>, String>
    {
        loop {
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            // // Ensure the buffer has capacity
            // if self.buffer.len() == self.cursor {
            //     // Grow the buffer
            //     self.buffer.resize(self.cursor * 2, 0);
            // }

            // Read into the buffer, tracking the number
            // of bytes read

            // let n = self.stream.read(
            //     &mut self.buffer[self.cursor..]).await?;
            let n = 0;

            if 0 == n {
                if self.cursor == 0 {
                    return Ok(None);
                } else {
                    return Err("connection reset by peer".into());
                }
            } else {
                // Update our cursor
                self.cursor += n;
            }
        }
    }


    fn parse_frame(&mut self)
                   -> Result<Option<Frame>, String>
    {
        // Create the `T: Buf` type.
        let mut buf = Cursor::new(&self.buffer[..]);

        // Check whether a full frame is available
        match Frame::check(&mut buf) {
            Ok(_) => {
                // Get the byte length of the frame
                let len = buf.position() as usize;

                // Reset the internal cursor for the
                // call to `parse`.
                buf.set_position(0);

                // Parse the frame
                let frame = Frame::parse(&mut buf)?;

                // Discard the frame from the buffer
                self.buffer.advance(len);

                // Return the frame to the caller.
                Ok(Some(frame))
            }
            // Not enough data has been buffered
            Err(Incomplete) => Ok(None),
            // An error was encountered
            Err(e) => Err(e.into()),
        }
    }
}

impl Frame {
    pub fn check(src: &mut Cursor<&[u8]>) -> Result<(), String> {
        match get_prefix_length(src)? {
            b'+' => {
                get_line(src)?;
                Ok(())
            }
            b'-' => {
                get_line(src)?;
                Ok(())
            }
            b':' => {
                let _ = get_decimal(src)?;
                Ok(())
            }
            b'$' => {
                if b'-' == peek_u8(src)? {
                    // Skip '-1\r\n'
                    skip(src, 4)
                } else {
                    // Read the bulk string
                    let len: usize = get_decimal(src)?.try_into()?;

                    // skip that number of bytes + 2 (\r\n).
                    skip(src, len + 2)
                }
            }
            b'*' => {
                let len = get_decimal(src)?;

                for _ in 0..len {
                    Frame::check(src)?;
                }

                Ok(())
            }
            actual => Err(format!("protocol error; invalid frame type byte `{}`", actual).into()),
        }
    }

    fn get_prefix_length<'a>(src: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], Error> {
        // Scan the bytes directly
        let start = src.position() as usize;
        // Scan to the second to last byte
        let end = src.get_ref().len() - 1;

        if end - start >= 2 {
            let a = src.get_ref()[start..start+2];
            let v = u16::from_le_bytes(src.get_ref()[start..start+2]);
        }

        Err(Error::Incomplete)
    }

    // /// Find a line
    // fn get_line<'a>(src: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], Error> {
    //     // Scan the bytes directly
    //     let start = src.position() as usize;
    //     // Scan to the second to last byte
    //     let end = src.get_ref().len() - 1;
    //
    //     for i in start..end {
    //         if src.get_ref()[i] == b'\r' && src.get_ref()[i + 1] == b'\n' {
    //             // We found a line, update the position to be *after* the \n
    //             src.set_position((i + 2) as u64);
    //
    //             // Return the line
    //             return Ok(&src.get_ref()[start..i]);
    //         }
    //     }
    //
    //     Err(Error::Incomplete)
    // }

}

