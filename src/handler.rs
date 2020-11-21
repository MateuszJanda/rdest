use crate::connection::Connection;
use crate::constant::{HASH_SIZE, PIECE_BLOCK_SIZE};
use crate::frame::{
    Bitfield, Frame, Handshake, Have, Interested, KeepAlive, NotInterested, Piece, Request,
};
use crate::{utils, Error};
use std::collections::VecDeque;
use std::fs;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::time;
use tokio::time::{Duration, Instant, Interval};

const KEEP_ALIVE_INTERVAL_SEC: u64 = 120;
const STATS_INTERVAL_SEC: u64 = 10;
const MAX_STATS_QUEUE_SIZE: usize = 2;

#[derive(Debug, Clone)]
pub enum BroadCmd {
    SendHave { index: usize },
    Change, // TODO
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
        block_begin: usize,
        block_length: usize,
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
    SendPiece {
        index: usize,
        block_begin: usize,
        block_length: usize,
        piece_hash: [u8; HASH_SIZE],
    },
    Ignore,
}

#[derive(Debug)]
pub enum UnchokeCmd {
    SendRequest {
        index: usize,
        piece_length: usize,
        piece_hash: [u8; HASH_SIZE],
    },
    SendNotInterested,
    Ignore,
}

#[derive(Debug)]
pub enum PieceDoneCmd {
    SendRequest {
        index: usize,
        piece_length: usize,
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
    piece_send: Option<PieceSend>,
    piece_recv: Option<PieceRecv>,
    peer_status: Status,
    stats: Stats,
    msg_buff: Vec<Frame>,
    job_ch: mpsc::Sender<JobCmd>,
    broad_ch: broadcast::Receiver<BroadCmd>,
}

struct PieceSend {
    index: usize,
    buff: Vec<u8>,
}

struct PieceRecv {
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

impl PieceRecv {
    fn new(index: usize, piece_length: usize, piece_hash: &[u8; HASH_SIZE]) -> PieceRecv {
        PieceRecv {
            index,
            hash: *piece_hash,
            buff: vec![0; piece_length],
            requested: VecDeque::from(vec![]),
            left: Self::left(piece_length),
        }
    }

