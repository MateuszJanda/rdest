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
    connection: Connection,
    own_id: [u8; 20],
    info_hash: [u8; 20],
    pieces_count: usize,
    piece_index: Option<usize>,
    piece_hash: [u8; 20],
    buff_piece: Vec<u8>,
    buff_pos: usize,

    keep_alive: bool,

    job_ch: mpsc::Sender<JobCmd>,
    broad_ch: broadcast::Receiver<BroadCmd>,
}

impl Handler {
    pub async fn run(
        addr: String,
        own_id: [u8; 20],
        info_hash: [u8; 20],
        pieces_count: usize,
        mut job_ch: mpsc::Sender<JobCmd>,
        broad_ch: broadcast::Receiver<BroadCmd>,
    ) {
        println!("Try connect to {}", &addr);
        if let Ok(stream) = TcpStream::connect(&addr).await {
            println!("connected");

            let mut handler = Handler {
                connection: Connection::new(addr, stream),
                own_id,
                info_hash,
                pieces_count,
                piece_index: None,
                piece_hash: [0; 20],
                buff_piece: vec![],
                buff_pos: 0,
                keep_alive: false,
                job_ch,
                broad_ch,
            };

            // Process the connection. If an error is encountered, log it.
            if let Err(e) = handler.event_loop().await {
                println!("jkl {:?}", e);
            }

            Self::kill_req(
                &handler.connection.addr,
                &handler.piece_index,
                &mut handler.job_ch,
            )
            .await;
        } else {
            Self::kill_req(&addr, &None, &mut job_ch).await;
        }
    }

    async fn kill_req(addr: &String, index: &Option<usize>, job_ch: &mut mpsc::Sender<JobCmd>) {
        let cmd = JobCmd::KillReq {
            addr: addr.clone(),
            index: *index,
        };

        job_ch
            .send(cmd)
            .await
            .expect("Can't inform manager about KillReq");
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
                cmd = self.broad_ch.recv() => {
                    if self.send_have(cmd?).await? == false {
                        break;
                    }
                }
                frame = self.connection.read_frame() => {
                    if self.handle_frame(frame?).await? == false {
                        break;
                    }
                }
            }
        }

        Ok(())
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
        opt_frame: Option<Frame>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match opt_frame {
            Some(frame) => match frame {
                Frame::Handshake(handshake) => self.handle_handshake(&handshake)?,
                Frame::KeepAlive(_) => self.handle_keep_alive(),
                Frame::Choke(_) => (),
                Frame::Unchoke(_) => self.handle_unchoke().await,
                Frame::Interested(_) => (),
                Frame::NotInterested(_) => (),
                Frame::Have(have) => self.handle_have(&have).await?,
                Frame::Bitfield(bitfield) => self.handle_bitfield(bitfield).await?,
                Frame::Request(_) => (),
                Frame::Piece(piece) => {
                    if self.handle_piece(&piece).await? == false {
                        return Ok(false);
                    }
                }
                Frame::Cancel(_) => (),
            },
            None => return Ok(false),
        }

        return Ok(true);
    }

    fn handle_handshake(
        &mut self,
        handshake: &Handshake,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.keep_alive = true;
        handshake.validate(&self.info_hash)?;
        Ok(())
    }

    fn handle_keep_alive(&mut self) {
        self.keep_alive = true;
    }

    async fn handle_unchoke(&mut self) {
        self.keep_alive = true;
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
            self.piece_index = Some(index);

            let length = self.chunk_length();
            let msg = Request::new(index, self.buff_pos, length);
            println!("Wysyłam request");
            self.connection.write_msg(&msg).await.unwrap();
        }
    }
    async fn handle_have(&mut self, have: &Have) -> Result<(), Box<dyn std::error::Error>> {
        self.keep_alive = true;
        have.validate(self.pieces_count)?;

        let cmd = JobCmd::RecvHave {
            addr: self.connection.addr.clone(),
            index: have.index(),
        };

        self.job_ch.send(cmd).await?;
        Ok(())
    }

    async fn handle_bitfield(
        &mut self,
        bitfield: Bitfield,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.keep_alive = true;
        bitfield.validate(&self.pieces_count)?;

        let (resp_tx, resp_rx) = oneshot::channel();

        let cmd = JobCmd::RecvBitfield {
            addr: self.connection.addr.clone(),
            bitfield,
            resp_ch: resp_tx,
        };
        self.job_ch.send(cmd).await?;

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

        Ok(())
    }

    async fn handle_piece(&mut self, piece: &Piece) -> Result<bool, Box<dyn std::error::Error>> {
        self.keep_alive = true;
        piece.validate(
            self.piece_index.unwrap(),
            self.buff_pos,
            self.chunk_length(),
        )?;

        if self.save_piece(piece).await? == false {
            return Ok(false);
        }

        Ok(true)
    }

    async fn save_piece(&mut self, piece: &Piece) -> Result<bool, Box<dyn std::error::Error>> {
        println!("Piece");

        self.buff_piece[self.buff_pos..self.buff_pos + piece.block.len()]
            .copy_from_slice(&piece.block);
        self.buff_pos += piece.block.len();

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
                    self.piece_index = Some(index);
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
            let msg = Request::new(self.piece_index.unwrap(), self.buff_pos, length);
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
        let name = utils::hash_to_string(&self.piece_hash) + ".piece";
        fs::write(name, &self.buff_piece).unwrap();
    }
}
