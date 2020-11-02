use crate::frame::{Bitfield, Interested};
use crate::{Connection, Error, Frame, Handshake, Request};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};

// #[derive(Debug)]
// pub struct Recv {
//     pub key: String,
//     pub frame: Frame,
//     pub channel: oneshot::Sender<Frame>,
// }

#[derive(Debug)]
pub enum Command {
    RecvBitfield {
        key: String,
        bitfield: Bitfield,
        channel: oneshot::Sender<Command>,
    },
    RecvUnchoke {
        key: String,
        channel: oneshot::Sender<Command>,
    },
    SendBitfield {
        bitfield: Bitfield,
        interested: bool,
    },
    SendRequest {
        req: Request,
    },
}

pub struct Handler {
    connection: Connection,
    tx: mpsc::Sender<Command>,
}

impl Handler {
    pub async fn fff(
        addr: String,
        info_hash: [u8; 20],
        peer_id: [u8; 20],
        tx2: mpsc::Sender<Command>,
    ) {
        println!("Try connect to {}", &addr);
        let stream = TcpStream::connect(&addr).await.unwrap();
        let connection = Connection::new(addr, stream);
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

    async fn run(&mut self, info_hash: &[u8; 20], peer_id: &[u8; 20]) -> Result<(), Error> {
        self.connection
            .write_frame(&Handshake::new(info_hash, peer_id))
            .await
            .unwrap();

        loop {
            match self.connection.read_frame().await? {
                Some(Frame::Handshake(_)) => {
                    println!("Handshake");
                    // TODO validate Handshake
                }
                Some(Frame::Bitfield(b)) => {
                    println!("Bitfield");
                    let (resp_tx, resp_rx) = oneshot::channel();

                    if let Err(e) = self.tx
                        .send(Command::RecvBitfield {
                            key: self.connection.addr.clone(),
                            bitfield: b,
                            channel: resp_tx,
                        })
                        .await {
                        println!("Coś nie tak {:?}", e);
                    }

                    if let Command::SendBitfield {
                        bitfield,
                        interested,
                    } = resp_rx.await.unwrap()
                    {
                        println!("Odsyłam Bitfield {:?}", bitfield);
                        if let Err(e) = self.connection.write_frame(&bitfield).await {
                            println!("After Bitfield {:?}", e);
                        }

                        if interested {
                            println!("Wysyłam Interested");
                            if let Err(e) = self.connection.write_frame(&Interested {}).await {
                                println!("After Interested {:?}", e);
                            }
                        }
                    }
                }
                Some(Frame::Unchoke(u)) => {
                    let (resp_tx, resp_rx) = oneshot::channel();
                    self.tx
                        .send(Command::RecvUnchoke {
                            key: self.connection.addr.clone(),
                            channel: resp_tx,
                        })
                        .await
                        .unwrap();

                    if let Command::SendRequest { req } = resp_rx.await.unwrap() {
                        println!("Wysyłam request");
                        self.connection.write_frame(&req).await.unwrap();
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
