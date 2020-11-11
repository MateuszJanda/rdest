use crate::connection::Connection;
use crate::frame::{Bitfield, Frame, Handshake, Have, Interested, KeepAlive, Piece, Request};
use crate::utils;
use std::fs;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::time::{interval_at, Duration, Instant};

#[derive(Debug, Clone)]
pub enum BroadCmd {
    SendHave { key: String, index: usize },
}
#[derive(Debug)]
pub enum JobCmd {
    RecvBitfield {
        addr: String,
        bitfield: Bitfield,
        resp_ch: oneshot::Sender<BitfieldCmd>,
    },
    RecvChoke {
        addr: String,
        resp_ch: oneshot::Sender<JobCmd>,
    },
    RecvUnchoke {
        addr: String,
        resp_ch: oneshot::Sender<JobCmd>,
    },
    // RecvInterested,
    // RecvNotInterested,
    RecvHave {
        addr: String,
        index: usize,
    },
    PieceDone {
        addr: String,
        resp_ch: oneshot::Sender<JobCmd>,
    },
    VerifyFail {
        addr: String,
        resp_ch: oneshot::Sender<JobCmd>,
    },
    KillReq {
        addr: String,
        index: Option<usize>,
    },

    SendRequest {
        index: usize,
        piece_size: usize,
        piece_hash: [u8; 20],
    },

    SendNotInterested,
    End,
}

#[derive(Debug)]
pub enum BitfieldCmd {
    SendBitfield {
        bitfield: Bitfield,
        interested: bool,
    },
    // PrepareKill,
}

pub struct Handler {
    info_hash: [u8; 20],
    own_id: [u8; 20],
    index: Option<usize>,
    pieces_count: usize,
    buff_piece: Vec<u8>,
    buff_pos: usize,
    piece_hash: [u8; 20],

    keep_alive: bool,

    connection: Connection,
    job_ch: mpsc::Sender<JobCmd>,
    broad_ch: broadcast::Receiver<BroadCmd>,
}

impl Handler {
    pub async fn run(
        addr: String,
        own_id: [u8; 20],
        info_hash: [u8; 20],
        pieces_count: usize,
        job_ch: mpsc::Sender<JobCmd>,
        broad_ch: broadcast::Receiver<BroadCmd>,
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
            buff_piece: vec![],
            buff_pos: 0,
            piece_hash: [0; 20],
            keep_alive: false,
            connection,
            job_ch,
            broad_ch,
        };

        // Process the connection. If an error is encountered, log it.
        if let Err(e) = handler.event_loop().await {
            // error!(cause = ?err, "connection error");
            panic!("jkl {:?}", e);
        }

