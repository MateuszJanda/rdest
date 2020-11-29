use crate::connection::Connection;
use crate::constant::{HASH_SIZE, PIECE_BLOCK_SIZE};
use crate::frame::{
    Bitfield, Choke, Frame, Handshake, Have, Interested, KeepAlive, NotInterested, Piece, Request,
    Unchoke,
};
use crate::{utils, Error};
use std::collections::{HashMap, VecDeque};
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
    SendHave {
        index: usize,
    },
    ChangeOwnState {
        am_choked_map: HashMap<String, bool>,
    },
}
#[derive(Debug)]
pub enum JobCmd {
    Init {
        addr: String,
        resp_ch: oneshot::Sender<InitCmd>,
    },
    RecvChoke {
        addr: String,
    },
    RecvUnchoke {
        addr: String,
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
        downloaded_rate: Option<u32>,
        uploaded_rate: Option<u32>,
        rejected_piece: u32,
    },
    KillReq {
        addr: String,
        index: Option<usize>,
        reason: String,
    },
}

#[derive(Debug)]
pub enum InitCmd {
    SendBitfield { bitfield: Bitfield },
}

#[derive(Debug)]
pub enum UnchokeCmd {
    SendInterestedAndRequest {
        index: usize,
        piece_length: usize,
        piece_hash: [u8; HASH_SIZE],
    },
    SendNotInterested,
}

#[derive(Debug)]
pub enum BitfieldCmd {
    SendState {
        with_am_unchoked: bool,
        am_interested: bool,
    },
}

#[derive(Debug)]
pub enum RequestCmd {
    LoadAndSendPiece {
        index: usize,
        block_begin: usize,
        block_length: usize,
        piece_hash: [u8; HASH_SIZE],
    },
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
    peer_id: Option<[u8; HASH_SIZE]>,
    info_hash: [u8; HASH_SIZE],
    pieces_num: usize,
    piece_tx: Option<PieceTx>,
    piece_rx: Option<PieceRx>,
    peer_state: State,
    stats: Stats,
    msg_buff: Vec<Frame>,
    job_ch: mpsc::Sender<JobCmd>,
    broad_ch: broadcast::Receiver<BroadCmd>,
}

struct PieceTx {
    index: usize,
    buff: Vec<u8>,
}

struct PieceRx {
    index: usize,
    hash: [u8; HASH_SIZE],
    buff: Vec<u8>,
    requested: VecDeque<(usize, usize)>,
    left: VecDeque<(usize, usize)>,
}

struct State {
    choked: bool,
    interested: bool,
    keep_alive: bool,
}

struct Stats {
    downloaded: VecDeque<usize>,
    uploaded: VecDeque<usize>,
    rejected_piece: u32,
}

impl PieceRx {
    fn new(index: usize, piece_length: usize, piece_hash: &[u8; HASH_SIZE]) -> PieceRx {
        PieceRx {
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
            uploaded: VecDeque::from(vec![0]),
            rejected_piece: 0,
        }
    }

    fn update_downloaded(&mut self, amount: usize) {
        self.downloaded[0] += amount;
    }

    fn update_uploaded(&mut self, amount: usize) {
        self.uploaded[0] += amount;
    }

    fn update_rejected_piece(&mut self) {
        self.rejected_piece += 1;
    }

    fn shift(&mut self) {
        if self.downloaded.len() == MAX_STATS_QUEUE_SIZE {
            self.downloaded.pop_back();
            self.uploaded.pop_back();
        }
        self.downloaded.push_front(0);
        self.uploaded.push_front(0);
        self.rejected_piece = 0;
    }

    fn downloaded_rate(&self) -> Option<u32> {
        if self.downloaded.len() != MAX_STATS_QUEUE_SIZE {
            return None;
        }

        Some(self.downloaded.iter().map(|d| *d as u32).sum::<u32>() / self.downloaded.len() as u32)
    }

    fn uploaded_rate(&self) -> Option<u32> {
        if self.uploaded.len() != MAX_STATS_QUEUE_SIZE {
            return None;
        }

        Some(self.uploaded.iter().map(|d| *d as u32).sum::<u32>() / self.uploaded.len() as u32)
    }
}

