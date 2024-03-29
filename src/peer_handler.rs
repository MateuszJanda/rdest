// Copyright 2020 Mateusz Janda.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::commands::{
    BitfieldCmd, BroadCmd, HaveCmd, InitCmd, NotInterestedCmd, PeerCmd, PieceCmd, ReqData,
    RequestCmd, UnchokeCmd,
};
use crate::connection::Connection;
use crate::constants::{HASH_SIZE, PEER_ID_SIZE, PIECE_BLOCK_SIZE};
use crate::frame::Frame;
use crate::messages::{
    Bitfield, Cancel, Choke, Handshake, Have, Interested, KeepAlive, NotInterested, Piece, Request,
    Unchoke,
};
use crate::{utils, Error};
use std::collections::VecDeque;
use tokio::fs;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::time;
use tokio::time::{Duration, Instant, Interval};

const KEEP_ALIVE_LIMIT: u32 = 2;
const KEEP_ALIVE_INTERVAL_SEC: u64 = 120;
const STATS_INTERVAL_SEC: u64 = 10;
const MAX_STATS_QUEUE_SIZE: usize = 2;

pub struct PeerHandler {
    connection: Connection,
    own_id: [u8; PEER_ID_SIZE],
    peer_id: Option<[u8; PEER_ID_SIZE]>,
    info_hash: [u8; HASH_SIZE],
    pieces_num: usize,
    piece_tx: Option<PieceTx>,
    piece_rx: Option<PieceRx>,
    peer_state: State,
    stats: Stats,
    msg_buff: Vec<Frame>,
    peer_ch: mpsc::Sender<PeerCmd>,
    broad_ch: broadcast::Receiver<BroadCmd>,
}

struct PieceTx {
    piece_index: usize,
    buff: Vec<u8>,
}

struct PieceRx {
    piece_index: usize,
    hash: [u8; HASH_SIZE],
    buff: Vec<u8>,
    requested: VecDeque<(usize, usize)>,
    left: VecDeque<(usize, usize)>,
}

struct State {
    choked: bool,
    interested: bool,
    keep_alive: u32,
}

struct Stats {
    downloaded: VecDeque<usize>,
    uploaded: VecDeque<usize>,
    unexpected_blocks: usize,
}

impl PieceRx {
    fn new(req_data: &ReqData) -> PieceRx {
        PieceRx {
            piece_index: req_data.piece_index,
            hash: req_data.piece_hash,
            buff: vec![0; req_data.piece_length],
            requested: VecDeque::from(vec![]),
            left: Self::left(req_data.piece_length),
        }
    }

