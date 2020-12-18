use crate::commands::{
    BitfieldCmd, BroadCmd, ExtractorCmd, HaveCmd, InitCmd, NotInterestedCmd, PeerCmd, PieceDoneCmd,
    ReqData, RequestCmd, TrackerCmd, UnchokeCmd, ViewCmd,
};
use crate::constants::{PEER_ID_SIZE, PORT};
use crate::extractor::Extractor;
use crate::messages::bitfield::Bitfield;
use crate::peer_handler::PeerHandler;
use crate::progress_view::ProgressView;
use crate::{Error, Metainfo, TrackerClient};
use rand::seq::SliceRandom;
use std::cmp::max;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time;
use tokio::time::{Duration, Instant, Interval};

const CHANNEL_SIZE: usize = 64;
const BROADCAST_CHANNEL_SIZE: usize = 32;
const CHANGE_STATE_INTERVAL_SEC: u64 = 10;
const MAX_NOT_INTERESTED: usize = 4;
const MAX_OPTIMISTIC_ROUNDS: u32 = 3;
const MAX_OPTIMISTIC: u32 = 1;
const MAX_UNCHOKED: u32 = 10;

/// Peer manager.
pub struct PeerManager {
    own_id: [u8; PEER_ID_SIZE],
    pieces_status: Vec<Status>,
    peers: HashMap<String, Peer>,
    general_channels: GeneralChannels,
    metainfo: Metainfo,
    candidates: Vec<(String, [u8; PEER_ID_SIZE])>,
    view: Option<View>,
    round: u32,
    tracker: Job<TrackerCmd>,
    extractor: Job<ExtractorCmd>,
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
}

#[derive(Debug)]
struct View {
    channel: mpsc::Sender<ViewCmd>,
    job: JoinHandle<()>,
}

#[derive(Debug)]
struct Job<Cmd> {
    job: Option<JoinHandle<()>>,
    tx_ch: mpsc::Sender<Cmd>,
    rx_ch: mpsc::Receiver<Cmd>,
}

#[derive(Debug)]
struct GeneralChannels {
    tx: mpsc::Sender<PeerCmd>,
    rx: mpsc::Receiver<PeerCmd>,
    broad: broadcast::Sender<BroadCmd>,
}

#[derive(PartialEq, Clone, Debug)]
pub enum Status {
    Missing,
    Reserved,
    Have,
}

impl<Cmd> Job<Cmd> {
    fn new(tx_ch: mpsc::Sender<Cmd>, rx_ch: mpsc::Receiver<Cmd>) -> Job<Cmd> {
        Job {
            job: None,
            tx_ch,
            rx_ch,
        }
    }
}

impl GeneralChannels {
    fn new(
        tx: mpsc::Sender<PeerCmd>,
        rx: mpsc::Receiver<PeerCmd>,
        broad: broadcast::Sender<BroadCmd>,
    ) -> GeneralChannels {
        GeneralChannels { tx, rx, broad }
    }
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
        }
    }
}

impl PeerManager {
    /// Create new instance managing peer-to-peer connection. Currently most settings, are set as
    /// constatns.
    ///
    /// # Example
    /// ```no_run
    /// use rdest::{Metainfo, PeerManager};
    /// use std::path::PathBuf;
    ///
    /// let path = PathBuf::from("ubuntu-20.04.1-desktop-amd64.iso.torrent");
    /// let torrent_file = Metainfo::from_file(path.as_path()).unwrap();
    /// let peer_id = b"AAAAABBBBBCCCCCDDDDD";
    ///
    /// let mut manager = PeerManager::new(torrent_file, *peer_id);
    /// ```
    pub fn new(metainfo: Metainfo, own_id: [u8; PEER_ID_SIZE]) -> PeerManager {
        let (peer_tx, peer_rx) = mpsc::channel(CHANNEL_SIZE);
        let (tracker_tx, tracker_rx) = mpsc::channel(CHANNEL_SIZE);
        let (extractor_tx, extractor_rx) = mpsc::channel(CHANNEL_SIZE);
        let (broad, _) = broadcast::channel(BROADCAST_CHANNEL_SIZE);

        PeerManager {
            own_id,
            pieces_status: vec![Status::Missing; metainfo.pieces_num()],
            peers: HashMap::new(),
            general_channels: GeneralChannels::new(peer_tx, peer_rx, broad),
            metainfo,
            candidates: vec![],
            view: None,
            round: 0,
            tracker: Job::new(tracker_tx, tracker_rx),
            extractor: Job::new(extractor_tx, extractor_rx),
        }
    }

