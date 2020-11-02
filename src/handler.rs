use crate::frame::{Bitfield, Interested};
use crate::{Connection, Error, Frame, Handshake, Request};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub enum Command {
    RecvBitfield(RecvBitfield),
    RecvUnchoke(RecvUnchoke),
    SendBitfield {
        bitfield: Bitfield,
        interested: bool,
    },
    SendRequest {
        index: usize,
        piece_size: usize,
    },
}

#[derive(Debug)]
pub struct RecvBitfield {
    pub(crate) key: String,
    pub(crate) bitfield: Bitfield,
    pub(crate) channel: oneshot::Sender<Command>,
}

#[derive(Debug)]
pub struct RecvUnchoke {
    pub(crate) key: String,
    pub(crate) channel: oneshot::Sender<Command>,
}

pub struct Handler {
    info_hash: [u8; 20],
    own_id: [u8; 20],
    connection: Connection,
    cmd_tx: mpsc::Sender<Command>,
}

impl Handler {
    pub async fn run(
        addr: String,
        own_id: [u8; 20],
        info_hash: [u8; 20],
        cmd_tx: mpsc::Sender<Command>,
    ) {
        println!("Try connect to {}", &addr);
        let stream = TcpStream::connect(&addr).await.unwrap();
        let connection = Connection::new(addr, stream);
        println!("connect");

        let mut handler = Handler {
            own_id,
            info_hash,
            connection,
            cmd_tx,
        };

        // Process the connection. If an error is encountered, log it.
        if let Err(err) = handler.msg_loop().await {
            // error!(cause = ?err, "connection error");
            panic!("jkl");
        }
    }

    async fn msg_loop(&mut self) -> Result<(), Error> {
        self.connection
            .write_msg(&Handshake::new(&self.info_hash, &self.own_id))
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

                    let cmd = RecvBitfield {
                        key: self.connection.addr.clone(),
                        bitfield: b,
                        channel: resp_tx,
                    };
                    if let Err(e) = self.cmd_tx.send(Command::RecvBitfield(cmd)).await {
                        println!("Coś nie tak {:?}", e);
                    }

                    if let Command::SendBitfield {
                        bitfield,
                        interested,
                    } = resp_rx.await.unwrap()
                    {
                        println!("Odsyłam Bitfield {:?}", bitfield);
                        if let Err(e) = self.connection.write_msg(&bitfield).await {
                            println!("After Bitfield {:?}", e);
                        }

                        if interested {
                            println!("Wysyłam Interested");
                            if let Err(e) = self.connection.write_msg(&Interested {}).await {
                                println!("After Interested {:?}", e);
                            }
                        }
                    }
                }
                Some(Frame::Unchoke(_)) => {
                    self.handle_unchoke().await;
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

    async fn handle_unchoke(&mut self) {
        let (resp_tx, resp_rx) = oneshot::channel();

        let cmd = RecvUnchoke {
            key: self.connection.addr.clone(),
            channel: resp_tx,
        };

        self.cmd_tx.send(Command::RecvUnchoke(cmd)).await.unwrap();

        if let Command::SendRequest { index, piece_size } = resp_rx.await.unwrap() {
            let msg = Request::new(index, 0, 0x4000 as usize);
            println!("Wysyłam request");
            self.connection.write_msg(&msg).await.unwrap();
        }
    }
}
