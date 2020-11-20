use crate::connection::Connection;
use crate::constant::HASH_SIZE;
use crate::frame::{
    Bitfield, Frame, Handshake, Have, Interested, KeepAlive, NotInterested, Piece, Request,
};
use crate::{utils, Error};
use std::collections::VecDeque;
use std::fs;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::time;
use tokio::time::{Duration, Instant};

const PIECE_BLOCK_SIZE: usize = 16384;
const KEEP_ALIVE_INTERVAL_SEC: u64 = 2 * 60;
const STATS_INTERVAL_SEC: u64 = 10;
const MAX_STATS_QUEUE_SIZE: usize = 2;

#[derive(Debug, Clone)]
pub enum BroadCmd {
    SendHave { index: usize },
}
#[derive(Debug)]
pub enum JobCmd {
    RecvChoke {
        addr: String,
    },
    RecvUnchoke {
        addr: String,
        buffered_req: bool,
        resp_ch: oneshot::Sender<UnchokeCmd>,
    },
    RecvInterested {
        addr: String,
    },
    RecvNotInterested {
        addr: String,
    },
    RecvHave {
        addr: String,
        index: usize,
    },
    RecvBitfield {
        addr: String,
        bitfield: Bitfield,
        resp_ch: oneshot::Sender<BitfieldCmd>,
    },
    RecvRequest {
        addr: String,
        index: usize,
        begin: usize,
        length: usize,
        resp_ch: oneshot::Sender<RequestCmd>,
    },
    PieceDone {
        addr: String,
        resp_ch: oneshot::Sender<PieceDoneCmd>,
    },
    SyncStats {
        addr: String,
        downloaded_rate: f32,
        unexpected_piece: u32,
        rejected_piece: u32,
    },
    KillReq {
        addr: String,
        index: Option<usize>,
        reason: String,
    },
}

#[derive(Debug)]
pub enum BitfieldCmd {
    SendBitfield {
        bitfield: Bitfield,
        interested: bool,
    },
    PrepareKill,
}

#[derive(Debug)]
pub enum RequestCmd {
    SendPiece { hash: [u8; HASH_SIZE] },
    Ignore,
}

#[derive(Debug)]
pub enum UnchokeCmd {
    SendRequest {
        index: usize,
        piece_size: usize,
        piece_hash: [u8; HASH_SIZE],
    },
    SendNotInterested,
    Ignore,
}

#[derive(Debug)]
pub enum PieceDoneCmd {
    SendRequest {
        index: usize,
        piece_size: usize,
        piece_hash: [u8; HASH_SIZE],
    },
    PrepareKill,
}

pub struct Handler {
    connection: Connection,
    own_id: [u8; HASH_SIZE],
    peer_id: [u8; HASH_SIZE],
    info_hash: [u8; HASH_SIZE],
    pieces_count: usize,
    piece_data: Option<PieceData>,
    peer_status: Status,
    stats: Stats,
    msg_buff: Vec<Frame>,
    job_ch: mpsc::Sender<JobCmd>,
    broad_ch: broadcast::Receiver<BroadCmd>,
}

struct PieceData {
    index: usize,
    hash: [u8; HASH_SIZE],
    buff: Vec<u8>,
    requested: VecDeque<(usize, usize)>,
    left: VecDeque<(usize, usize)>,
}

struct Status {
    choked: bool,
    interested: bool,
    keep_alive: bool,
}

struct Stats {
    downloaded: VecDeque<usize>,
    unexpected_piece: u32,
    rejected_piece: u32,
}

impl PieceData {
    fn new(piece_index: usize, piece_size: usize, piece_hash: &[u8; HASH_SIZE]) -> PieceData {
        PieceData {
            index: piece_index,
            hash: *piece_hash,
            buff: vec![0; piece_size],
            requested: VecDeque::from(vec![]),
            left: Self::left(piece_size),
        }
    }

    fn left(piece_size: usize) -> VecDeque<(usize, usize)> {
        let mut res = VecDeque::from(vec![]);
        for block_begin in (0..piece_size).step_by(PIECE_BLOCK_SIZE) {
            let block_len = if block_begin + PIECE_BLOCK_SIZE > piece_size {
                piece_size % PIECE_BLOCK_SIZE
            } else {
                PIECE_BLOCK_SIZE
            };
            res.push_back((block_begin, block_len))
        }

        res
    }
}

