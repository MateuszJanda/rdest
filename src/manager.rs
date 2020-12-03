use crate::constant::HASH_SIZE;
use crate::frame::Bitfield;
use crate::handler::{
    BitfieldCmd, BroadCmd, Handler, HaveCmd, InitCmd, JobCmd, NotInterestedCmd, PieceDoneCmd,
    RequestCmd, UnchokeCmd,
};
use crate::progress::{Progress, ViewCmd};
use crate::tracker_client::TrackerCmd;
use crate::{utils, Error, Metainfo, TrackerClient};
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, Write};
use std::path::Path;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time;
use tokio::time::{Duration, Instant, Interval};

const JOB_CHANNEL_SIZE: usize = 64;
const BROADCAST_CHANNEL_SIZE: usize = 32;
const CHANGE_STATE_INTERVAL_SEC: u64 = 10;
const OPTIMISTIC_UNCHOKE_ROUND: u32 = 3;
const MAX_UNCHOKED: u32 = 3;

pub struct Manager {
    own_id: [u8; HASH_SIZE],
    pieces_status: Vec<Status>,
    peers: HashMap<String, Peer>,
    metainfo: Metainfo,
    candidates: Vec<(String, [u8; HASH_SIZE])>,
    view: Option<View>,
    change_round: u32,
    ttt: Option<JoinHandle<()>>,
    job_tx_ch: mpsc::Sender<JobCmd>,
    job_rx_ch: mpsc::Receiver<JobCmd>,
    tracker_tx_ch: mpsc::Sender<TrackerCmd>,
    tracker_rx_ch: mpsc::Receiver<TrackerCmd>,
    broad_ch: broadcast::Sender<BroadCmd>,
}

#[derive(Debug)]
struct Peer {
    pieces: Vec<bool>,
    job: Option<JoinHandle<()>>,
    index: Option<usize>,
    am_interested: bool,
    am_choked: bool,
    interested: bool,
    choked: bool,
    optimistic_unchoke: bool,
    download_rate: Option<u32>,
    uploaded_rate: Option<u32>,
    rejected_piece: u32,
}

#[derive(Debug)]
struct View {
    channel: mpsc::Sender<ViewCmd>,
    job: JoinHandle<()>,
}

#[derive(PartialEq, Clone, Debug)]
pub enum Status {
    Missing,
    Reserved,
    Have,
}

impl Peer {
    fn new(pieces_num: usize, job: JoinHandle<()>) -> Peer {
        Peer {
            pieces: vec![false; pieces_num],
            job: Some(job),
            index: None,
            am_interested: false,
            am_choked: true,
            interested: false,
            choked: true,
            optimistic_unchoke: false,
            download_rate: None,
            uploaded_rate: None,
            rejected_piece: 0,
        }
    }
}

impl Manager {
    pub fn new(metainfo: Metainfo, own_id: [u8; HASH_SIZE]) -> Manager {
        let (job_tx_ch, job_rx_ch) = mpsc::channel(JOB_CHANNEL_SIZE);
        let (tracker_tx_ch, tracker_rx_ch) = mpsc::channel(JOB_CHANNEL_SIZE);
        let (broad_ch, _) = broadcast::channel(BROADCAST_CHANNEL_SIZE);

        Manager {
            own_id,
            pieces_status: vec![Status::Missing; metainfo.pieces_num()],
            peers: HashMap::new(),
            metainfo,
            candidates: vec![],
            view: None,
            change_round: 0,
            ttt: None,
            job_tx_ch,
            job_rx_ch,
            tracker_tx_ch,
            tracker_rx_ch,
            broad_ch,
        }
    }

    pub async fn run(&mut self) {
        self.spawn_view();

        let mut t = TrackerClient::new(
            &self.own_id,
            self.metainfo.clone(),
            self.tracker_tx_ch.clone(),
        );
        self.ttt = Some(tokio::spawn(async move { t.run().await }));

        self.event_loop().await;
        // self.spawn_jobs();
    }

    fn spawn_view(&mut self) {
        let (mut view, channel) = Progress::new();
        self.view = Some(View {
            channel,
            job: tokio::spawn(async move { view.run().await }),
        });
    }

    fn spawn_jobs(&mut self) {
        let (addr, peer_id) = match self.candidates.pop() {
            Some(value) => value,
            None => return,
        };
        let addr222 = addr.clone();

        // TODO: spawn MAX_UNCHOKED + 1 + 1 jobs
        let own_id = self.own_id.clone();
        let info_hash = *self.metainfo.info_hash();
        let pieces_num = self.metainfo.pieces_num();
        let job_ch = self.job_tx_ch.clone();
        let broad_ch = self.broad_ch.subscribe();

        let job = tokio::spawn(async move {
            Handler::run(
                addr,
                own_id,
                Some(peer_id),
                info_hash,
                pieces_num,
                job_ch,
                broad_ch,
            )
            .await
        });

        let peer = Peer::new(self.metainfo.pieces_num(), job);
        self.peers.insert(addr222, peer);
    }