impl Handler {
    pub async fn run(
        addr: String,
        own_id: [u8; HASH_SIZE],
        peer_id: Option<[u8; HASH_SIZE]>,
        info_hash: [u8; HASH_SIZE],
        pieces_num: usize,
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
                    pieces_num,
                    piece_tx: None,
                    piece_rx: None,
                    peer_state: State {
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
                    Ok(_) => "End job normally".to_string(),
                    Err(e) => e.to_string(),
                };

                let index = handler.piece_rx.map_or(None, |p| Some(p.index));
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
        if self.peer_id.is_some() {
            self.init_handshake().await?;
        }

        let mut keep_alive_timer = self.start_keep_alive_timer();
        let mut sync_stats_timer = self.start_sync_stats_timer();

        loop {
            tokio::select! {
                _ = keep_alive_timer.tick() => self.timeout_keep_alive().await?,
                _ = sync_stats_timer.tick() => self.timeout_sync_stats().await?,
                Ok(cmd) = self.broad_ch.recv() => self.handle_manager_cmd(cmd).await?,
                Ok(frame) = self.connection.recv_frame() => {
                    if self.handle_frame(frame).await? == false {
                        println!("handle_frame - koncze normalnie");
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
        if !self.peer_state.keep_alive {
            return Err(Error::KeepAliveTimeout.into());
        }
        self.send_keep_alive().await?;
        self.peer_state.keep_alive = false;
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
                self.peer_state.keep_alive = true;
                let handled = match frame {
                    Frame::Handshake(handshake) => self.handle_handshake(&handshake).await?,
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
            None => {
                return Err(Error::ConnectionClosed.into());
            }
        }

        return Ok(true);
    }

    async fn handle_handshake(
        &mut self,
        handshake: &Handshake,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        handshake.validate(&self.info_hash, &self.peer_id)?;

        let peer_init_handshake = self.peer_id.is_none();
        self.peer_id = Some(*handshake.peer_id());

        if peer_init_handshake {
            self.init_handshake().await?;
        }

        Ok(true)
    }

    async fn handle_choke(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        self.peer_state.choked = true;
        self.cmd_recv_choke().await?;
        Ok(true)
    }

    async fn handle_unchoke(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        self.peer_state.choked = false;

        if !self.msg_buff.is_empty() {
            for frame in self.msg_buff.iter() {
                self.connection.send_frame(frame).await?;
            }
            self.msg_buff.clear();
        }

        self.cmd_recv_unchoke().await?;

        Ok(true)
    }

    async fn handle_interested(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        self.peer_state.interested = true;
        self.cmd_recv_interested().await?;
        Ok(true)
    }

    async fn handle_not_interested(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        self.peer_state.interested = false;
        self.cmd_recv_not_interested().await?;
        Ok(true)
    }

    async fn handle_have(&mut self, have: &Have) -> Result<bool, Box<dyn std::error::Error>> {
        have.validate(self.pieces_num)?;

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
        bitfield.validate(self.pieces_num)?;
        self.cmd_recv_bitfield(bitfield).await?;
        Ok(true)
    }

    async fn handle_request(
        &mut self,
        request: Request,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match &self.piece_tx {
            Some(piece_tx) => request.validate(Some(piece_tx.buff.len()), self.pieces_num)?,
            None => request.validate(None, self.pieces_num)?,
        }

        match &self.piece_tx {
            Some(piece_tx) if piece_tx.index == request.index() => {
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
        let piece_rx = self.piece_rx.as_mut().ok_or(Error::PieceNotRequested)?;
        if !piece_rx
            .requested
            .iter()
            .any(|(block_begin, block_length)| {
                piece
                    .validate(piece_rx.index, *block_begin, *block_length)
                    .is_ok()
            })
        {
            println!("handle_piece {:?}", piece_rx.requested);
            return Err(Error::BlockNotRequested.into());
        }

        // Removed piece from "requested" queue
        piece_rx.requested.retain(|(block_begin, block_length)| {
            !(*block_begin == piece.block_begin() && *block_length == piece.block_length())
        });

        self.stats.update_downloaded(piece.block_length());
        // Save piece block
        piece_rx.buff[piece.block_begin()..piece.block_begin() + piece.block_length()]
            .copy_from_slice(&piece.block());

        // Send new request or call manager to decide
        if piece_rx.left.is_empty() && piece_rx.requested.is_empty() {
            if !self.verify_piece_hash() {
                self.stats.update_rejected_piece();
                return Ok(true);
            }

            self.save_piece_to_file();
            return Ok(self.cmd_recv_piece().await?);
        } else {
            self.send_request().await?;
        }

        Ok(true)
    }

    async fn init_handshake(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("wysyłam handshake");
        self.connection
            .send_msg(&Handshake::new(&self.info_hash, &self.own_id))
            .await?;

        let (resp_tx, resp_rx) = oneshot::channel();
        self.job_ch
            .send(JobCmd::Init {
                addr: self.connection.addr.clone(),
                resp_ch: resp_tx,
            })
            .await?;

        match resp_rx.await? {
            InitCmd::SendBitfield { bitfield } => {
                println!("wysyłam bitfield");
                self.connection.send_msg(&bitfield).await?;
            }
        }

        Ok(())
    }

    async fn cmd_recv_choke(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.job_ch
            .send(JobCmd::RecvChoke {
                addr: self.connection.addr.clone(),
            })
            .await?;

        Ok(())
    }

    async fn cmd_recv_unchoke(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.job_ch
            .send(JobCmd::RecvUnchoke {
                addr: self.connection.addr.clone(),
                resp_ch: resp_tx,
            })
            .await?;

        match resp_rx.await? {
            UnchokeCmd::SendInterestedAndRequest {
                index,
                piece_length,
                piece_hash,
            } => {
                self.piece_rx = Some(PieceRx::new(index, piece_length, &piece_hash));
                self.connection.send_msg(&Interested::new()).await?;

                // BEP3 suggests send more than one request to get good better TCP performance (pipeline)
                self.send_request().await?;
                self.send_request().await?;
            }
            UnchokeCmd::SendNotInterested => {
                self.connection.send_msg(&NotInterested::new()).await?
            }
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
            BitfieldCmd::SendState {
                with_am_unchoked,
                am_interested,
            } => {
                match with_am_unchoked {
                    true => self.connection.send_msg(&Unchoke::new()).await?,
                    false => (),
                }

                match am_interested {
                    true => self.connection.send_msg(&Interested::new()).await?,
                    false => self.connection.send_msg(&NotInterested::new()).await?,
                }
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
                block_begin: request.block_begin(),
                block_length: request.block_length(),
                resp_ch: resp_tx,
            })
            .await?;

        match resp_rx.await? {
            RequestCmd::LoadAndSendPiece {
                index,
                block_begin,
                block_length,
                piece_hash,
            } => {
                self.load_piece_from_file(index, &piece_hash)?;
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
                self.piece_rx = Some(PieceRx::new(index, piece_length, &piece_hash));
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
                uploaded_rate: self.stats.uploaded_rate(),
                rejected_piece: self.stats.rejected_piece,
            })
            .await?;

        Ok(())
    }

    async fn send_keep_alive(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.connection.send_msg(&KeepAlive::new()).await?;
        Ok(())
    }

    async fn handle_manager_cmd(
        &mut self,
        cmd: BroadCmd,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match cmd {
            BroadCmd::SendHave { index } => match self.peer_state.choked {
                true => self.msg_buff.push(Frame::Have(Have::new(index))),
                false => self.connection.send_msg(&Have::new(index)).await?,
            },
            BroadCmd::ChangeOwnState { am_choked_map } => {
                match am_choked_map.get(&self.connection.addr) {
                    Some(true) => self.connection.send_msg(&Choke::new()).await?,
                    Some(false) => self.connection.send_msg(&Unchoke::new()).await?,
                    None => (),
                }
            }
        }

        Ok(())
    }

    async fn send_request(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(piece_rx) = self.piece_rx.as_mut() {
            if let Some((block_begin, block_len)) = piece_rx.left.pop_front() {
                piece_rx.requested.push_back((block_begin, block_len));

                println!("Wysyłam kolejny request");
                let msg = Request::new(piece_rx.index, block_begin, block_len);
                self.connection.send_msg(&msg).await?;
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
        match &self.piece_tx {
            Some(piece_tx) => {
                self.stats.update_uploaded(block_length);
                self.connection
                    .send_msg(&Piece::new(
                        index,
                        block_begin,
                        piece_tx.buff[block_begin..block_begin + block_length].to_vec(),
                    ))
                    .await?;
                Ok(())
            }
            None => Err(Error::PieceNotLoaded.into()),
        }
    }

    fn verify_piece_hash(&self) -> bool {
        match self.piece_rx.as_ref() {
            Some(piece_rx) => {
                let mut m = sha1::Sha1::new();
                m.update(piece_rx.buff.as_ref());
                println!("Checksum: {:?} {:?}", m.digest().bytes(), piece_rx.hash);

                return m.digest().bytes() == piece_rx.hash;
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
                self.piece_tx = Some(PieceTx { index, buff: data });
                Ok(())
            }
            Err(_) => Err(Error::FileNotFound),
        }
    }

    fn save_piece_to_file(&mut self) {
        let piece_rx = self
            .piece_rx
            .take()
            .ok_or(Error::PieceBuffMissing)
            .expect("Saving to file: piece data not exist after validation");
        let name = utils::hash_to_string(&piece_rx.hash) + ".piece";
        fs::write(name, &piece_rx.buff).unwrap();
    }
}
