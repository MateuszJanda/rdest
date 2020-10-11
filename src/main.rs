// use rdest::BValue;
use rdest::{Torrent, ResponseParser};
// use rdest::TrackerClient;
// use hex_literal::hex;
use tokio;
use tokio::net::{TcpListener, TcpStream};
use std::io::Cursor;
use std::fmt;
use tokio::io::BufReader;
// use tokio::io::util::async_read_ext::AsyncReadExt;
use tokio::io::AsyncReadExt;


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

        let connection = Connection::new(socket);

        let mut handler = Handler {
            connection: connection
        };

        tokio::spawn(async move {
            // Process the connection. If an error is encountered, log it.
            if let Err(err) =
            handler.run().await {
                // error!(cause = ?err, "connection error");
                panic!("asdf");
            }
        });
    }

}

struct Handler {
    connection: Connection,
}

impl Handler {
    async fn run(&mut self) -> Result<(), Error> {
        loop {
            let res = self.connection.read_frame().await?;
        }

        Ok(())
    }
}

enum Frame {
    Handshake,
    KeepAlive,
    Choke,
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
                            -> Result<Option<Frame>, Error>
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

            // let mut stream = BufReader::new(self.stream);

            let n = self.stream.read(
                &mut self.buffer[self.cursor..]).await?;


            // let mut line = String::new();
            // stream.read_line(&mut line).await.unwrap();

            // let mut bb = [20; 0];
            // self.stream.read(&mut bb).await?;

            // let mut line = String::new();
            // self.stream.read_line(&mut line).await.unwrap();

            // self.stream.read_buf(&mut self.buffer[self.cursor..]).await?;
            // self.stream.read_exact(&mut self.buffer[self.cursor..]).await?;
            let n = 0;

            if 0 == n {
                if self.cursor == 0 {
                    return Ok(None);
                } else {
                    return Err(Error::S("connection reset by peer".into()));
                }
            } else {
                // Update our cursor
                self.cursor += n;
            }
        }
    }


    fn parse_frame(&mut self)
                   -> Result<Option<Frame>, Error>
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
                self.buffer.drain(..len);
                self.buffer.resize(65536, 0);

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
    pub fn check(src: &mut Cursor<&[u8]>) -> Result<(), Error> {
        if Self::get_length(src)? == 0 {
            return Ok(())
        }
        match Self::get_id(src)? {
            0 => Ok(()),
            _ => Err(Error::S("fuck".into()))
        }
    }

    fn get_length(src: &Cursor<&[u8]>) -> Result<u16, Error> {
        let start = src.position() as usize;
        let end = src.get_ref().len();

        if end - start >= 2 {
            // let b : [u8; 2] = src.get_ref()[0..2];
            let b = [src.get_ref()[0], src.get_ref()[1]];
            return Ok(u16::from_be_bytes(b));
        }

        Err(Error::Incomplete)
    }


    fn get_id(src: &Cursor<&[u8]>) -> Result<u8, Error> {
        let start = src.position() as usize;
        let end = src.get_ref().len();

        if end - start >= 3 {
            return Ok(src.get_ref()[3]);
        }

        Err(Error::Incomplete)
    }

    pub fn parse(src: &mut Cursor<&[u8]>) -> Result<Frame, Error>
    {
        Ok(Frame::Choke)
    }
}

#[derive(Debug)]
enum Error {
    Incomplete,
    S(String),
    I(std::io::Error)
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Incomplete => write!(f, "dddd"),
            Error::S(s) => write!(f, "ssss"),
            Error::I(i) =>  write!(f, "iii"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::I(error)
    }
}