    fn left(piece_length: usize) -> VecDeque<(usize, usize)> {
        let mut res = VecDeque::from(vec![]);
        for block_begin in (0..piece_length).step_by(PIECE_BLOCK_SIZE) {
            let block_length = match block_begin + PIECE_BLOCK_SIZE > piece_length {
                true => piece_length % PIECE_BLOCK_SIZE,
                false => PIECE_BLOCK_SIZE,
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
            unexpected_blocks: 0,
        }
    }

    fn update_downloaded(&mut self, amount: usize) {
        self.downloaded[0] += amount;
    }

    fn update_uploaded(&mut self, amount: usize) {
        self.uploaded[0] += amount;
    }

    fn increment_unexpected_piece(&mut self) {
        self.unexpected_blocks += 1;
    }

    fn shift(&mut self) {
        if self.downloaded.len() == MAX_STATS_QUEUE_SIZE {
            self.downloaded.pop_back();
            self.uploaded.pop_back();
        }
        self.downloaded.push_front(0);
        self.uploaded.push_front(0);

        self.unexpected_blocks = 0;
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

    fn unexpected_blocks(&mut self) -> usize {
        self.unexpected_blocks
    }
}

impl PeerHandler {
    pub fn new(
        addr: String,
        own_id: [u8; PEER_ID_SIZE],
        peer_id: Option<[u8; PEER_ID_SIZE]>,
        info_hash: [u8; HASH_SIZE],
        pieces_num: usize,
        peer_ch: mpsc::Sender<PeerCmd>,
        broad_ch: broadcast::Receiver<BroadCmd>,
    ) -> PeerHandler {
        PeerHandler {
            connection: Connection::new(addr),
            own_id,
            peer_id,
            info_hash,
            pieces_num,
            piece_tx: None,
            piece_rx: None,
            peer_state: State {
                choked: true,
                interested: false,
                keep_alive: 0,
            },
            stats: Stats::new(),
            msg_buff: vec![],
            peer_ch,
            broad_ch,
        }
    }

    pub async fn run_incoming(&mut self) {
        match TcpStream::connect(&self.connection.addr).await {
            Ok(socket) => {
                self.connection.with_socket(socket);
                self.run().await;
            }
            Err(_) => {
                Self::kill_req(
                    &self.connection.addr,
                    &"Connection fail".to_string(),
                    &mut self.peer_ch,
                )
                .await
            }
        }
    }

    pub async fn run_outgoing(&mut self, socket: TcpStream) {
        self.connection.with_socket(socket);
        self.run().await;
    }

    async fn run(&mut self) {
        let reason = match self.event_loop().await {
            Ok(_) => "End job normally".to_string(),
            Err(e) => e.to_string(),
        };

        Self::kill_req(&self.connection.addr, &reason, &mut self.peer_ch).await;
    }

    async fn kill_req(addr: &String, reason: &String, peer_ch: &mut mpsc::Sender<PeerCmd>) {
        peer_ch
            .send(PeerCmd::KillReq {
                addr: addr.clone(),
                reason: reason.clone(),
            })
            .await
            .expect("Can't inform manager about KillReq");
    }

    async fn event_loop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(peer_id) = self.peer_id {
            self.init_handshake(peer_id).await?;
        }

        let mut keep_alive_timer = self.start_keep_alive_timer();
        let mut sync_stats_timer = self.start_sync_stats_timer();

        loop {
            tokio::select! {
                _ = keep_alive_timer.tick() => self.timeout_keep_alive().await?,
                _ = sync_stats_timer.tick() => self.timeout_sync_stats().await?,
                Ok(cmd) = self.broad_ch.recv() => {
                    if self.handle_manager_cmd(cmd).await? == false {
                        break;
                    }
                },
                Ok(frame) = self.connection.recv_frame() => {
                    if self.handle_frame(frame).await? == false {
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
        if self.peer_state.keep_alive == KEEP_ALIVE_LIMIT {
            return Err(Error::KeepAliveTimeout.into());
        }
        self.connection.send_msg(&KeepAlive::new()).await?;
        self.peer_state.keep_alive += 1;
        Ok(())
    }

    async fn timeout_sync_stats(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.stats.downloaded.len() == MAX_STATS_QUEUE_SIZE {
            self.trigger_cmd_sync_stats().await?;
        }
        self.stats.shift();
        Ok(())
    }

    async fn handle_manager_cmd(
        &mut self,
        cmd: BroadCmd,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match cmd {
            BroadCmd::SendHave { piece_index } => {
                if let Some(piece_rx) = &self.piece_rx {
                    if piece_rx.piece_index == piece_index {
                        for (block_begin, block_length) in &piece_rx.requested {
                            self.connection
                                .send_msg(&Cancel::new(piece_index, *block_begin, *block_length))
                                .await?
                        }

                        self.piece_rx = None;
                        self.trigger_cmd_piece_finish(false).await?;
                    }
                }

                match self.peer_state.choked {
                    true => self.msg_buff.push(Frame::Have(Have::new(piece_index))),
                    false => self.connection.send_msg(&Have::new(piece_index)).await?,
                }
            }
            BroadCmd::SendOwnState { am_choked_map } => {
                match am_choked_map.get(&self.connection.addr) {
                    Some(true) => self.connection.send_msg(&Choke::new()).await?,
                    Some(false) => self.connection.send_msg(&Unchoke::new()).await?,
                    None => (),
                }
            }
        }

        Ok(true)
    }

    async fn handle_frame(
        &mut self,
        opt_frame: Option<Frame>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match opt_frame {
            Some(frame) => {
                self.peer_state.keep_alive = match frame {
                    Frame::KeepAlive(_) => self.peer_state.keep_alive,
                    _ => 0,
                };

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
            None => return Err(Error::ConnectionClosed.into()),
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
            self.init_handshake(*handshake.peer_id()).await?;
        }

        Ok(true)
    }

    async fn handle_choke(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        self.peer_state.choked = true;
        self.trigger_cmd_recv_choke().await?;
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

        self.trigger_cmd_recv_unchoke().await?;
        Ok(true)
    }

    async fn handle_interested(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        self.peer_state.interested = true;
        self.trigger_cmd_recv_interested().await?;
        Ok(true)
    }

    async fn handle_not_interested(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        self.peer_state.interested = false;
        self.trigger_cmd_recv_not_interested().await
    }

    async fn handle_have(&mut self, have: &Have) -> Result<bool, Box<dyn std::error::Error>> {
        have.validate(self.pieces_num)?;
        self.trigger_cmd_recv_have(have).await?;
        Ok(true)
    }

    async fn handle_bitfield(
        &mut self,
        bitfield: Bitfield,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        bitfield.validate(self.pieces_num)?;
        self.trigger_cmd_recv_bitfield(bitfield).await?;
        Ok(true)
    }

    async fn handle_request(
        &mut self,
        request: Request,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match &self.piece_tx {
            Some(piece_tx) => {
                if piece_tx.piece_index != request.piece_index() {
                    self.trigger_cmd_recv_request(&request).await?;
                }
            }
            None => self.trigger_cmd_recv_request(&request).await?,
        };

        match &self.piece_tx {
            Some(piece_tx) => {
                request.validate(piece_tx.piece_index, self.pieces_num, piece_tx.buff.len())?;
                self.send_piece(&request).await?
            }
            None => (),
        }

        Ok(true)
    }

    async fn handle_piece(&mut self, piece: &Piece) -> Result<bool, Box<dyn std::error::Error>> {
        if !self.is_piece_requested(piece) {
            self.stats.increment_unexpected_piece();
            return Ok(true);
        }

        let piece_rx = self.piece_rx.as_mut().ok_or(Error::PieceNotRequested)?;

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
            self.verify_piece_hash()?;
            self.save_piece_to_file().await?;
            return Ok(self.trigger_cmd_piece_finish(true).await?);
        } else {
            self.send_request().await?;
        }

        Ok(true)
    }

    fn is_piece_requested(&self, piece: &Piece) -> bool {
        match &self.piece_rx {
            Some(piece_rx) => {
                if piece_rx.piece_index != piece.piece_index() {
                    return false;
                }

                piece_rx
                    .requested
                    .iter()
                    .any(|(block_begin, block_length)| {
                        piece
                            .validate(piece_rx.piece_index, *block_begin, *block_length)
                            .is_ok()
                    })
            }
            None => false,
        }
    }

    async fn init_handshake(
        &mut self,
        peer_id: [u8; PEER_ID_SIZE],
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.connection
            .send_msg(&Handshake::new(&self.info_hash, &self.own_id))
            .await?;

        let (resp_tx, resp_rx) = oneshot::channel();
        self.peer_ch
            .send(PeerCmd::Init {
                addr: self.connection.addr.clone(),
                peer_id,
                resp_ch: resp_tx,
            })
            .await?;

        match resp_rx.await? {
            InitCmd::SendBitfield { bitfield } => self.connection.send_msg(&bitfield).await?,
        }

        Ok(())
    }

    async fn trigger_cmd_recv_choke(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.peer_ch
            .send(PeerCmd::RecvChoke {
                addr: self.connection.addr.clone(),
            })
            .await?;

        Ok(())
    }

    async fn trigger_cmd_recv_unchoke(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.peer_ch
            .send(PeerCmd::RecvUnchoke {
                addr: self.connection.addr.clone(),
                resp_ch: resp_tx,
            })
            .await?;

        match resp_rx.await? {
            UnchokeCmd::SendInterestedAndRequest(req_data) => {
                self.new_piece_request(true, &req_data).await?
            }
            UnchokeCmd::SendRequest(req_data) => self.new_piece_request(false, &req_data).await?,
            UnchokeCmd::SendNotInterested => {
                self.connection.send_msg(&NotInterested::new()).await?
            }
            UnchokeCmd::Ignore => (),
        }

        Ok(())
    }

    async fn trigger_cmd_recv_interested(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.peer_ch
            .send(PeerCmd::RecvInterested {
                addr: self.connection.addr.clone(),
            })
            .await?;

        Ok(())
    }

    async fn trigger_cmd_recv_not_interested(
        &mut self,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.peer_ch
            .send(PeerCmd::RecvNotInterested {
                addr: self.connection.addr.clone(),
                resp_ch: resp_tx,
            })
            .await?;

        match resp_rx.await? {
            NotInterestedCmd::PrepareKill => Ok(false),
            NotInterestedCmd::Ignore => Ok(true),
        }
    }

    async fn trigger_cmd_recv_have(
        &mut self,
        have: &Have,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.peer_ch
            .send(PeerCmd::RecvHave {
                addr: self.connection.addr.clone(),
                piece_index: have.piece_index(),
                resp_ch: resp_tx,
            })
            .await?;

        match resp_rx.await? {
            HaveCmd::SendInterestedAndRequest(req_data) => {
                self.new_piece_request(true, &req_data).await?
            }

            HaveCmd::SendInterested => self.connection.send_msg(&Interested::new()).await?,
            HaveCmd::Ignore => (),
        }

        Ok(())
    }

    async fn trigger_cmd_recv_bitfield(
        &mut self,
        bitfield: Bitfield,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.peer_ch
            .send(PeerCmd::RecvBitfield {
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

    async fn trigger_cmd_recv_request(
        &mut self,
        request: &Request,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.peer_ch
            .send(PeerCmd::RecvRequest {
                addr: self.connection.addr.clone(),
                piece_index: request.piece_index(),
                resp_ch: resp_tx,
            })
            .await?;

        match resp_rx.await? {
            RequestCmd::LoadAndSendPiece {
                piece_index,
                piece_hash,
            } => self.load_piece_from_file(piece_index, &piece_hash).await?,
            RequestCmd::Ignore => self.piece_tx = None,
        };

        Ok(())
    }

    async fn trigger_cmd_piece_finish(
        &mut self,
        done: bool,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = match done {
            true => PeerCmd::PieceDone {
                addr: self.connection.addr.clone(),
                resp_ch: resp_tx,
            },
            false => PeerCmd::PieceCancel {
                addr: self.connection.addr.clone(),
                resp_ch: resp_tx,
            },
        };

        self.peer_ch.send(cmd).await?;

        match resp_rx.await? {
            PieceCmd::SendRequest(req_data) => self.new_piece_request(false, &req_data).await?,
            PieceCmd::SendNotInterested => self.connection.send_msg(&NotInterested::new()).await?,
            PieceCmd::PrepareKill => return Ok(false),
            PieceCmd::Ignore => (),
        }

        Ok(true)
    }

    async fn trigger_cmd_sync_stats(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.peer_ch
            .send(PeerCmd::SyncStats {
                addr: self.connection.addr.clone(),
                downloaded_rate: self.stats.downloaded_rate(),
                uploaded_rate: self.stats.uploaded_rate(),
                unexpected_blocks: self.stats.unexpected_blocks(),
            })
            .await?;

        Ok(())
    }

    async fn new_piece_request(
        &mut self,
        interested: bool,
        req_data: &ReqData,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.piece_rx = Some(PieceRx::new(req_data));

        if interested {
            self.connection.send_msg(&Interested::new()).await?;
        }

        // BEP3 suggests to send more than one request to get better TCP performance (pipeline)
        self.send_request().await?;
        self.send_request().await?;
        Ok(())
    }

    async fn send_request(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(piece_rx) = self.piece_rx.as_mut() {
            if let Some((block_begin, block_len)) = piece_rx.left.pop_front() {
                piece_rx.requested.push_back((block_begin, block_len));
                let msg = Request::new(piece_rx.piece_index, block_begin, block_len);
                self.connection.send_msg(&msg).await?;
            }
        }

        Ok(())
    }

    async fn send_piece(&mut self, request: &Request) -> Result<(), Box<dyn std::error::Error>> {
        match &self.piece_tx {
            Some(piece_tx) => {
                let block_end = request.block_begin() + request.block_length();

                self.stats.update_uploaded(request.block_length());
                self.connection
                    .send_msg(&Piece::new(
                        request.piece_index(),
                        request.block_begin(),
                        piece_tx.buff[request.block_begin()..block_end].to_vec(),
                    ))
                    .await?;
                Ok(())
            }
            None => Err(Error::PieceNotLoaded.into()),
        }
    }

    fn verify_piece_hash(&self) -> Result<(), Error> {
        match self.piece_rx.as_ref() {
            Some(piece_rx) => {
                let mut hasher = sha1_smol::Sha1::new();
                hasher.update(piece_rx.buff.as_ref());

                match hasher.digest().bytes() == piece_rx.hash {
                    true => Ok(()),
                    false => Err(Error::PieceHashMismatch),
                }
            }
            None => Err(Error::PieceBuffMissing),
        }
    }

    async fn load_piece_from_file(
        &mut self,
        piece_index: usize,
        piece_hash: &[u8; HASH_SIZE],
    ) -> Result<(), Error> {
        let name = utils::hash_to_string(piece_hash) + ".piece";
        match fs::read(name).await {
            Ok(data) => {
                self.piece_tx = Some(PieceTx {
                    piece_index,
                    buff: data,
                });
                Ok(())
            }
            Err(_) => Err(Error::FileNotFound),
        }
    }

    async fn save_piece_to_file(&mut self) -> Result<(), Error> {
        let piece_rx = self
            .piece_rx
            .take()
            .ok_or(Error::PieceBuffMissing)
            .expect("Saving to file: piece data not exist after validation");
        let name = utils::hash_to_string(&piece_rx.hash) + ".piece";
        match fs::write(name, &piece_rx.buff).await {
            Ok(()) => Ok(()),
            Err(_) => Err(Error::FileCannotWrite),
        }
    }
}
