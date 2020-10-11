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
use std::net::Ipv4Addr;
use std::convert::{TryFrom, TryInto};


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

    // let mut listener = TcpListener::bind("127.0.0.1:6881").await.unwrap();
    let mut listener = TcpListener::bind((Ipv4Addr::new(0, 0, 0, 0), 6881)).await.unwrap();

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
    Unchoke,
    Interested,
    NotInterested,
    Have,
    Bitfield,
    Request,
    Piece,
    Cancel,
    Port,
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
                // buf.set_position(0);

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

enum MessageId {
    Choke = 0,
    Unchoke = 1,
    Interested = 2,
    NotInterested = 3,
    Have = 4,
    Bitfield = 5,
    Request = 6,
    Piece = 7,
    Cancel = 8,
    Port = 9,
}

impl TryFrom<u8> for MessageId {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == MessageId::Choke as u8 => Ok(MessageId::Choke),
            x if x == MessageId::Unchoke as u8 => Ok(MessageId::Unchoke),
            x if x == MessageId::Interested as u8 => Ok(MessageId::Interested),
            x if x == MessageId::NotInterested as u8 => Ok(MessageId::NotInterested),
            x if x == MessageId::Have as u8 => Ok(MessageId::Have),
            x if x == MessageId::Bitfield as u8 => Ok(MessageId::Bitfield),
            x if x == MessageId::Request as u8 => Ok(MessageId::Request),
            x if x == MessageId::Piece as u8 => Ok(MessageId::Piece),
            x if x == MessageId::Cancel as u8 => Ok(MessageId::Cancel),
            x if x == MessageId::Port as u8 => Ok(MessageId::Port),
            _ => Err(()),
        }
    }
}

const LENGTH_FIELD_LEN: usize = 2;
const ID_FIELD_LEN: usize = 1;

const HANDSHAKE_PSTR_LEN: usize = 19;

const KEEP_ALIVE_LEN: usize = 0;
const CHOKE_LEN: usize = 1;
const UNCHOKE_LEN: usize = 1;
const INTERESTED_LEN: usize = 1;
const NOT_INTERESTED_LEN: usize = 1;
const HAVE_LEN: usize = 5;
const REQUEST_LEN: usize = 13;
const CANCEL_LEN: usize = 13;
const PORT_LEN: usize = 3;

const MIN_PIECE_LEN: usize = 9;

const HANDSHAKE: &[u8; 19] = b"BitTorrent protocol";

impl Frame {
    pub fn check(src: &mut Cursor<&[u8]>) -> Result<(), Error> {
        let length = Self::get_message_length(src)?;
        if length == KEEP_ALIVE_LEN {
            return Ok(())
        }

        let msg_id = Self::get_message_id(src)?;

        if msg_id == b'i' && Self::get_handshake_length(src)? == HANDSHAKE_PSTR_LEN && Self::available_data(src) - 1 >= HANDSHAKE_PSTR_LEN {
            for idx in 0..HANDSHAKE.len() {
                if src.get_ref()[idx + 1] != HANDSHAKE[idx] {
                    return Err(Error::S("nope".into()))
                }
            }

            return Ok(())
        }

        let available_data = Self::available_data(src) - LENGTH_FIELD_LEN;
        match msg_id.try_into() {
            Ok(MessageId::Choke) => Ok(()),
            Ok(MessageId::Unchoke) => Ok(()),
            Ok(MessageId::Interested) => Ok(()),
            Ok(MessageId::NotInterested) => Ok(()),
            Ok(MessageId::Have) => Ok(()),
            Ok(MessageId::Bitfield) if available_data  >= length => Ok(()),
            Ok(MessageId::Request) => Ok(()),
            Ok(MessageId::Piece) if available_data >= length => Ok(()),
            Ok(MessageId::Cancel) if length == CANCEL_LEN => Ok(()),
            Ok(MessageId::Port) if length == PORT_LEN => Ok(()),
            _ => Err(Error::S("fuck".into()))
        }
    }

