use crate::frame::{Bitfield, Interested, Piece};
use crate::{Connection, Error, Frame, Handshake, Request};
use std::fs;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{interval_at, Duration, Instant};

#[derive(Debug)]
pub enum Command {
    RecvBitfield(RecvBitfield),
    RecvUnchoke(RecvUnchoke),
    RecvHave(RecvHave),

    SendBitfield {
        bitfield: Bitfield,
        interested: bool,
    },
    SendRequest {
        index: usize,
        piece_size: usize,
        piece_hash: [u8; 20],
    },
    Done(Done),
    VerifyFail(VerifyFail),
    End,
    KillReq {
        key: String,
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

#[derive(Debug)]
pub struct RecvHave {
    pub(crate) key: String,
    pub(crate) index: usize,
}

#[derive(Debug)]
pub struct Done {
    pub key: String,
    pub channel: oneshot::Sender<Command>,
}

#[derive(Debug)]
pub struct VerifyFail {
    pub key: String,
    pub channel: oneshot::Sender<Command>,
}

pub struct Handler {
    info_hash: [u8; 20],
    own_id: [u8; 20],
    index: Option<usize>,
    piece: Vec<u8>,
    position: usize,
    piece_hash: [u8; 20],
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
            piece_hash: [0; 20],
            connection,
            cmd_tx,
        };

        // Process the connection. If an error is encountered, log it.
        if let Err(e) = handler.msg_loop().await {
            // error!(cause = ?err, "connection error");
            panic!("jkl {:?}", e);
        }
    }

    async fn msg_loop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.connection
            .write_msg(&Handshake::new(&self.info_hash, &self.own_id))
            .await
            .unwrap();

        let start = Instant::now() + Duration::from_millis(0);
        let mut interval = interval_at(start, Duration::from_millis(2000));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // TODO: keep alive
                },
                frame = self.connection.read_frame() => {
                    if false == self.handle_frame(frame?).await? {
                        break;
                    }
                }
            }
        }

        {
            let cmd = Command::KillReq {
                key: self.connection.addr.clone(),
            };

            self.cmd_tx.send(cmd).await?;
        }

        Ok(())
    }

    async fn handle_frame(
        &mut self,
        frame: Option<Frame>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match frame {
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
            Some(Frame::Have(h)) => {
                let cmd = RecvHave {
                    key: self.connection.addr.clone(),
                    index: h.index(),
                };

                self.cmd_tx.send(Command::RecvHave(cmd)).await?;
            }
            Some(Frame::Unchoke(_)) => {
                self.handle_unchoke().await;
            }
            Some(Frame::Piece(p)) => {
                if false == self.handle_piece(&p).await? {
                    return Ok(false);
                }
            }
            Some(f) => {
                println!("Frame: {:?}", f);
            }
            _ => {}
        }

        return Ok(true);
    }

    async fn handle_unchoke(&mut self) {
        let (resp_tx, resp_rx) = oneshot::channel();

        let cmd = RecvUnchoke {
            key: self.connection.addr.clone(),
            channel: resp_tx,
        };

        self.cmd_tx.send(Command::RecvUnchoke(cmd)).await.unwrap();

        if let Command::SendRequest {
            index,
            piece_size,
            piece_hash,
        } = resp_rx.await.unwrap()
        {
            self.piece = vec![0; piece_size];
            self.piece_hash = piece_hash;
            self.index = Some(index);

            let length = self.chunk_length();
            let msg = Request::new(index, self.position, length);
            println!("Wysyłam request");
            self.connection.write_msg(&msg).await.unwrap();
        }
    }

    async fn handle_piece(&mut self, piece: &Piece) -> Result<bool, Box<dyn std::error::Error>> {
        println!("Piece");

        self.piece[self.position..self.position + piece.block.len()].copy_from_slice(&piece.block);
        self.position += piece.block.len();

        if self.position == self.piece.len() {
            if !self.verify() {
                let (resp_tx, resp_rx) = oneshot::channel();
                println!("Verify fail");
                let cmd = Command::VerifyFail(VerifyFail {
                    key: self.connection.addr.clone(),
                    channel: resp_tx,
                });

                self.cmd_tx.send(cmd).await?;

                match resp_rx.await? {
                    Command::End => return Ok(false),
                    _ => (),
                }

                return Ok(true);
            }

            self.write_piece();

            let (resp_tx, resp_rx) = oneshot::channel();
            let cmd = Command::Done(Done {
                key: self.connection.addr.clone(),
                channel: resp_tx,
            });

            self.cmd_tx.send(cmd).await?;

            match resp_rx.await? {
                Command::SendRequest {
                    index,
                    piece_size,
                    piece_hash,
                } => {
                    self.index = Some(index);
                    self.position = 0;
                    self.piece = vec![0; piece_size];
                    self.piece_hash = piece_hash;

                    let length = self.chunk_length();
                    let msg = Request::new(index, self.position, length);
                    println!("Wysyłam nowy request");
                    self.connection.write_msg(&msg).await.unwrap();
                }
                Command::End => return Ok(false),
                _ => (),
            }
        } else {
            let length = self.chunk_length();
            let msg = Request::new(self.index.unwrap(), self.position, length);
            println!("Wysyłam kolejny request");
            self.connection.write_msg(&msg).await.unwrap();
        }

        Ok(true)
    }

    fn chunk_length(&self) -> usize {
        if self.position + 0x4000 > self.piece.len() {
            return self.piece.len() % 0x4000;
        }

        return 0x4000;
    }

    fn verify(&self) -> bool {
        let mut m = sha1::Sha1::new();
        m.update(self.piece.as_ref());

        println!("Checksum: {:?} {:?}", m.digest().bytes(), self.piece_hash);

        return m.digest().bytes() == self.piece_hash;
    }

    fn write_piece(&self) {
        let name: String = self
            .piece_hash
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect();

        fs::write(name, &self.piece).unwrap(); // TODO: remove
    }
}