    /// Run PeerManager who will try connect to tracker, get list of available peers, and establish
    /// connection with them
    ///
    /// # Example
    /// ```no_run
    /// use rdest::{Metainfo, PeerManager};
    /// use std::path::Path;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let path = Path::new("ubuntu-20.04.1-desktop-amd64.iso.torrent");
    /// let torrent_file = Metainfo::from_file(path).unwrap();
    /// let peer_id = b"AAAAABBBBBCCCCCDDDDD";
    ///
    /// let mut manager = PeerManager::new(torrent_file, *peer_id);
    /// manager.run().await;
    /// # }
    /// ```
    pub async fn run(&mut self) {
        self.spawn_view();
        self.spawn_tracker();
        self.event_loop().await;
    }

    async fn event_loop(&mut self) {
        let mut listener = TcpListener::bind((Ipv4Addr::new(0, 0, 0, 0), PORT))
            .await
            .expect(&format!("Can't bind to port {}", PORT));

        let mut change_state_timer = self.start_change_conn_state_timer();

        loop {
            tokio::select! {
                _ = change_state_timer.tick() => self.timeout_change_conn_state().expect("Can't change connection state"),
                Ok((socket, _)) = listener.accept() => self.spawn_peer_listener(socket),
                Some(cmd) = self.tracker.rx_ch.recv() => self.handle_tracker_cmd(cmd).await,
                Some(cmd) = self.extractor.rx_ch.recv() => self.handle_extractor_cmd(cmd).await,
                Some(cmd) = self.general_channels.rx.recv() => {
                    if self.handle_peer_cmd(cmd).await.expect("Can't handle command") == false {
                        self.kill_view().await;
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
        self.round = (self.round + 1) % MAX_OPTIMISTIC_ROUNDS;

        // If not all peers reported their state, do nothing
        if self
            .peers
            .iter()
            .any(|(_, peer)| peer.download_rate.is_none() || peer.uploaded_rate.is_none())
        {
            return Ok(());
        }

        let new_optimistic = match self.round {
            0 => self.new_optimistic_peers()?,
            _ => vec![],
        };

        let is_seeder = self
            .pieces_status
            .iter()
            .all(|status| *status == Status::Have);

        let make_pair = match is_seeder {
            true => |(addr, peer): (&String, &Peer)| (addr.clone(), peer.download_rate.unwrap()),
            false => |(addr, peer): (&String, &Peer)| (addr.clone(), peer.uploaded_rate.unwrap()),
        };

        let mut rate = self
            .peers
            .iter()
            .map(|param| make_pair(param))
            .collect::<Vec<(String, u32)>>();

        let cmd = self.change_state_cmd(&mut rate, &new_optimistic)?;
        let _ = self.general_channels.broad.send(cmd);
        Ok(())
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
        new_optimistic: &Vec<String>,
    ) -> Result<BroadCmd, Box<dyn std::error::Error>> {
        // Downloaded/uploaded rate in descending order
        rates.sort_by(|(_, r1), (_, r2)| r2.cmp(&r1));

        let mut am_choked_map: HashMap<String, bool> = HashMap::new();
        let mut count = 0;
        for (addr, _) in rates.iter() {
            let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
            if count < MAX_UNCHOKED {
                // Choked state changed to Unchoked
                if peer.am_choked && peer.interested && !new_optimistic.contains(addr) {
                    peer.am_choked = false;
                    am_choked_map.insert(addr.clone(), false);
                    count += 1;
                // Unchoked state doesn't change
                } else if !peer.am_choked && peer.interested {
                    count += 1;
                // Choke, because peer is not interested
                } else if !peer.am_choked && !peer.interested {
                    peer.am_choked = true;
                    am_choked_map.insert(addr.clone(), true);
                }
            // Limit reached so change all rest peers states from Unchoked to Choked
            } else if !peer.am_choked {
                peer.am_choked = true;
                am_choked_map.insert(addr.clone(), true);
            }

            // If some new optimistic then disable "optimistic unchoke"
            if !new_optimistic.is_empty() {
                peer.optimistic_unchoke = false;
            }
        }

        // Set new optimistic unchoke
        for addr in new_optimistic.iter() {
            let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
            peer.am_choked = false;
            am_choked_map.insert(addr.clone(), false);
            peer.optimistic_unchoke = true;
        }

        Ok(BroadCmd::SendOwnState { am_choked_map })
    }

    async fn handle_tracker_cmd(&mut self, cmd: TrackerCmd) {
        match cmd {
            TrackerCmd::TrackerResp(resp) => {
                let peers = resp.peers();
                self.log(format!("Ok, got {} peers from tracker", peers.len()))
                    .await;
                self.candidates.extend_from_slice(&peers);

                let all_am_interested = self
                    .peers
                    .iter()
                    .filter(|(_, peer)| peer.am_interested)
                    .count() as i32;
                let spawn_num = (MAX_UNCHOKED + MAX_OPTIMISTIC) as i32 - all_am_interested;
                // let spawn_num = 1;

                for _ in 0..max(0, spawn_num) {
                    self.spawn_peer_handler();
                }
            }
            TrackerCmd::Fail(e) => self.log(format!("Tracker fail: {}", e)).await,
        }
        self.kill_tracker().await;
    }

    async fn handle_extractor_cmd(&mut self, cmd: ExtractorCmd) {
        match cmd {
            ExtractorCmd::Done => self.log("Extractor done".to_string()).await,
            ExtractorCmd::Fail(e) => self.log("Extractor fail: ".to_string() + &e).await,
        }

        self.kill_extractor().await;
    }

    async fn handle_peer_cmd(&mut self, cmd: PeerCmd) -> Result<bool, Error> {
        match cmd {
            PeerCmd::Init { addr, resp_ch } => self.handle_init(&addr, resp_ch).await,
            PeerCmd::RecvChoke { addr } => self.handle_choke(&addr),
            PeerCmd::RecvUnchoke { addr, resp_ch } => self.handle_unchoke(&addr, resp_ch),
            PeerCmd::RecvInterested { addr } => self.handle_interested(&addr),
            PeerCmd::RecvNotInterested { addr, resp_ch } => {
                self.handle_not_interested(&addr, resp_ch)
            }
            PeerCmd::RecvHave {
                addr,
                index,
                resp_ch,
            } => self.handle_have(&addr, index, resp_ch),
            PeerCmd::RecvBitfield {
                addr,
                bitfield,
                resp_ch,
            } => self.handle_bitfield(&addr, &bitfield, resp_ch).await,
            PeerCmd::RecvRequest {
                addr,
                index,
                resp_ch,
            } => self.handle_request(&addr, index, resp_ch),
            PeerCmd::PieceDone { addr, resp_ch } => self.handle_piece_done(&addr, resp_ch).await,
            PeerCmd::SyncStats {
                addr,
                downloaded_rate,
                uploaded_rate,
            } => self.handle_sync_stats(&addr, &downloaded_rate, &uploaded_rate),
            PeerCmd::KillReq { addr, reason } => self.handle_kill_req(&addr, &reason).await,
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
        self.peer_log(addr, "Handshake with peer".to_string()).await;

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
            Some(index) => {
                self.pieces_status[index] = Status::Reserved;
                match peer.am_interested {
                    true => UnchokeCmd::SendRequest(self.req_data(index)),
                    false => UnchokeCmd::SendInterestedAndRequest(self.req_data(index)),
                }
            }
            None => match peer.am_interested {
                true => UnchokeCmd::SendNotInterested,
                false => UnchokeCmd::Ignore,
            },
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
        let cmd = match !peer.am_interested && peer.index.is_none() && index.is_none() {
            true => NotInterestedCmd::PrepareKill,
            false => NotInterestedCmd::Ignore,
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
                HaveCmd::SendInterestedAndRequest(self.req_data(index))
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

    async fn handle_bitfield(
        &mut self,
        addr: &String,
        bitfield: &Bitfield,
        resp_ch: oneshot::Sender<BitfieldCmd>,
    ) -> Result<bool, Error> {
        self.peer_log(addr, format!("Received a bitfield")).await;
        // Update peer pieces bitfield
        {
            let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
            peer.pieces.copy_from_slice(&bitfield.to_vec());
        }

        // BEP3 says "whenever a downloader doesn't have something they currently would ask a peer
        // for in unchoked, they must express lack of interest, despite being choked"
        //
        // Sending NotInterested explicitly (this is default state) is mandatory according BEP3, but
        // Interested should be send only after Unchoke. It appears, unfortunately, that many
        // clients wait for this message (doesn't send Unchoke and send KeepAlive instead).
        let index = self.choose_piece(&bitfield.to_vec());
        let am_interested = match index {
            Some(_) => true,
            None => false,
        };

        // Change to unchoked or not
        let peer = self.peers.get(addr).ok_or(Error::PeerNotFound)?;
        let with_am_unchoked = self.unchoked_num() < MAX_UNCHOKED && peer.am_choked;

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

    fn handle_request(
        &mut self,
        addr: &String,
        index: usize,
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

        if self.pieces_status[index] != Status::Have {
            let _ = resp_ch.send(RequestCmd::Ignore);
            return Ok(true);
        }

        let _ = resp_ch.send(RequestCmd::LoadAndSendPiece {
            index,
            piece_hash: *self.metainfo.piece(index),
        });

        Ok(true)
    }

    async fn handle_piece_done(
        &mut self,
        addr: &String,
        resp_ch: oneshot::Sender<PieceDoneCmd>,
    ) -> Result<bool, Error> {
        match self.peers.get(addr).ok_or(Error::PeerNotFound)?.index {
            Some(index) => {
                self.peer_log(addr, format!("New piece downloaded: {}", index))
                    .await;
                self.pieces_status[index] = Status::Have;
                let _ = self
                    .general_channels
                    .broad
                    .send(BroadCmd::SendHave { index });
            }
            None => panic!("Piece downloaded but not requested"),
        }

        let pieces = &self.peers[addr].pieces;
        let index = self.choose_piece(pieces);

        let cmd = match index {
            Some(index) => {
                let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
                self.pieces_status[index] = Status::Reserved;
                peer.index = Some(index);
                match peer.choked {
                    true => PieceDoneCmd::Ignore,
                    false => PieceDoneCmd::SendRequest(self.req_data(index)),
                }
            }
            None => {
                let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
                peer.index = None;
                peer.am_interested = false;
                match peer.interested {
                    true => PieceDoneCmd::SendNotInterested,
                    false => PieceDoneCmd::PrepareKill,
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
    ) -> Result<bool, Error> {
        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        peer.download_rate = *downloaded_rate;
        peer.uploaded_rate = *uploaded_rate;
        Ok(true)
    }

    async fn handle_kill_req(&mut self, addr: &String, reason: &String) -> Result<bool, Error> {
        self.peer_log(addr, "Killed, reason: ".to_string() + reason)
            .await;
        self.kill_peer(&addr).await;

        let all = self
            .pieces_status
            .iter()
            .all(|status| *status == Status::Have);

        if all {
            self.spawn_extractor();
        } else if self.candidates.is_empty() {
            self.spawn_tracker();
        } else {
            self.spawn_peer_handler();
        }

        Ok(true)
    }

    fn unchoked_num(&self) -> u32 {
        self.peers
            .iter()
            .filter(|(_, peer)| peer.am_choked == false && peer.optimistic_unchoke == true)
            .count() as u32
    }

    fn req_data(&self, index: usize) -> ReqData {
        ReqData {
            index,
            piece_length: self.metainfo.piece_length(index),
            piece_hash: *self.metainfo.piece(index),
        }
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

    fn spawn_view(&mut self) {
        let broad_ch = self.general_channels.broad.subscribe();
        let (mut view, channel) = ProgressView::new(self.pieces_status.len(), broad_ch);
        self.view = Some(View {
            channel,
            job: tokio::spawn(async move { view.run().await }),
        });
    }

    fn spawn_tracker(&mut self) {
        let mut tracker = TrackerClient::new(
            &self.own_id,
            self.metainfo.clone(),
            self.tracker.tx_ch.clone(),
        );
        self.tracker.job = Some(tokio::spawn(async move { tracker.run().await }));
    }

    fn spawn_extractor(&mut self) {
        let mut extractor = Extractor::new(self.metainfo.clone(), self.extractor.tx_ch.clone());
        self.extractor.job = Some(tokio::spawn(async move { extractor.run().await }));
    }

    fn spawn_peer_handler(&mut self) {
        let (addr, peer_id) = match self.candidates.pop() {
            Some(value) => value,
            None => return,
        };

        if self.peers.contains_key(&addr) {
            return;
        }

        let peer_addr = addr.clone();
        let own_id = self.own_id.clone();
        let info_hash = *self.metainfo.info_hash();
        let pieces_num = self.metainfo.pieces_num();
        let peer_ch = self.general_channels.tx.clone();
        let broad_ch = self.general_channels.broad.subscribe();

        let job = tokio::spawn(async move {
            PeerHandler::run_incoming(
                addr,
                own_id,
                Some(peer_id),
                info_hash,
                pieces_num,
                peer_ch,
                broad_ch,
            )
            .await
        });

        let peer = Peer::new(self.metainfo.pieces_num(), job);
        self.peers.insert(peer_addr, peer);
    }

    fn spawn_peer_listener(&mut self, socket: TcpStream) {
        let am_not_interested = self
            .peers
            .iter()
            .filter(|(_, peer)| !peer.am_interested)
            .count();
        if am_not_interested >= MAX_NOT_INTERESTED {
            return;
        }

        let addr = match socket.peer_addr() {
            Ok(addr) => addr.to_string(),
            Err(_) => return,
        };

        let peer_addr = addr.clone();
        let own_id = self.own_id.clone();
        let info_hash = *self.metainfo.info_hash();
        let pieces_num = self.metainfo.pieces_num();
        let job_ch = self.general_channels.tx.clone();
        let broad_ch = self.general_channels.broad.subscribe();

        let job = tokio::spawn(async move {
            PeerHandler::run_outgoing(
                socket, addr, own_id, None, info_hash, pieces_num, job_ch, broad_ch,
            )
            .await
        });

        let peer = Peer::new(self.metainfo.pieces_num(), job);
        self.peers.insert(peer_addr, peer);
    }

    async fn kill_view(&mut self) {
        match &mut self.view.take() {
            Some(view) => {
                let _ = view.channel.send(ViewCmd::Kill).await;
                let job = &mut view.job;
                job.await.expect("Can't kill view");
            }
            None => (),
        }
    }

    async fn kill_peer(&mut self, addr: &String) {
        match self.peers.get_mut(addr) {
            Some(peer) => {
                // Reset piece status
                if let Some(index) = peer.index {
                    if self.pieces_status[index] != Status::Have {
                        self.pieces_status[index] = Status::Missing
                    }
                }

                // Wait for task to finish
                if let Some(job) = peer.job.take() {
                    job.await.expect("Can't kill peer job")
                }
            }
            None => (),
        }

        // Remove peer data from map
        self.peers.remove(addr);
    }

    async fn kill_tracker(&mut self) {
        match &mut self.tracker.job.take() {
            Some(job) => job.await.expect("Can't kill tracker"),
            None => (),
        }
    }

    async fn kill_extractor(&mut self) {
        match &mut self.extractor.job.take() {
            Some(job) => job.await.expect("Can't kill extractor"),
            None => (),
        }
    }

    async fn log(&mut self, text: String) {
        if let Some(view) = &mut self.view {
            let _ = view.channel.send(ViewCmd::Log(text)).await;
        }
    }

    async fn peer_log(&mut self, addr: &String, text: String) {
        if let Some(view) = &mut self.view {
            let line = format!("[{}] {}", addr, text);
            let _ = view.channel.send(ViewCmd::Log(line)).await;
        }
    }
}