impl Stats {
    fn new() -> Stats {
        Stats {
            downloaded: VecDeque::from(vec![0]),
            unexpected_piece: 0,
            rejected_piece: 0,
        }
    }

    fn update_downloaded(&mut self, amount: usize) {
        self.downloaded[0] += amount;
    }

    fn update_unexpected(&mut self) {
        self.unexpected_piece += 1;
    }

    fn update_rejected(&mut self) {
        self.rejected_piece += 1;
    }

    fn shift(&mut self) {
        if self.downloaded.len() == MAX_STATS_QUEUE_SIZE {
            self.downloaded.pop_back();
        }
        self.downloaded.push_front(0);
        self.unexpected_piece = 0;
        self.rejected_piece = 0;
    }

    fn downloaded_rate(&self) -> f32 {
        self.downloaded.iter().map(|d| *d as u32).sum::<u32>() as f32 / self.downloaded.len() as f32
    }
}

impl Handler {
    pub async fn run(
        addr: String,
        own_id: [u8; HASH_SIZE],
        peer_id: [u8; HASH_SIZE],
        info_hash: [u8; HASH_SIZE],
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
                peer_id,
                info_hash,
                pieces_count,
                piece_data: None,
                peer_status: Status {
                    choked: true,
                    interested: false,
                    keep_alive: false,
                },
                stats: Stats::new(),
                msg_buff: vec![],
                job_ch,
                broad_ch,
            };

            let reason = if let Err(e) = handler.event_loop().await {
                e.to_string()
            } else {
                "".to_string()
            };