    async fn event_loop(&mut self) {
        let mut change_state_timer = self.start_change_conn_state_timer();

        // TODO: add listen
        // TODO: add tracker req
        // TODO: add file extractor job
        loop {
            tokio::select! {
                _ = change_state_timer.tick() => self.timeout_change_conn_state().expect("Can't change connection state"),
                Some(cmd) = self.tracker_rx_ch.recv() => {
                    println!("Tracker resp");
                    match cmd {
                        TrackerCmd::TrackerResp(resp) => {
                            self.candidates.extend_from_slice(&resp.peers());
                            for _ in 0..1 {
                                self.spawn_jobs();
                            }
                        }
                        TrackerCmd::Fail(_) => (),
                    }

                    // kill tracker client
                    match &mut self.ttt.take() {
                        Some(job) => {
                            job.await.expect("Can't kill tracker client");
                        }
                        _ => (),
                    }
                }
                Some(cmd) = self.job_rx_ch.recv() => {
                    if self.handle_job_cmd(cmd).await.expect("Can't handle command") == false {
                        break;
                    }
                }
            }
        }
    }

    fn start_change_conn_state_timer(&self) -> Interval {
        let start = Instant::now() + Duration::from_secs(CHANGE_STATE_INTERVAL_SEC);
        time::interval_at(start, Duration::from_secs(CHANGE_STATE_INTERVAL_SEC))
    }

    fn timeout_change_conn_state(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.change_round = (self.change_round + 1) % OPTIMISTIC_UNCHOKE_ROUND;

        // If not all peers reported their state, do nothing
        if self
            .peers
            .iter()
            .any(|(_, peer)| peer.download_rate.is_none() || peer.uploaded_rate.is_none())
        {
            return Ok(());
        }

        let new_opti = match self.change_round {
            0 => self.new_optimistic_peers()?,
            _ => vec![],
        };

        let mut rate = match self.is_seeder_mode() {
            true => self
                .peers
                .iter()
                .map(|(addr, peer)| (addr.clone(), peer.download_rate.unwrap()))
                .collect::<Vec<(String, u32)>>(),
            false => self
                .peers
                .iter()
                .map(|(addr, peer)| (addr.clone(), peer.uploaded_rate.unwrap()))
                .collect::<Vec<(String, u32)>>(),
        };

        let cmd = self.change_state_cmd(&mut rate, &new_opti)?;
        let _ = self.broad_ch.send(cmd);

        Ok(())
    }

    fn is_seeder_mode(&self) -> bool {
        self.pieces_status
            .iter()
            .all(|status| *status == Status::Have)
    }

    fn new_optimistic_peers(&mut self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let all_choked = self
            .peers
            .iter()
            .filter(|(_, peer)| peer.am_choked && peer.interested)
            .map(|(addr, _)| addr.clone())
            .collect::<Vec<String>>();

        match all_choked.choose(&mut rand::thread_rng()) {
            Some(addr) => Ok(vec![addr.clone()]),
            None => Ok(vec![]),
        }
    }

    fn change_state_cmd(
        &mut self,
        rates: &mut Vec<(String, u32)>,
        new_opti: &Vec<String>,
    ) -> Result<BroadCmd, Box<dyn std::error::Error>> {
        // Downloaded/uploaded rate in descending order
        rates.sort_by(|(_, r1), (_, r2)| r2.cmp(&r1));

        let mut am_choked_map: HashMap<String, bool> = HashMap::new();
        let mut count = 0;
        for (addr, _) in rates.iter() {
            let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
            if count < MAX_UNCHOKED {
                if peer.am_choked && peer.interested && !new_opti.contains(addr) {
                    // Choked state changed to Unchoked
                    peer.am_choked = false;
                    am_choked_map.insert(addr.clone(), false);
                    count += 1;
                } else if !peer.am_choked && peer.interested {
                    // Unchoked state doesn't change
                    count += 1;
                } else if !peer.am_choked && !peer.interested {
                    // Choke, because peer is not interested
                    peer.am_choked = true;
                    am_choked_map.insert(addr.clone(), true);
                }
            } else if !peer.am_choked {
                // Limit reached so change all rest peers states from Unchoked to Choked
                peer.am_choked = true;
                am_choked_map.insert(addr.clone(), true);
            }

            // If some new optimistic then disable "optimistic unchoke"
            if !new_opti.is_empty() {
                peer.optimistic_unchoke = false;
            }
        }

        // Set new optimistic unchoke
        for addr in new_opti.iter() {
            let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
            peer.am_choked = false;
            am_choked_map.insert(addr.clone(), false);
            peer.optimistic_unchoke = true;
        }

        Ok(BroadCmd::SendOwnState { am_choked_map })
    }

