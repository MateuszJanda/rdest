use rdest::{Connection, Error, Frame, Handshake, Metainfo, Request, TrackerClient};
use std::net::Ipv4Addr;
use tokio;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
struct Recv {
    key: String,
    frame: Frame,
    channel: oneshot::Sender<Frame>,
}

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
    let r = TrackerClient::connect(&t).await.unwrap(); // TODO

    // for v in r.peers {
    //     println!("{:?}", v);
    // }

    // println!("{:?}", ResponseParser::from_file("response.data".to_string()));

    let (mut tx, mut rx) = mpsc::channel(32);

    let pieces_len = t.pieces().len();
    let piece_length = t.piece_length();
    let manager = tokio::spawn(async move {
        let pieces = vec![false; pieces_len];
        while let Some(Recv {
            key,
            frame,
            channel,
        }) = rx.recv().await
        {
            match frame {
                Frame::Bitfield(bitfield) => {
                    let available = bitfield.available_pieces();
                    for i in 0..pieces.len() {
                        if pieces[i] == false && available[i] == true {
                            channel.send(Frame::Request(Request::new(i, 0, piece_length as usize)));
                            break;
                        }
                    }
                }
                _ => (),
            }
        }
    });

    let addr = r.peers()[2].clone();
    let info_hash = t.info_hash();
    let peer_id = b"ABCDEFGHIJKLMNOPQRST";

    let tx2 = mpsc::Sender::clone(&tx);
    // let mut tx2 = tx.clone();

    // let job = tokio::spawn(async move {
    //
    //     // let addr = "127.0.0.1:8888";
    //     println!("Try connect to {}", &addr);
    //     let socket = TcpStream::connect(&addr).await.unwrap();
    //     let connection = Connection::new(addr, socket);
    //     println!("connect");
    //
    //     let mut handler2 = Handler { connection, tx: tx2 };
    //
    //     // Process the connection. If an error is encountered, log it.
    //     if let Err(err) = handler2.run2(&info_hash, peer_id).await {
    //         // error!(cause = ?err, "connection error");
    //         panic!("jkl");
    //     }
    // });

    let job = tokio::spawn(fff(addr, info_hash, *peer_id, tx2));

    job.await.unwrap();
    manager.await.unwrap();

    // {
    //     let addr = "127.0.0.1:8888";
    //     let socket = TcpStream::connect(addr).await.unwrap();
    //     let connection = Connection::new(socket);
    //
    //     let mut handler3 = Handler { connection };
    //
    //     tokio::spawn(async move {
    //         if let Err(err) = handler3.run3().await {
    //             panic!("jkl");
    //         }
    //     });
    // }

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

    println!("-==[ koniec ]==-");
}

async fn fff(addr: String, info_hash: [u8; 20], peer_id: [u8; 20], tx2: mpsc::Sender<Recv>) {
    // let addr = "127.0.0.1:8888";
    println!("Try connect to {}", &addr);
    let socket = TcpStream::connect(&addr).await.unwrap();
    let connection = Connection::new(addr, socket);
    println!("connect");

    let mut handler2 = Handler {
        connection,
        tx: tx2,
    };

    // Process the connection. If an error is encountered, log it.
    if let Err(err) = handler2.run2(&info_hash, &peer_id).await {
        // error!(cause = ?err, "connection error");
        panic!("jkl");
    }
}

struct Handler {
    connection: Connection,
    tx: mpsc::Sender<Recv>,
}

impl Handler {
    // async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    //     loop {
    //         let res = self.connection.read_frame().await?;
    //         break;
    //     }
    //
    //     Ok(())
    // }

    async fn run2(&mut self, info_hash: &[u8; 20], peer_id: &[u8; 20]) -> Result<(), Error> {
        self.connection
            .write_frame(&Handshake::new(info_hash, peer_id))
            .await
            .unwrap();

        loop {
            match self.connection.read_frame().await? {
                Some(Frame::Handshake(_)) => {
                    println!("Handshake");
                }
                Some(Frame::Bitfield(b)) => {
                    println!("Bitfield");
                    let (resp_tx, resp_rx) = oneshot::channel();
                    self.tx
                        .send(Recv {
                            key: self.connection.addr.clone(),
                            frame: Frame::Bitfield(b),
                            channel: resp_tx,
                        })
                        .await
                        .unwrap();

                    if let Frame::Request(res) = resp_rx.await.unwrap() {
                        println!("Odsyłam Requst {:?}", res);
                        self.connection.write_frame(&res).await.unwrap();
                    }
                }
                Some(Frame::Piece(_)) => {
                    println!("Piece");
                }
                Some(f) => {
                    println!("Frame: {:?}", f);
                }
                _ => {}
            }
        }
        Ok(())
    }

    // async fn run3(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    //     println!("run3");
    //     self.connection.stream.write_all(b"asdf").await?;
    //     let n = self
    //         .connection
    //         .stream
    //         .read_buf(&mut self.connection.buffer)
    //         .await?;
    //     println!("the n {}", n);
    //
    //     Ok(())
    // }
}