    fn left(piece_length: usize) -> VecDeque<(usize, usize)> {
        let mut res = VecDeque::from(vec![]);
        for block_begin in (0..piece_length).step_by(PIECE_BLOCK_SIZE) {
            let block_length = if block_begin + PIECE_BLOCK_SIZE > piece_length {
                piece_length % PIECE_BLOCK_SIZE
            } else {
                PIECE_BLOCK_SIZE
            };
            res.push_back((block_begin, block_length))
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
        match TcpStream::connect(&addr).await {
            Ok(stream) => {
                println!("connected");

                let mut handler = Handler {
                    connection: Connection::new(addr, stream),
                    own_id,
                    peer_id,
                    info_hash,
                    pieces_count,
                    piece_send: None,
                    piece_recv: None,
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

                let reason = match handler.event_loop().await {
                    Ok(_) => "".to_string(),
                    Err(e) => e.to_string(),
                };

                let index = handler.piece_recv.map_or(None, |p| Some(p.index));
                Self::kill_req(
                    &handler.connection.addr,
                    &index,
                    &reason,
                    &mut handler.job_ch,
                )
                .await;
            }
            Err(_) => {
                Self::kill_req(&addr, &None, &"Connection fail".to_string(), &mut job_ch).await
            }
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

        let mut keep_alive_timer = self.start_keep_alive_timer();
        let mut sync_stats_timer = self.start_sync_stats_timer();

        loop {
            tokio::select! {
                _ = keep_alive_timer.tick() => self.timeout_keep_alive().await?,
                _ = sync_stats_timer.tick() => self.timeout_sync_stats().await?,
                cmd = self.broad_ch.recv() => self.handle_manager(cmd?).await?,
                frame = self.connection.recv_frame() => {
                    if self.handle_frame(frame?).await? == false {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    fn start_keep_alive_timer(&self) -> Interval {
        let start = Instant::now() + Duration::from_secs(KEEP_ALIVE_INTERVAL_SEC);
        time::interval_at(start, Duration::from_secs(KEEP_ALIVE_INTERVAL_SEC))
    }

    fn start_sync_stats_timer(&self) -> Interval {
        let start = Instant::now() + Duration::from_secs(STATS_INTERVAL_SEC);
        time::interval_at(start, Duration::from_secs(STATS_INTERVAL_SEC))
    }

    async fn timeout_keep_alive(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.peer_status.keep_alive {
            Err(Error::KeepAliveTimeout)?
        }
        self.send_keep_alive().await?;
        self.peer_status.keep_alive = false;
        Ok(())
    }

    async fn timeout_sync_stats(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.stats.downloaded.len() == MAX_STATS_QUEUE_SIZE {
            self.cmd_sync_stats().await?;
        }
        self.stats.shift();
        Ok(())
    }

    async fn handle_frame(
        &mut self,
        opt_frame: Option<Frame>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match opt_frame {
            Some(frame) => {
                self.peer_status.keep_alive = true;
                let handled = match frame {
                    Frame::Handshake(handshake) => self.handle_handshake(&handshake)?,
                    Frame::KeepAlive(_) => true,
                    Frame::Choke(_) => self.handle_choke().await?,
                    Frame::Unchoke(_) => self.handle_unchoke().await?,
                    Frame::Interested(_) => self.handle_interested().await?,
                    Frame::NotInterested(_) => self.handle_not_interested().await?,
                    Frame::Have(have) => self.handle_have(&have).await?,
                    Frame::Bitfield(bitfield) => self.handle_bitfield(bitfield).await?,
                    Frame::Request(request) => self.handle_request(request).await?,
                    Frame::Piece(piece) => self.handle_piece(&piece).await?,
                    Frame::Cancel(_) => true,
                };

                if handled == false {
                    return Ok(false);
                }
            }
            None => return Ok(false),
        }

        return Ok(true);
    }

    fn handle_handshake(
        &mut self,
        handshake: &Handshake,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        handshake.validate(&self.info_hash, &self.peer_id)?;
        Ok(true)
    }

    async fn handle_choke(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        self.peer_status.choked = true;
        self.cmd_recv_choke().await?;
        Ok(true)
    }

    async fn handle_unchoke(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        self.peer_status.choked = false;

        let buffered_req = self.any_request_in_msg_buff();
        if !self.msg_buff.is_empty() {
            for frame in self.msg_buff.iter() {
                self.connection.send_frame(frame).await?;
            }
            self.msg_buff.clear();
        }

        self.cmd_recv_unchoke(buffered_req).await?;

        Ok(true)
    }

    async fn handle_interested(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        self.peer_status.interested = true;
        self.cmd_recv_interested().await?;
        Ok(true)
    }

    async fn handle_not_interested(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        self.peer_status.interested = false;
        self.cmd_recv_not_interested().await?;
        Ok(true)
    }

    async fn handle_have(&mut self, have: &Have) -> Result<bool, Box<dyn std::error::Error>> {
        have.validate(self.pieces_count)?;

        let cmd = JobCmd::RecvHave {
            addr: self.connection.addr.clone(),
            index: have.index(),
        };

        self.job_ch.send(cmd).await?;
        Ok(true)
    }

    async fn handle_bitfield(
        &mut self,
        bitfield: Bitfield,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        bitfield.validate(self.pieces_count)?;
        self.cmd_recv_bitfield(bitfield).await
    }

    async fn handle_request(
        &mut self,
        request: Request,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match &self.piece_send {
            Some(p) => request.validate(Some(p.buff.len()), self.pieces_count)?,
            None => request.validate(None, self.pieces_count)?,
        }

        match &self.piece_send {
            Some(p) if p.index == request.index() => {
                self.send_piece(
                    request.index(),
                    request.block_begin(),
                    request.block_length(),
                )
                .await?;
            }
            _ => self.cmd_recv_request(request).await?,
        }

        Ok(true)
    }

    async fn handle_piece(&mut self, piece: &Piece) -> Result<bool, Box<dyn std::error::Error>> {
        // Verify message
        let piece_recv = self.piece_recv.as_mut().ok_or(Error::NotFound)?;
        if !piece_recv
            .requested
            .iter()
            .any(|(block_begin, block_length)| {
                piece
                    .validate(piece_recv.index, *block_begin, *block_length)
                    .is_ok()
            })
        {
            Err(Error::NotFound)?;
        }

        // Removed piece from "requested" queue
        piece_recv.requested.retain(|(block_begin, block_length)| {
            !(*block_begin == piece.block_begin() && *block_length == piece.block_length())
        });

        // Save piece block
        self.stats.update_downloaded(piece.block_length());
        piece_recv.buff[piece.block_begin()..piece.block_begin() + piece.block_length()]
            .copy_from_slice(&piece.block());

        // Send new request or call manager to decide
        if piece_recv.left.is_empty() && piece_recv.requested.is_empty() {
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
                piece_length,
                piece_hash,
            } => {
                self.piece_recv = Some(PieceRecv::new(index, piece_length, &piece_hash));

                // BEP3 suggests send more than one request to get good better TCP performance (pipeline)
                self.send_request().await?;
                self.send_request().await?;
            }
            UnchokeCmd::SendNotInterested => (), // TODO
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
    ) -> Result<bool, Box<dyn std::error::Error>> {
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
                match interested {
                    true => self.connection.send_msg(&Interested::new()).await?,
                    false => self.connection.send_msg(&NotInterested::new()).await?,
                }
            }
            BitfieldCmd::PrepareKill => return Ok(false),
        }

        Ok(true)
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
                block_begin: request.block_begin(),
                block_length: request.block_length(),
                resp_ch: resp_tx,
            })
            .await?;

        match resp_rx.await? {
            RequestCmd::SendPiece {
                index,
                block_begin,
                block_length,
                piece_hash,
            } => {
                if let Some(piece_send) = &self.piece_send {
                    if piece_send.index != index {
                        self.load_piece_from_file(index, &piece_hash)?;
                    }
                }

                self.send_piece(index, block_begin, block_length).await?;
            }
            RequestCmd::Ignore => (),
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
                piece_length,
                piece_hash,
            } => {
                self.piece_recv = Some(PieceRecv::new(index, piece_length, &piece_hash));
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

    async fn handle_manager(&mut self, cmd: BroadCmd) -> Result<(), Box<dyn std::error::Error>> {
        match cmd {
            BroadCmd::SendHave { index } => match self.peer_status.choked {
                true => self.msg_buff.push(Frame::Have(Have::new(index))),
                false => self.connection.send_msg(&Have::new(index)).await?,
            },
            BroadCmd::Change => (),
        }

        Ok(())
    }

    async fn send_request(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(piece_recv) = self.piece_recv.as_mut() {
            if let Some((block_begin, block_len)) = piece_recv.left.pop_front() {
                piece_recv.requested.push_back((block_begin, block_len));

                let msg = Request::new(piece_recv.index, block_begin, block_len);
                println!("Wysyłam kolejny request");
                match self.peer_status.choked {
                    true => self.msg_buff.push(Frame::Request(msg)),
                    false => self.connection.send_msg(&msg).await?,
                }
            }
        }

        Ok(())
    }

    async fn send_piece(
        &mut self,
        index: usize,
        block_begin: usize,
        block_length: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match &self.piece_send {
            Some(piece_send) => {
                self.connection
                    .send_msg(&Piece::new(
                        index,
                        block_begin,
                        piece_send.buff[block_begin..block_begin + block_length].to_vec(),
                    ))
                    .await?;
                Ok(())
            }
            None => Err(Error::NotFound)?,
        }
    }

    fn any_request_in_msg_buff(&self) -> bool {
        self.msg_buff.iter().any(|f| match f {
            Frame::Request(_) => true,
            _ => false,
        })
    }

    fn verify_piece_hash(&self) -> bool {
        match self.piece_recv.as_ref() {
            Some(piece_recv) => {
                let mut m = sha1::Sha1::new();
                m.update(piece_recv.buff.as_ref());
                println!("Checksum: {:?} {:?}", m.digest().bytes(), piece_recv.hash);

                return m.digest().bytes() == piece_recv.hash;
            }
            None => false,
        }
    }

    fn load_piece_from_file(
        &mut self,
        index: usize,
        piece_hash: &[u8; HASH_SIZE],
    ) -> Result<(), Error> {
        let name = utils::hash_to_string(piece_hash) + ".piece";
        match fs::read(name) {
            Ok(data) => {
                self.piece_send = Some(PieceSend { index, buff: data });
                Ok(())
            }
            Err(_) => Err(Error::FileNotFound),
        }
    }

    fn save_piece_to_file(&mut self) {
        let piece_recv = self
            .piece_recv
            .take()
            .ok_or(Error::NotFound)
            .expect("Saving to file: piece data not exist after validation");
        let name = utils::hash_to_string(&piece_recv.hash) + ".piece";
        fs::write(name, &piece_recv.buff).unwrap();
    }
}
