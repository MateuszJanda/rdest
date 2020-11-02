use crate::frame::{Bitfield, Interested, Piece};
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
    Done {
        key: String,
        channel: oneshot::Sender<Command>,
    },
    Kill,
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
    index: Option<usize>,
    piece: Vec<u8>,
    position: usize,
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
            index: None,
            piece: vec![],
            position: 0,
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
                Some(Frame::Piece(p)) => {
                    self.handle_piece(&p).await;
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
            self.piece = vec![0; piece_size];
            self.index = Some(index);

            let length = self.chunk_length();
            let msg = Request::new(index, self.position, length);
            println!("Wysyłam request");
            self.connection.write_msg(&msg).await.unwrap();
        }
    }

    async fn handle_piece(&mut self, piece: &Piece) {
        println!("Piece");

        self.piece[self.position..self.position + piece.block.len()].copy_from_slice(&piece.block);
        self.position += piece.block.len();

        if self.position == piece.block.len() {
            let (resp_tx, resp_rx) = oneshot::channel();

            let cmd = Command::Done {
                key: self.connection.addr.clone(),
                channel: resp_tx,
            };

            if let Err(e) = self.cmd_tx.send(cmd).await {
                println!("Coś nie tak {:?}", e);
            }

            match resp_rx.await.unwrap() {
                Command::SendRequest { index, piece_size } => {
                    self.index = Some(index);
                    self.position = 0;
                    self.piece = vec![0; piece_size];

                    let length = self.chunk_length();
                    let msg = Request::new(index, self.position, length);
                    println!("Wysyłam nowy request");
                    self.connection.write_msg(&msg).await.unwrap();
                }
                Command::Kill => {}
                _ => (),
            }
        } else {
            let length = self.chunk_length();
            let msg = Request::new(self.index.unwrap(), self.position, length);
            println!("Wysyłam kolejny request");
            self.connection.write_msg(&msg).await.unwrap();
        }
    }

    fn chunk_length(&self) -> usize {
        if self.position + 0x4000 > self.piece.len() {
            return self.piece.len() % 0x4000;
        }

        return 0x4000;
    }
}