    fn get_handshake_length(src: &Cursor<&[u8]>) -> Result<usize, Error> {
        let start = src.position() as usize;
        let end = src.get_ref().len();

        if end - start >= 1 {
            return Ok(src.get_ref()[0] as usize);
        }

        Err(Error::Incomplete)
    }

    fn get_message_length(src: &Cursor<&[u8]>) -> Result<usize, Error> {
        let start = src.position() as usize;
        let end = src.get_ref().len();

        if end - start >= LENGTH_FIELD_LEN as usize {
            // let b : [u8; 2] = src.get_ref()[0..2];
            let b = [src.get_ref()[0], src.get_ref()[1]];
            return Ok(u16::from_be_bytes(b) as usize);
        }

        Err(Error::Incomplete)
    }


    fn get_message_id(src: &Cursor<&[u8]>) -> Result<u8, Error> {
        let start = src.position() as usize;
        let end = src.get_ref().len();

        if end - start >= (LENGTH_FIELD_LEN + 1) as usize {
            return Ok(src.get_ref()[3]);
        }

        Err(Error::Incomplete)
    }

    fn available_data(src: &Cursor<&[u8]>) -> usize {
        let start = src.position() as usize;
        let end = src.get_ref().len();

        return end - start
    }

    pub fn parse(src: &mut Cursor<&[u8]>) -> Result<Frame, Error>
    {
        let length = Self::get_message_length(src)?;
        if length == KEEP_ALIVE_LEN {
            src.set_position(LENGTH_FIELD_LEN as u64);
            return Ok(Frame::KeepAlive)
        }

        let msg_id = Self::get_message_id(src)?;

        if msg_id == b'i' && Self::get_handshake_length(src)? == HANDSHAKE_PSTR_LEN && Self::available_data(src) - 1 >= HANDSHAKE_PSTR_LEN {
            for idx in 0..HANDSHAKE.len() {
                if src.get_ref()[idx + 1] != HANDSHAKE[idx] {
                    return Err(Error::S("nope".into()))
                }
            }
            return Ok(Frame::Handshake)
        }

        let available_data = Self::available_data(src) - LENGTH_FIELD_LEN;
        match msg_id.try_into() {
            Ok(MessageId::Choke) if length == CHOKE_LEN => {
                src.set_position((LENGTH_FIELD_LEN + length) as u64);
                Ok(Frame::Choke)
            },
            Ok(MessageId::Unchoke) if length == UNCHOKE_LEN => {
                src.set_position((LENGTH_FIELD_LEN + length) as u64);
                Ok(Frame::Unchoke)
            },
            Ok(MessageId::Interested) if length == INTERESTED_LEN => {
                src.set_position((LENGTH_FIELD_LEN + length) as u64);
                Ok(Frame::Interested)
            },
            Ok(MessageId::NotInterested) if length == NOT_INTERESTED_LEN => {
                src.set_position((LENGTH_FIELD_LEN + length) as u64);
                Ok(Frame::NotInterested)
            },
            Ok(MessageId::Have) if length == HAVE_LEN => {
                src.set_position((LENGTH_FIELD_LEN + length) as u64);
                Ok(Frame::Have)
            },
            Ok(MessageId::Bitfield) if available_data >= length => {
                src.set_position((LENGTH_FIELD_LEN + length) as u64);
                Ok(Frame::Bitfield)
            },
            Ok(MessageId::Request) if length == REQUEST_LEN => {
                src.set_position((LENGTH_FIELD_LEN + length) as u64);
                Ok(Frame::Request)
            },
            Ok(MessageId::Piece) if available_data >= length => {
                src.set_position((LENGTH_FIELD_LEN + length) as u64);
                Ok(Frame::Piece)
            },
            Ok(MessageId::Cancel) if length == CANCEL_LEN => {
                src.set_position((LENGTH_FIELD_LEN + length) as u64);
                Ok(Frame::Cancel)
            },
            Ok(MessageId::Port) if length == PORT_LEN => {
                src.set_position((LENGTH_FIELD_LEN + length) as u64);
                Ok(Frame::Port)
            },
            _ => Err(Error::S("fuck".into()))
        }
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