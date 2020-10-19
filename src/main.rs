use rdest::{Error, Frame, Handshake, Metainfo, TrackerClient, TrackerResp};
use std::io::Cursor;
use std::net::Ipv4Addr;
use tokio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use bytes::{Buf, BytesMut};
use std::error;

/*
#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {

    // let r = TrackerResp::from_file("response.data".to_string()).unwrap();
    let t = Metainfo::from_file(String::from("ubuntu-20.04.1-desktop-amd64.iso.torrent")).unwrap();
    let r = TrackerClient::connect1(&t).await.unwrap(); // TODO

    let addr = &r.peers()[3];
    println!("Try connect to {}", addr);
    let mut stream = TcpStream::connect(addr).await?;

    // let mut stream = TcpStream::connect("127.0.0.1:8888").await?;

    let mut buffer =BytesMut::with_capacity(4096);
    println!("buf len {}", buffer.len());
    // let mut buffer = [0; 10];

    let mut connection = Connection {
        stream,
        buffer,
    };

    let peer_id = b"ABCDEFGHIJKLMNOPQRST";
    connection.init_frame(&t.info_hash, peer_id).await?;
    connection.read_frame().await?;

    // let n = stream.read_buf(&mut buffer).await?;
    // println!("{}", n);

    Ok(())
}


 */
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
    // let r = TrackerClient::connect1(&t).await.unwrap(); // TODO

    // for v in r.peers {
    //     println!("{:?}", v);
    // }

    // println!("{:?}", ResponseParser::from_file("response.data".to_string()));

    // {
    //     let addr = &r.peers()[1];
    //     println!("Try connect to {}", addr);
    //     let socket = TcpStream::connect(addr).await.unwrap();
    //     let connection = Connection::new(socket);
    //     println!("connect");
    //
    //     let mut handler2 = Handler { connection };
    //
    //     let info_hash = t.info_hash;
    //     let peer_id = b"ABCDEFGHIJKLMNOPQRST";
    //     tokio::spawn(async move {
    //         // Process the connection. If an error is encountered, log it.
    //         if let Err(err) = handler2.run2(&info_hash, peer_id).await {
    //             // error!(cause = ?err, "connection error");
    //             panic!("jkl");
    //         }
    //     });
    // }

    {
        let addr = "127.0.0.1:8888";
        let socket = TcpStream::connect(addr).await.unwrap();
        let connection = Connection::new(socket);

        let mut handler3 = Handler { connection };

        tokio::spawn(async move {
            if let Err(err) = handler3.run3().await {
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

    println!("koniec");
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
        // let res = match self.connection.read_frame().await {
        //     Err(e) => {
        //         println!("coś nie tak {}", e);
        //         Err(e)?
        //     }
        //     Ok(r) => {
        //         println!("jest ok");
        //         r
        //     },
        // };

        println!("{:?}", res.unwrap());

        Ok(())
    }

    async fn run3(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("run3");
        self.connection.stream.write_all(b"asdf").await?;
        let n = self.connection.stream.read_buf(&mut self.connection.buffer).await?;
        println!("the n {}", n);

        Ok(())
    }
}

struct Connection {
    stream: TcpStream,
    buffer: BytesMut,
}

const BUFFER_SIZE: usize = 65536 + 2;

impl Connection {
    pub fn new(stream: TcpStream) -> Connection {
        Connection {
            stream,
            buffer: BytesMut::with_capacity(BUFFER_SIZE),
        }
    }

    pub async fn init_frame(
        &mut self,
        info_hash: &[u8; 20],
        peer_id: &[u8; 20],
    ) -> Result<(), Box<dyn std::error::Error>> {
        // self.stream.read(&mut self.buffer[self.cursor..]).await?;

        self.stream
            .write_all(Handshake::new(info_hash, peer_id).to_vec().as_slice()).await?;
        // self.stream.write_all(b"asdf").await?;

        println!("Handshake send");

        Ok(())
    }

    pub async fn read_frame(&mut self) -> Result<Option<Frame>, Box<dyn std::error::Error>> {
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
            let n = self.stream.read_buf(&mut self.buffer).await?;
            // let n = self.stream.read(&mut self.buffer).await?;
            // let n = match self.stream.read_buf(&mut self.buffer) {
            //     Err(e) => {
            //         println!("tutaj");
            //         println!("{:?}", e);
            //         0
            //     },
            //     Ok(n) => {
            //         println!("tutaj dobrze");
            //         n
            //     },
            //     _ => {
            //         println!("coś jescze");
            //         0
            //     }
            // };
            println!("read n {} {}", n, self.buffer.is_empty());
            if n == 0 {
                return if self.buffer.is_empty() {
                    Ok(None)
                } else {
                    Err(Error::Peer("connection reset by peer".into()).into())
                }
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
                // self.buffer.drain(..len);
                // self.buffer.resize(BUFFER_SIZE, 0);

                self.buffer.advance(len);

                // Return the frame to the caller.
                Ok(Some(frame))
            }
            // Not enough data has been buffered
            Err(Error::Incomplete) => {
                println!("Incomplete");
                Ok(None)
            },
            // An error was encountered
            Err(e) => Err(e.into()),
        }
    }
}