            let index = handler.piece_data.map_or(None, |p| Some(p.index));
            Self::kill_req(
                &handler.connection.addr,
                &index,
                &reason,
                &mut handler.job_ch,
            )
            .await;
        } else {
            Self::kill_req(&addr, &None, &"Connection fail".to_string(), &mut job_ch).await;
        }
    }

    async fn kill_req(
        addr: &String,
        index: &Option<usize>,
        reason: &String,
        job_ch: &mut mpsc::Sender<JobCmd>,
    ) {
        job_ch
            .send(JobCmd::KillReq {
                addr: addr.clone(),
                index: *index,
                reason: reason.clone(),
            })
            .await
            .expect("Can't inform manager about KillReq");
    }

    async fn event_loop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.connection
            .send_msg(&Handshake::new(&self.info_hash, &self.own_id))
            .await?;

        let start = Instant::now() + Duration::from_secs(KEEP_ALIVE_INTERVAL_SEC);
        let mut interval = time::interval_at(start, Duration::from_secs(KEEP_ALIVE_INTERVAL_SEC));

        let start = Instant::now() + Duration::from_secs(STATS_INTERVAL_SEC);
        let mut download_rate = time::interval_at(start, Duration::from_secs(STATS_INTERVAL_SEC));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if !self.peer_status.keep_alive {
                        println!("Close connection, no messages from peer");
                        break;
                    }
                    self.send_keep_alive().await?;
                    self.peer_status.keep_alive = false;
                }
                _ = download_rate.tick() =>
                {
                    if self.stats.downloaded.len() == MAX_STATS_QUEUE_SIZE {
                        self.cmd_sync_stats().await?;
                    }
                    self.stats.shift();
                }
                cmd = self.broad_ch.recv() => self.send_have(cmd?).await?,
                frame = self.connection.recv_frame() => {
                    if self.handle_frame(frame?).await? == false {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_frame(
        &mut self,
        opt_frame: Option<Frame>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match opt_frame {
            Some(frame) => {
                self.peer_status.keep_alive = true;
                match frame {
                    Frame::Handshake(handshake) => self.handle_handshake(&handshake)?,
                    Frame::KeepAlive(_) => (),
                    Frame::Choke(_) => self.handle_choke().await?,
                    Frame::Unchoke(_) => self.handle_unchoke().await?,
                    Frame::Interested(_) => self.handle_interested().await?,
                    Frame::NotInterested(_) => self.handle_not_interested().await?,
                    Frame::Have(have) => self.handle_have(&have).await?,
                    Frame::Bitfield(bitfield) => self.handle_bitfield(bitfield).await?,
                    Frame::Request(request) => self.handle_request(request).await?,
                    Frame::Piece(piece) => {
                        if self.handle_piece(&piece).await? == false {
                            return Ok(false);
                        }
                    }
                    Frame::Cancel(_) => (),
                }
            }
            None => return Ok(false),
        }

        return Ok(true);
    }

    fn handle_handshake(
        &mut self,
        handshake: &Handshake,
    ) -> Result<(), Box<dyn std::error::Error>> {
        handshake.validate(&self.info_hash, &self.peer_id)?;
        Ok(())
    }

    async fn handle_choke(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.peer_status.choked = true;
        self.cmd_recv_choke().await?;
        Ok(())
    }

    async fn handle_unchoke(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.peer_status.choked = false;

        let buffered_req = self.any_request_in_msg_buff();
        if !self.msg_buff.is_empty() {
            for frame in self.msg_buff.iter() {
                self.connection.send_frame(frame).await?;
            }
            self.msg_buff.clear();
        }

        self.cmd_recv_unchoke(buffered_req).await?;

        Ok(())
    }

    async fn handle_interested(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.peer_status.interested = true;
        self.cmd_recv_interested().await?;
        Ok(())
    }

    async fn handle_not_interested(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.peer_status.interested = false;
        self.cmd_recv_not_interested().await?;
        Ok(())
    }

    async fn handle_have(&mut self, have: &Have) -> Result<(), Box<dyn std::error::Error>> {
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
        bitfield.validate(self.pieces_count)?;
        self.cmd_recv_bitfield(bitfield).await?;
        Ok(())
    }

    async fn handle_request(&mut self, request: Request) -> Result<(), Box<dyn std::error::Error>> {
        request.validate(self.pieces_count)?;
        self.cmd_recv_request(request).await?;
        Ok(())
    }

    async fn handle_piece(&mut self, piece: &Piece) -> Result<bool, Box<dyn std::error::Error>> {
        // Verify message
        let piece_data = self.piece_data.as_mut().ok_or(Error::NotFound)?;
        if !piece_data.requested.iter().any(|(block_begin, block_len)| {
            piece
                .validate(piece_data.index, *block_begin, *block_len)
                .is_ok()
        }) {
            Err(Error::NotFound)?;
        }

        // Removed piece from "requested" queue
        piece_data.requested.retain(|(block_begin, block_len)| {
            *block_begin == piece.block_begin() && *block_len == piece.block_len()
        });

        // Save piece block
        self.stats.update_downloaded(piece.block_len());
        piece_data.buff[piece.block_begin()..piece.block_begin() + piece.block_len()]
            .copy_from_slice(&piece.block());

        // Send new request or call manager to decide
        if piece_data.left.is_empty() && piece_data.requested.is_empty() {
            if !self.verify_piece_hash() {
                self.stats.update_rejected();
                return Ok(true);
            }

            self.save_piece_to_file();
            return Ok(self.cmd_recv_piece().await?);
        } else {
            self.send_request().await?;
        }

        Ok(true)
    }

    async fn cmd_recv_choke(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.job_ch
            .send(JobCmd::RecvChoke {
                addr: self.connection.addr.clone(),
            })
            .await?;

        Ok(())
    }

    async fn cmd_recv_unchoke(
        &mut self,
        buffered_req: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.job_ch
            .send(JobCmd::RecvUnchoke {
                addr: self.connection.addr.clone(),
                buffered_req,
                resp_ch: resp_tx,
            })
            .await?;

        match resp_rx.await? {
            UnchokeCmd::SendRequest {
                index,
                piece_size,
                piece_hash,
            } => {
                self.piece_data = Some(PieceData::new(index, piece_size, &piece_hash));

                // BEP3 suggests send more than one request to get good TCP performance (pipeline)
                self.send_request().await?;
                self.send_request().await?;
            }
            UnchokeCmd::SendNotInterested => (),
            UnchokeCmd::Ignore => (),
        }

        Ok(())
    }

    async fn cmd_recv_interested(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        self.job_ch
            .send(JobCmd::RecvInterested {
                addr: self.connection.addr.clone(),
            })
            .await?;

        return Ok(true);
    }

    async fn cmd_recv_not_interested(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        self.job_ch
            .send(JobCmd::RecvNotInterested {
                addr: self.connection.addr.clone(),
            })
            .await?;

        return Ok(true);
    }

    async fn cmd_recv_bitfield(
        &mut self,
        bitfield: Bitfield,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.job_ch
            .send(JobCmd::RecvBitfield {
                addr: self.connection.addr.clone(),
                bitfield,
                resp_ch: resp_tx,
            })
            .await?;

        match resp_rx.await? {
            BitfieldCmd::SendBitfield {
                bitfield,
                interested,
            } => {
                self.connection.send_msg(&bitfield).await?;

                if interested {
                    println!("Wysyłam Interested");
                    self.connection.send_msg(&Interested::new()).await?
                } else {
                    self.connection.send_msg(&NotInterested::new()).await?
                }
            }
            BitfieldCmd::PrepareKill => {
                // TODO
            }
        }

        Ok(())
    }

    async fn cmd_recv_request(
        &mut self,
        request: Request,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.job_ch
            .send(JobCmd::RecvRequest {
                addr: self.connection.addr.clone(),
                index: request.index(),
                begin: request.block_begin(),
                length: request.block_len(),
                resp_ch: resp_tx,
            })
            .await?;

        match resp_rx.await? {
            RequestCmd::SendPiece { hash } => (), // TODO
            RequestCmd::Ignore => (),             // TODO
        }

        Ok(())
    }

    async fn cmd_recv_piece(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.job_ch
            .send(JobCmd::PieceDone {
                addr: self.connection.addr.clone(),
                resp_ch: resp_tx,
            })
            .await?;

        match resp_rx.await? {
            PieceDoneCmd::SendRequest {
                index,
                piece_size,
                piece_hash,
            } => {
                self.piece_data = Some(PieceData::new(index, piece_size, &piece_hash));
                self.send_request().await?;
            }
            PieceDoneCmd::PrepareKill => return Ok(false),
        }

        Ok(true)
    }

    async fn cmd_sync_stats(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.job_ch
            .send(JobCmd::SyncStats {
                addr: self.connection.addr.clone(),
                downloaded_rate: self.stats.downloaded_rate(),
                unexpected_piece: self.stats.unexpected_piece,
                rejected_piece: self.stats.rejected_piece,
            })
            .await?;

        Ok(())
    }

    async fn send_keep_alive(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.connection.send_msg(&KeepAlive::new()).await?;
        Ok(())
    }

    async fn send_have(&mut self, cmd: BroadCmd) -> Result<(), Box<dyn std::error::Error>> {
        match cmd {
            BroadCmd::SendHave { index } => {
                if self.peer_status.choked {
                    self.msg_buff.push(Frame::Have(Have::new(index)));
                } else {
                    self.connection.send_msg(&Have::new(index)).await?;
                }
            }
        }

        Ok(())
    }

    async fn send_request(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(piece_data) = self.piece_data.as_mut() {
            if let Some((block_begin, block_len)) = piece_data.left.pop_front() {
                piece_data.requested.push_back((block_begin, block_len));

                let msg = Request::new(piece_data.index, block_begin, block_len);
                if self.peer_status.choked {
                    self.msg_buff.push(Frame::Request(msg));
                } else {
                    println!("Wysyłam kolejny request");
                    self.connection.send_msg(&msg).await?;
                }
            }
        }

        Ok(())
    }

    fn any_request_in_msg_buff(&self) -> bool {
        self.msg_buff.iter().any(|f| {
            if let Frame::Request(_) = f {
                true
            } else {
                false
            }
        })
    }

    fn verify_piece_hash(&self) -> bool {
        if let Some(piece_data) = self.piece_data.as_ref() {
            let mut m = sha1::Sha1::new();
            m.update(piece_data.buff.as_ref());
            println!("Checksum: {:?} {:?}", m.digest().bytes(), piece_data.hash);

            return m.digest().bytes() == piece_data.hash;
        }

        return false;
    }

    fn save_piece_to_file(&mut self) {
        let piece_data = self
            .piece_data
            .take()
            .ok_or(Error::NotFound)
            .expect("Saving to file: piece data not exist after validation");
        let name = utils::hash_to_string(&piece_data.hash) + ".piece";
        fs::write(name, &piece_data.buff).unwrap();
    }
}