    async fn handle_job_cmd(&mut self, cmd: JobCmd) -> Result<bool, Error> {
        match cmd {
            JobCmd::Init { addr, resp_ch } => self.handle_init(&addr, resp_ch).await,
            JobCmd::RecvChoke { addr } => self.handle_choke(&addr),
            JobCmd::RecvUnchoke { addr, resp_ch } => self.handle_unchoke(&addr, resp_ch),
            JobCmd::RecvInterested { addr } => self.handle_interested(&addr),
            JobCmd::RecvNotInterested { addr, resp_ch } => {
                self.handle_not_interested(&addr, resp_ch)
            }
            JobCmd::RecvHave {
                addr,
                index,
                resp_ch,
            } => self.handle_have(&addr, index, resp_ch),
            JobCmd::RecvBitfield {
                addr,
                bitfield,
                resp_ch,
            } => self.handle_bitfield(&addr, &bitfield, resp_ch),
            JobCmd::RecvRequest {
                addr,
                index,
                block_begin,
                block_length,
                resp_ch,
            } => self.handle_request(&addr, index, block_begin, block_length, resp_ch),
            JobCmd::PieceDone { addr, resp_ch } => self.handle_piece_done(&addr, resp_ch),
            JobCmd::SyncStats {
                addr,
                downloaded_rate,
                uploaded_rate,
                rejected_piece,
            } => self.handle_sync_stats(&addr, &downloaded_rate, &uploaded_rate, rejected_piece),
            JobCmd::KillReq {
                addr,
                index,
                reason,
            } => self.handle_kill_req(&addr, &index, &reason).await,
        }
    }

    async fn handle_init(
        &mut self,
        addr: &String,
        resp_ch: oneshot::Sender<InitCmd>,
    ) -> Result<bool, Error> {
        let bitfield = Bitfield::from_vec(
            &self
                .pieces_status
                .iter()
                .map(|status| *status == Status::Have)
                .collect(),
        );

        let _ = resp_ch.send(InitCmd::SendBitfield { bitfield });
        self.send_log(&format!("Handshake with peer: {}", addr))
            .await;

        Ok(true)
    }

    fn handle_choke(&mut self, addr: &String) -> Result<bool, Error> {
        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        peer.choked = true;

        match peer.index {
            Some(index) if self.pieces_status[index] == Status::Reserved => {
                self.pieces_status[index] = Status::Missing
            }
            _ => (),
        }
        Ok(true)
    }

    fn handle_unchoke(
        &mut self,
        addr: &String,
        resp_ch: oneshot::Sender<UnchokeCmd>,
    ) -> Result<bool, Error> {
        let pieces = &self.peers[addr].pieces;
        let index = self.choose_piece(pieces);

        let peer = self.peers.get(addr).ok_or(Error::PeerNotFound)?;
        let cmd = match index {
            Some(index) if !peer.am_interested => {
                self.pieces_status[index] = Status::Reserved;
                UnchokeCmd::SendInterestedAndRequest {
                    index,
                    piece_length: self.metainfo.piece_length(index),
                    piece_hash: *self.metainfo.piece(index),
                }
            }
            Some(index) => {
                self.pieces_status[index] = Status::Reserved;
                UnchokeCmd::SendRequest {
                    index,
                    piece_length: self.metainfo.piece_length(index),
                    piece_hash: *self.metainfo.piece(index),
                }
            }
            None if peer.am_interested => UnchokeCmd::SendNotInterested,
            None => UnchokeCmd::Ignore,
        };

        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        peer.choked = false;
        peer.index = index;
        peer.am_interested = index.is_some();

        let _ = &resp_ch.send(cmd);
        Ok(true)
    }

    fn handle_interested(&mut self, addr: &String) -> Result<bool, Error> {
        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        peer.interested = true;
        Ok(true)
    }

    fn handle_not_interested(
        &mut self,
        addr: &String,
        resp_ch: oneshot::Sender<NotInterestedCmd>,
    ) -> Result<bool, Error> {
        {
            let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
            peer.interested = false;
        }

        let peer = self.peers.get(addr).ok_or(Error::PeerNotFound)?;
        let index = self.choose_piece(&peer.pieces);
        let cmd = if !peer.am_interested && peer.index.is_none() && index.is_none() {
            NotInterestedCmd::PrepareKill
        } else {
            NotInterestedCmd::Ignore
        };

        let _ = resp_ch.send(cmd);

        Ok(true)
    }

