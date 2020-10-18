use rdest::{Error, Frame, Handshake, Metainfo, TrackerClient, TrackerResp};
use std::io::Cursor;
use std::net::Ipv4Addr;
use tokio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

// fn main() {
//     let t = Torrent::from_file(String::from("ubuntu-20.04.1-desktop-amd64.iso.torrent"));
//     println!("{:?}", t);
//     match TrackerClient::connect1(&t.unwrap()) {
//         Ok(_) => println!("Http Ok"),
//         Err(e) => println!("Http Problem {:?}", e),
//     }
//
//     // println!("{:?}", ResponseParser::from_file("response.data".to_string()));
// }

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    // let mut listener = TcpListener::bind("127.0.0.1:6881").await.unwrap();
    let mut listener = TcpListener::bind((Ipv4Addr::new(0, 0, 0, 0), 6881))
        .await
        .unwrap();

    let t = Metainfo::from_file(String::from("ubuntu-20.04.1-desktop-amd64.iso.torrent")).unwrap();
    // println!("{:?}", t);

    // let r = TrackerResp::from_file("response.data".to_string()).unwrap();
    let r = TrackerClient::connect1(&t).await.unwrap(); // TODO

    // for v in r.peers {
    //     println!("{:?}", v);
    // }

    // println!("{:?}", ResponseParser::from_file("response.data".to_string()));

    {
        let addr = &r.peers()[0];
        let socket = TcpStream::connect(addr).await.unwrap();
        let connection = Connection::new(socket);
        println!("connect");

        let mut handler2 = Handler { connection };

        let info_hash = t.info_hash;
        let peer_id = b"ABCDEFGHIJKLMNOPQRST";
        tokio::spawn(async move {
            // Process the connection. If an error is encountered, log it.
            if let Err(err) = handler2.run2(&info_hash, peer_id).await {
                // error!(cause = ?err, "connection error");
                panic!("jkl");
            }
        });
    }

    // loop {
    //     println!("Listening");
    //     // The second item contains the IP and port of the new connection.
    //     let (socket, _) = listener.accept().await.unwrap();
    //     println!("accept");
    //
    //     let connection = Connection::new(socket);
    //
    //     let mut handler = Handler { connection };
    //
    //     tokio::spawn(async move {
    //         // Process the connection. If an error is encountered, log it.
    //         if let Err(err) = handler.run().await {
    //             // error!(cause = ?err, "connection error");
    //             panic!("asdf");
    //         }
    //     });
    // }
}

struct Handler {
    connection: Connection,
}

impl Handler {
    async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let res = self.connection.read_frame().await?;
        }

        Ok(())
    }

    async fn run2(
        &mut self,
        info_hash: &[u8; 20],
        peer_id: &[u8; 20],
    ) -> Result<(), Box<dyn std::error::Error>> {

        self.connection.init_frame(info_hash, peer_id).await?;
        let res = self.connection.read_frame().await?;

        println!("{:?}", res.unwrap());


        Ok(())
    }
}

struct Connection {
    stream: TcpStream,
    buffer: Vec<u8>,
    cursor: usize,
}

const BUFFER_SIZE: usize = 65536 + 2;

impl Connection {
    pub fn new(stream: TcpStream) -> Connection {
        Connection {
            stream,
            buffer: vec![0; BUFFER_SIZE],
            cursor: 0,
        }
    }

    pub async fn init_frame(
        &mut self,
        info_hash: &[u8; 20],
        peer_id: &[u8; 20],
    ) -> Result<(), Box<dyn std::error::Error>> {
        // self.stream.read(&mut self.buffer[self.cursor..]).await?;
        self.stream
            .write_all(Handshake::new(info_hash, peer_id).to_vec().as_slice());

        println!("Handshake send");

        Ok(())
    }

    pub async fn read_frame(&mut self) -> Result<Option<Frame>, Box<dyn std::error::Error>> {
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

            let n = self.stream.read(&mut self.buffer[self.cursor..]).await?;

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
                    return Err(Error::Peer("connection reset by peer".into()).into());
                }
            } else {
                // Update our cursor
                self.cursor += n;
            }
        }
    }

    fn parse_frame(&mut self) -> Result<Option<Frame>, Error> {
        let mut crs = Cursor::new(&self.buffer[..]);

        // Check whether a full frame is available
        match Frame::check(&mut crs) {
            Ok(_) => {
                // Parse the frame
                let frame = Frame::parse(&mut crs)?;

                // Discard the frame from the buffer
                let len = crs.position() as usize;
                self.buffer.drain(..len);
                self.buffer.resize(BUFFER_SIZE, 0);

                // Return the frame to the caller.
                Ok(Some(frame))
            }
            // Not enough data has been buffered
            Err(Error::Incomplete) => Ok(None),
            // An error was encountered
            Err(e) => Err(e.into()),
        }
    }
}