        handler.kill_req().await;
    }

    async fn event_loop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.connection
            .write_msg(&Handshake::new(&self.info_hash, &self.own_id))
            .await?;

        let start = Instant::now() + Duration::from_secs(2 * 60);
        let mut interval = interval_at(start, Duration::from_secs(2 * 60));

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
                c = self.broad_ch.recv() => {
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
        let cmd = JobCmd::KillReq {
            addr: self.connection.addr.clone(),
            index: self.index,
        };

        // Should panic if can't inform manager
        self.job_ch.send(cmd).await.unwrap();
    }

    async fn send_keep_alive(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.connection.write_msg(&KeepAlive::new()).await?;

        Ok(())
    }

    async fn send_have(&mut self, cmd: BroadCmd) -> Result<bool, Box<dyn std::error::Error>> {
        match cmd {
            BroadCmd::SendHave { key, index } => {
                if key != self.connection.addr {
                    self.connection.write_msg(&Have::new(index)).await?;
                }

                return Ok(true);
            }
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

                let cmd = JobCmd::RecvBitfield {
                    addr: self.connection.addr.clone(),
                    bitfield: b,
                    resp_ch: resp_tx,
                };
                if let Err(e) = self.job_ch.send(cmd).await {
                    println!("Coś nie tak {:?}", e);
                }

                if let BitfieldCmd::SendBitfield {
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
                        if let Err(e) = self.connection.write_msg(&Interested::new()).await {
                            println!("After Interested {:?}", e);
                        }
                    }
                }
            }
            Some(Frame::Have(h)) => {
                h.validate(self.pieces_count)?;

                self.keep_alive = true;
                let cmd = JobCmd::RecvHave {
                    addr: self.connection.addr.clone(),
                    index: h.index(),
                };

                self.job_ch.send(cmd).await?;
            }
            Some(Frame::Unchoke(_)) => {
                self.keep_alive = true;
                self.handle_unchoke().await;
            }
            Some(Frame::Piece(p)) => {
                p.validate(self.index.unwrap(), self.buff_pos, self.chunk_length())?;

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

        let cmd = JobCmd::RecvUnchoke {
            addr: self.connection.addr.clone(),
            resp_ch: resp_tx,
        };

        self.job_ch.send(cmd).await.unwrap();

        if let JobCmd::SendRequest {
            index,
            piece_size,
            piece_hash,
        } = resp_rx.await.unwrap()
        {
            self.buff_piece = vec![0; piece_size];
            self.piece_hash = piece_hash;
            self.index = Some(index);

            let length = self.chunk_length();
            let msg = Request::new(index, self.buff_pos, length);
            println!("Wysyłam request");
            self.connection.write_msg(&msg).await.unwrap();
        }
    }

    async fn handle_piece(
        &mut self,
        buff_piece: &Piece,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        println!("Piece");

        self.buff_piece[self.buff_pos..self.buff_pos + buff_piece.block.len()]
            .copy_from_slice(&buff_piece.block);
        self.buff_pos += buff_piece.block.len();

        if self.buff_pos == self.buff_piece.len() {
            if !self.verify() {
                let (resp_tx, resp_rx) = oneshot::channel();
                println!("Verify fail");
                let cmd = JobCmd::VerifyFail {
                    addr: self.connection.addr.clone(),
                    resp_ch: resp_tx,
                };

                self.job_ch.send(cmd).await?;

                match resp_rx.await? {
                    JobCmd::End => return Ok(false),
                    _ => (),
                }

                return Ok(true);
            }

            self.write_piece();

            let (resp_tx, resp_rx) = oneshot::channel();
            let cmd = JobCmd::PieceDone {
                addr: self.connection.addr.clone(),
                resp_ch: resp_tx,
            };

            self.job_ch.send(cmd).await?;

            match resp_rx.await? {
                JobCmd::SendRequest {
                    index,
                    piece_size,
                    piece_hash,
                } => {
                    self.index = Some(index);
                    self.buff_pos = 0;
                    self.buff_piece = vec![0; piece_size];
                    self.piece_hash = piece_hash;

                    let length = self.chunk_length();
                    let msg = Request::new(index, self.buff_pos, length);
                    println!("Wysyłam nowy request");
                    self.connection.write_msg(&msg).await.unwrap();
                }
                JobCmd::End => return Ok(false),
                _ => (),
            }
        } else {
            let length = self.chunk_length();
            let msg = Request::new(self.index.unwrap(), self.buff_pos, length);
            println!("Wysyłam kolejny request");
            self.connection.write_msg(&msg).await.unwrap();
        }

        Ok(true)
    }

    fn chunk_length(&self) -> usize {
        if self.buff_pos + 0x4000 > self.buff_piece.len() {
            return self.buff_piece.len() % 0x4000;
        }

        return 0x4000;
    }

    fn verify(&self) -> bool {
        let mut m = sha1::Sha1::new();
        m.update(self.buff_piece.as_ref());

        println!("Checksum: {:?} {:?}", m.digest().bytes(), self.piece_hash);

        return m.digest().bytes() == self.piece_hash;
    }

    fn write_piece(&self) {
        let name = utils::hash_to_string(&self.piece_hash) + ".buff_piece";
        fs::write(name, &self.buff_piece).unwrap(); // TODO: remove
    }
}
