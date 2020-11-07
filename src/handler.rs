use crate::frame::{Bitfield, Interested, Piece};
use crate::{Connection, Frame, Handshake, Have, KeepAlive, Request};
use std::fs;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::time::{interval_at, Duration, Instant};

#[derive(Debug, Clone)]
pub enum BroadcastCommand {
    SendHave { key: String, index: usize },
}
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
        index: Option<usize>,
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
    pieces_count: usize,
    piece: Vec<u8>,
    position: usize,
    piece_hash: [u8; 20],

    keep_alive: bool,

    connection: Connection,
    cmd_tx: mpsc::Sender<Command>,
    b_rx: broadcast::Receiver<BroadcastCommand>,
}

impl Handler {
    pub async fn run(
        addr: String,
        own_id: [u8; 20],
        info_hash: [u8; 20],
        pieces_count: usize,
        cmd_tx: mpsc::Sender<Command>,
        b_rx: broadcast::Receiver<BroadcastCommand>,
    ) {
        println!("Try connect to {}", &addr);
        let stream = TcpStream::connect(&addr).await.unwrap();
        let connection = Connection::new(addr, stream);
        println!("connect");

        let mut handler = Handler {
            own_id,
            info_hash,
            index: None,
            pieces_count,
            piece: vec![],
            position: 0,
            piece_hash: [0; 20],
            keep_alive: false,
            connection,
            cmd_tx,
            b_rx,
        };

        // Process the connection. If an error is encountered, log it.
        if let Err(e) = handler.msg_loop().await {
            // error!(cause = ?err, "connection error");
            panic!("jkl {:?}", e);
        }

        handler.kill_req().await;
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
                    if !self.keep_alive {
                        println!("Close connection, no messages from peer");
                        break;
                    }
                    self.send_keep_alive().await?;
                    self.keep_alive = false;
                }
                c = self.b_rx.recv() => {
                    if false == self.send_have(c?).await? {
                        break;
                    }
                }
                frame = self.connection.read_frame() => {
                    if false == self.handle_frame(frame?).await? {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    async fn kill_req(&mut self) {
        let cmd = Command::KillReq {
            key: self.connection.addr.clone(),
            index: self.index,
        };

        // Should panic if can't inform manager
        self.cmd_tx.send(cmd).await.unwrap();
    }

    async fn send_keep_alive(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.connection.write_msg(&KeepAlive::new()).await?;

        Ok(())
    }

    async fn send_have(
        &mut self,
        cmd: BroadcastCommand,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match cmd {
            BroadcastCommand::SendHave { key, index } => {
                if key != self.connection.addr {
                    self.connection.write_msg(&Have::new(index)).await?;
                }

                return Ok(true);
            }
            _ => Ok(false),
        }
    }

    async fn handle_frame(
        &mut self,
        frame: Option<Frame>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match frame {
            Some(Frame::Handshake(h)) => {
                println!("Handshake");
                self.keep_alive = true;
                h.validate(&self.info_hash)?;
            }
            Some(Frame::KeepAlive(_)) => {
                self.keep_alive = true;
            }
            Some(Frame::Bitfield(b)) => {
                b.validate(&self.pieces_count)?;

                self.keep_alive = true;
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
                h.validate(self.pieces_count)?;

                self.keep_alive = true;
                let cmd = RecvHave {
                    key: self.connection.addr.clone(),
                    index: h.index(),
                };

                self.cmd_tx.send(Command::RecvHave(cmd)).await?;
            }
            Some(Frame::Unchoke(_)) => {
                self.keep_alive = true;
                self.handle_unchoke().await;
            }
            Some(Frame::Piece(p)) => {
                p.validate(self.pieces_count, self.piece.len())?;

                self.keep_alive = true;
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
