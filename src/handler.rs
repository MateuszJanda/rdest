use tokio::sync::{mpsc, oneshot};
use tokio::net::TcpStream;
use crate::{Connection, Handshake, Frame, Error};


#[derive(Debug)]
pub struct Recv {
    pub key: String,
    pub frame: Frame,
    pub channel: oneshot::Sender<Frame>,
}


pub async fn fff(addr: String, info_hash: [u8; 20], peer_id: [u8; 20], tx2: mpsc::Sender<Recv>) {
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
    if let Err(err) = handler2.run(&info_hash, &peer_id).await {
        // error!(cause = ?err, "connection error");
        panic!("jkl");
    }
}

struct Handler {
    connection: Connection,
    tx: mpsc::Sender<Recv>,
}

impl Handler {
    async fn run(&mut self, info_hash: &[u8; 20], peer_id: &[u8; 20]) -> Result<(), Error> {
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
}