    fn handle_have(
        &mut self,
        addr: &String,
        index: usize,
        resp_ch: oneshot::Sender<HaveCmd>,
    ) -> Result<bool, Error> {
        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        peer.pieces[index] = true;

        let cmd = if self.pieces_status[index] == Status::Missing && !peer.am_interested {
            if !peer.choked && peer.index.is_none() {
                self.pieces_status[index] = Status::Reserved;
                peer.index = Some(index);
                peer.am_interested = true;
                HaveCmd::SendInterestedAndRequest {
                    index,
                    piece_length: self.metainfo.piece_length(index),
                    piece_hash: *self.metainfo.piece(index),
                }
            } else {
                peer.am_interested = true;
                HaveCmd::SendInterested
            }
        } else {
            HaveCmd::Ignore
        };

        let _ = resp_ch.send(cmd);

        Ok(true)
    }

    // Sending NotInterested explicitly (this is default state) is mandatory according BEP3, but
    // Interested should be send only after Unchoke. It appears that many clients unfortunately
    // wait for this message (doesn't send Unchoke and send KeepAlive instead).
    fn handle_bitfield(
        &mut self,
        addr: &String,
        bitfield: &Bitfield,
        resp_ch: oneshot::Sender<BitfieldCmd>,
    ) -> Result<bool, Error> {
        // Update peer pieces bitfield
        {
            let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
            peer.pieces.copy_from_slice(&bitfield.to_vec());
        }

        // BEP3 "whenever a downloader doesn't have something they currently would ask a peer for
        // in unchoked, they must express lack of interest, despite being choked"
        let index = self.choose_piece(&bitfield.to_vec());
        let am_interested = match index {
            Some(_) => true,
            None => false,
        };

        // Change to unchoked or not
        let peer = self.peers.get(addr).ok_or(Error::PeerNotFound)?;
        let with_am_unchoked = if self.unchoked_num() < MAX_UNCHOKED && peer.am_choked {
            true
        } else {
            false
        };

        // Update own state
        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        peer.am_interested = am_interested;
        if with_am_unchoked {
            peer.am_choked = true;
        }

        let cmd = BitfieldCmd::SendState {
            with_am_unchoked,
            am_interested,
        };

        let _ = &resp_ch.send(cmd);

        Ok(true)
    }

    fn unchoked_num(&self) -> u32 {
        self.peers
            .iter()
            .filter(|(_, peer)| peer.am_choked == false && peer.optimistic_unchoke == true)
            .count() as u32
    }

    fn handle_request(
        &mut self,
        addr: &String,
        index: usize,
        block_begin: usize,
        block_length: usize,
        resp_ch: oneshot::Sender<RequestCmd>,
    ) -> Result<bool, Error> {
        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        if peer.am_choked {
            let _ = resp_ch.send(RequestCmd::Ignore);
            return Ok(true);
        }

        if index >= self.metainfo.pieces_num() {
            let _ = resp_ch.send(RequestCmd::Ignore);
            return Ok(true);
        }

        if block_begin + block_length >= self.metainfo.piece_length(index) {
            let _ = resp_ch.send(RequestCmd::Ignore);
            return Ok(true);
        }

        if self.pieces_status[index] != Status::Have {
            let _ = resp_ch.send(RequestCmd::Ignore);
            return Ok(true);
        }

        let _ = resp_ch.send(RequestCmd::LoadAndSendPiece {
            index,
            block_begin,
            block_length,
            piece_hash: *self.metainfo.piece(index),
        });

        Ok(true)
    }

    fn handle_piece_done(
        &mut self,
        addr: &String,
        resp_ch: oneshot::Sender<PieceDoneCmd>,
    ) -> Result<bool, Error> {
        match self.peers.get(addr).ok_or(Error::PeerNotFound)?.index {
            Some(index) => {
                self.pieces_status[index] = Status::Have;
                let _ = self.broad_ch.send(BroadCmd::SendHave { index });
            }
            None => panic!("Piece downloaded but not requested"),
        }

        let pieces = &self.peers[addr].pieces;
        let index = self.choose_piece(pieces);

        let cmd = match index {
            Some(index) => {
                println!("Some index {:?}", index);
                let peer = self.peers.get(addr).ok_or(Error::PeerNotFound)?;
                if !peer.choked {
                    PieceDoneCmd::SendRequest {
                        index,
                        piece_length: self.metainfo.piece_length(index),
                        piece_hash: *self.metainfo.piece(index),
                    }
                } else {
                    PieceDoneCmd::Ignore
                }
            }
            None => {
                let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
                peer.am_interested = false;
                if peer.interested {
                    PieceDoneCmd::SendNotInterested
                } else {
                    PieceDoneCmd::PrepareKill
                }
            }
        };

        let _ = resp_ch.send(cmd);
        Ok(true)
    }

    fn handle_sync_stats(
        &mut self,
        addr: &String,
        downloaded_rate: &Option<u32>,
        uploaded_rate: &Option<u32>,
        rejected_piece: u32,
    ) -> Result<bool, Error> {
        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        peer.download_rate = *downloaded_rate;
        peer.uploaded_rate = *uploaded_rate;
        // TODO: log this
        peer.rejected_piece = rejected_piece;
        Ok(true)
    }

    async fn handle_kill_req(
        &mut self,
        addr: &String,
        index: &Option<usize>,
        reason: &String,
    ) -> Result<bool, Error> {
        println!("Kill reason: {}", reason);
        self.kill_job(&addr, &index).await;

        if self.peers.is_empty() {
            self.kill_view().await;
            if let Err(_) = self.extract_files() {
                ()
            }
            return Ok(false);
        }

        Ok(true)
    }

    fn choose_piece(&self, pieces: &Vec<bool>) -> Option<usize> {
        // Count how many peers have specific piece
        let mut vec: Vec<u32> = vec![0; self.metainfo.pieces_num()];
        for (_, peer) in self.peers.iter() {
            for (index, have) in peer.pieces.iter().enumerate() {
                if *have {
                    vec[index] += 1;
                }
            }
        }

        // Create pair (index, count) for missing pieces
        let mut rarest: Vec<(usize, u32)> = vec
            .iter()
            .enumerate()
            .filter(|(index, _)| self.pieces_status[*index] == Status::Missing)
            .map(|(index, count)| (index, *count))
            .collect();

        // Shuffle to get better distribution of pieces from peers
        rarest.shuffle(&mut rand::thread_rng());

        // Sort by rarest
        rarest.sort_by(|(_, count1), (_, count2)| count1.cmp(&count2));

        for (index, count) in rarest.iter() {
            if count > &0 && pieces[*index] == true {
                return Some(*index);
            }
        }

        None
    }

    async fn kill_job(&mut self, addr: &String, index: &Option<usize>) {
        // Reset piece status
        match index {
            Some(index) if self.pieces_status[*index] != Status::Have => {
                self.pieces_status[*index] = Status::Missing
            }
            _ => (),
        }

        // Wait fot the task to finish
        match self.peers.get_mut(addr) {
            Some(peer) => match peer.job.take() {
                Some(job) => job.await.expect("Can't kill job"),
                None => (),
            },
            None => (),
        }

        // Remove peer data from map
        self.peers.remove(addr);

        println!("Job killed");
    }

    async fn kill_view(&mut self) {
        match &mut self.view.take() {
            Some(view) => {
                let _ = view.channel.send(ViewCmd::Kill).await;
                let job = &mut view.job;
                job.await.expect("Can't kill view");
            }
            _ => (),
        }
    }

    async fn send_log(&mut self, text: &String) {
        match &mut self.view {
            Some(view) => {
                let _ = view.channel.send(ViewCmd::Log(text.clone())).await;
            }
            _ => (),
        }
    }

    fn extract_files(&self) -> Result<(), Box<dyn std::error::Error>> {
        for (path, start, end) in self.metainfo.file_piece_ranges().iter() {
            // Create directories if needed
            fs::create_dir_all(Path::new(path).parent().unwrap())?;

            // Create output file
            let mut writer = BufWriter::new(File::create(path)?);

            // Write pieces/chunks
            for index in start.file_index..end.file_index {
                let name = utils::hash_to_string(&self.metainfo.piece(index)) + ".piece";
                let reader = &mut BufReader::new(File::open(name)?);

                if index == start.file_index {
                    reader.seek(std::io::SeekFrom::Start(start.byte_index as u64))?;
                }

                let mut buffer = vec![];
                reader.read_to_end(&mut buffer)?;
                writer.write_all(buffer.as_slice())?;
            }

            // Write last chunk
            let name = utils::hash_to_string(&self.metainfo.piece(end.file_index)) + ".piece";
            let reader = &mut BufReader::new(File::open(name)?);

            let mut buffer = vec![0; end.byte_index];
            reader.read_exact(buffer.as_mut_slice())?;
            writer.write_all(buffer.as_slice())?;
        }

        Ok(())
    }
}
