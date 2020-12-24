use crate::commands::{
    BitfieldCmd, BroadCmd, ExtractorCmd, HaveCmd, InitCmd, NotInterestedCmd, PeerCmd, PieceCmd,
    RequestCmd, TrackerCmd, UnchokeCmd, ViewCmd,
};
use crate::constants::{
    MAX_NOT_INTERESTED, MAX_OPTIMISTIC, MAX_OPTIMISTIC_ROUNDS, MAX_UNCHOKED, PEER_ID_SIZE, PORT,
};
use crate::extractor::Extractor;
use crate::messages::Bitfield;
use crate::peer::Peer;
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

const END_GAME_LIMIT: usize = 10;
const CHANNEL_SIZE: usize = 64;
const BROADCAST_CHANNEL_SIZE: usize = 32;
const CHANGE_STATE_INTERVAL_SEC: u64 = 10;

/// Session manager.
pub struct Session {
    own_id: [u8; PEER_ID_SIZE],
    pieces_status: Vec<Status>,
    peers: HashMap<String, Peer>,
    general_channels: GeneralChannels,
    metainfo: Metainfo,
    candidates: Vec<(String, [u8; PEER_ID_SIZE])>,
    view: Option<View>,
    tracker: Job<TrackerCmd>,
    extractor: Job<ExtractorCmd>,
    round: usize,
    files_extracted: bool,
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
    Reserved(usize),
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

impl Session {
    /// Create new instance managing peer-to-peer connection. Currently most settings, are set as
    /// constatns.
    ///
    /// # Example
    /// ```no_run
    /// use rdest::{Metainfo, Session};
    /// use std::path::PathBuf;
    ///
    /// let path = PathBuf::from("ubuntu-20.04.1-desktop-amd64.iso.torrent");
    /// let torrent_file = Metainfo::from_file(path.as_path()).unwrap();
    /// let peer_id = b"AAAAABBBBBCCCCCDDDDD";
    ///
    /// let mut session = Session::new(torrent_file, *peer_id);
    /// ```
    pub fn new(metainfo: Metainfo, own_id: [u8; PEER_ID_SIZE]) -> Session {
        let (peer_tx, peer_rx) = mpsc::channel(CHANNEL_SIZE);
        let (tracker_tx, tracker_rx) = mpsc::channel(CHANNEL_SIZE);
        let (extractor_tx, extractor_rx) = mpsc::channel(CHANNEL_SIZE);
        let (broad, _) = broadcast::channel(BROADCAST_CHANNEL_SIZE);

        Session {
            own_id,
            pieces_status: vec![Status::Missing; metainfo.pieces_num()],
            peers: HashMap::new(),
            general_channels: GeneralChannels::new(peer_tx, peer_rx, broad),
            metainfo,
            candidates: vec![],
            view: None,
            tracker: Job::new(tracker_tx, tracker_rx),
            extractor: Job::new(extractor_tx, extractor_rx),
            round: 0,
            files_extracted: false,
        }
    }

    /// Run Session that will try connect to tracker, get list of available peers, and establish
    /// connection with them
    ///
    /// # Example
    /// ```no_run
    /// use rdest::{Metainfo, Session};
    /// use std::path::Path;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let path = Path::new("ubuntu-20.04.1-desktop-amd64.iso.torrent");
    /// let torrent_file = Metainfo::from_file(path).unwrap();
    /// let peer_id = b"AAAAABBBBBCCCCCDDDDD";
    ///
    /// let mut session = Session::new(torrent_file, *peer_id);
    /// session.run().await;
    /// # }
    /// ```
    pub async fn run(&mut self) {
        for i in 0..(self.pieces_status.len() - 20) {
            self.pieces_status[i] = Status::Have
        }

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
                _ = change_state_timer.tick() => self.timeout_change_conn_state().await.expect("Can't change connection state"),
                Ok((socket, _)) = listener.accept() => self.spawn_peer_listener(socket).await,
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

    async fn timeout_change_conn_state(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
            0 => self.new_optimistic_peers(),
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

        let state_before = self.conn_state_text();
        let cmd = self.change_conn_state(&mut rate, &new_optimistic)?;
        let state_after = self.conn_state_text();

        self.log(format!(
            "Peers: {}, connection state {} -> {}",
            self.peers.len(),
            state_before,
            state_after
        ))
        .await;
        let _ = self.general_channels.broad.send(cmd);
        Ok(())
    }

    fn conn_state_text(&self) -> String {
        let text: String = self
            .peers
            .iter()
            .map(|(_, peer)| "|".to_owned() + peer.status_abbreviation().as_str())
            .collect();

        match text.is_empty() {
            true => text,
            false => text + "|",
        }
    }

    fn new_optimistic_peers(&mut self) -> Vec<String> {
        let all_am_choked_and_peer_interested = self
            .peers
            .iter()
            .filter(|(_, peer)| peer.am_choked && peer.interested)
            .map(|(addr, _)| addr.clone())
            .collect::<Vec<String>>();

        match all_am_choked_and_peer_interested.choose(&mut rand::thread_rng()) {
            Some(addr) => vec![addr.clone()],
            None => vec![],
        }
    }

    fn change_conn_state(
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
            ExtractorCmd::Done => self.log("File extractor finish".to_string()).await,
            ExtractorCmd::Fail(e) => self.log("File extractor fail: ".to_string() + &e).await,
        }

        self.kill_extractor().await;
    }

    async fn handle_peer_cmd(&mut self, cmd: PeerCmd) -> Result<bool, Error> {
        match cmd {
            PeerCmd::Init {
                addr,
                peer_id,
                resp_ch,
            } => self.handle_init(&addr, peer_id, resp_ch).await,
            PeerCmd::RecvChoke { addr } => self.handle_choke(&addr).await,
            PeerCmd::RecvUnchoke { addr, resp_ch } => self.handle_unchoke(&addr, resp_ch).await,
            PeerCmd::RecvInterested { addr } => self.handle_interested(&addr).await,
            PeerCmd::RecvNotInterested { addr, resp_ch } => {
                self.handle_not_interested(&addr, resp_ch).await
            }
            PeerCmd::RecvHave {
                addr,
                piece_index,
                resp_ch,
            } => self.handle_have(&addr, piece_index, resp_ch),
            PeerCmd::RecvBitfield {
                addr,
                bitfield,
                resp_ch,
            } => self.handle_bitfield(&addr, &bitfield, resp_ch).await,
            PeerCmd::RecvRequest {
                addr,
                piece_index,
                resp_ch,
            } => self.handle_request(&addr, piece_index, resp_ch).await,
            PeerCmd::PieceDone { addr, resp_ch } => self.handle_piece_done(&addr, resp_ch).await,
            PeerCmd::PieceCancel { addr, resp_ch } => {
                self.handle_piece_cancel(&addr, resp_ch).await
            }
            PeerCmd::SyncStats {
                addr,
                downloaded_rate,
                uploaded_rate,
                unexpected_blocks,
            } => {
                self.handle_sync_stats(&addr, &downloaded_rate, &uploaded_rate, unexpected_blocks)
                    .await
            }
            PeerCmd::KillReq { addr, reason } => self.handle_kill_req(&addr, &reason).await,
        }
    }

    async fn handle_init(
        &mut self,
        addr: &String,
        peer_id: [u8; PEER_ID_SIZE],
        resp_ch: oneshot::Sender<InitCmd>,
    ) -> Result<bool, Error> {
        self.log_peer(addr, "Handshake with peer".to_string()).await;

        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        let cmd = peer.handle_init(peer_id, &self.pieces_status);
        let _ = resp_ch.send(cmd);
        Ok(true)
    }

    async fn handle_choke(&mut self, addr: &String) -> Result<bool, Error> {
        self.log_peer(addr, "Peer change state to Choke".to_string())
            .await;

        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        peer.handle_choke(&mut self.pieces_status);
        Ok(true)
    }

    async fn handle_unchoke(
        &mut self,
        addr: &String,
        resp_ch: oneshot::Sender<UnchokeCmd>,
    ) -> Result<bool, Error> {
        self.log_peer(addr, "Peer change state to Unchoke".to_string())
            .await;

        let chosen_index = self.choose_piece_index(addr).await;
        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        let cmd = peer.handle_unchoke(chosen_index, &mut self.pieces_status, &self.metainfo);
        let _ = &resp_ch.send(cmd);
        Ok(true)
    }

    async fn handle_interested(&mut self, addr: &String) -> Result<bool, Error> {
        self.log_peer(addr, "Peer change state to Interested".to_string())
            .await;

        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        peer.handle_interested();
        Ok(true)
    }

    async fn handle_not_interested(
        &mut self,
        addr: &String,
        resp_ch: oneshot::Sender<NotInterestedCmd>,
    ) -> Result<bool, Error> {
        self.log_peer(addr, "Peer change state to NotInterested".to_string())
            .await;

        let chosen_index = self.choose_piece_index(addr).await;
        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        let cmd = peer.handle_not_interested(chosen_index);
        let _ = resp_ch.send(cmd);
        Ok(true)
    }

    fn handle_have(
        &mut self,
        addr: &String,
        piece_index: usize,
        resp_ch: oneshot::Sender<HaveCmd>,
    ) -> Result<bool, Error> {
        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        let cmd = peer.handle_have(piece_index, &mut self.pieces_status, &self.metainfo);
        let _ = resp_ch.send(cmd);
        Ok(true)
    }

    async fn handle_bitfield(
        &mut self,
        addr: &String,
        bitfield: &Bitfield,
        resp_ch: oneshot::Sender<BitfieldCmd>,
    ) -> Result<bool, Error> {
        self.log_peer(addr, format!("Received a bitfield")).await;

        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        peer.update_pieces(&bitfield.to_vec(self.metainfo.pieces_num())?);

        let chosen_index = self.choose_piece_index(addr).await;
        let unchoked_num = self.unchoked_num();

        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        let cmd = peer.handle_bitfield(chosen_index, unchoked_num);
        let _ = &resp_ch.send(cmd);

        Ok(true)
    }

    async fn handle_request(
        &mut self,
        addr: &String,
        piece_index: usize,
        resp_ch: oneshot::Sender<RequestCmd>,
    ) -> Result<bool, Error> {
        self.log_peer(
            addr,
            format!("Peer sent request for piece: {}", piece_index),
        )
        .await;

        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        let cmd = peer.handle_request(piece_index, &self.pieces_status, &self.metainfo);
        let _ = resp_ch.send(cmd);
        Ok(true)
    }

    async fn handle_piece_done(
        &mut self,
        addr: &String,
        resp_ch: oneshot::Sender<PieceCmd>,
    ) -> Result<bool, Error> {
        match self.peers.get(addr).ok_or(Error::PeerNotFound)?.piece_index {
            Some(piece_index) => {
                self.pieces_status[piece_index] = Status::Have;
                let _ = self
                    .general_channels
                    .broad
                    .send(BroadCmd::SendHave { piece_index });
            }
            None => panic!("Piece downloaded but not requested"),
        }

        let chosen_index = self.choose_piece_index(addr).await;
        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        let cmd = peer.handle_piece(chosen_index, &mut self.pieces_status, &self.metainfo);
        let _ = resp_ch.send(cmd);
        Ok(true)
    }

    async fn handle_piece_cancel(
        &mut self,
        addr: &String,
        resp_ch: oneshot::Sender<PieceCmd>,
    ) -> Result<bool, Error> {
        match self.peers.get(addr).ok_or(Error::PeerNotFound)?.piece_index {
            Some(piece_index) => {
                self.pieces_status[piece_index] = match self.pieces_status[piece_index] {
                    Status::Reserved(peers_count) => match peers_count >= 2 {
                        true => Status::Reserved(peers_count - 1),
                        false => Status::Missing,
                    },
                    Status::Missing => Status::Missing,
                    Status::Have => Status::Have,
                }
            }
            None => panic!("Piece cancelled but not requested"),
        }

        let chosen_index = self.choose_piece_index(addr).await;
        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        let cmd = peer.handle_piece(chosen_index, &mut self.pieces_status, &self.metainfo);
        let _ = resp_ch.send(cmd);
        Ok(true)
    }

    async fn handle_sync_stats(
        &mut self,
        addr: &String,
        downloaded_rate: &Option<u32>,
        uploaded_rate: &Option<u32>,
        unexpected_blocks: usize,
    ) -> Result<bool, Error> {
        if unexpected_blocks > 0 {
            self.log_peer(
                addr,
                format!("Stats: Unexpected pieces {}", unexpected_blocks),
            )
            .await;
        }
        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        peer.handle_sync_stats(downloaded_rate, uploaded_rate);
        Ok(true)
    }

    async fn handle_kill_req(&mut self, addr: &String, reason: &String) -> Result<bool, Error> {
        self.log_peer(addr, "Peer killed, reason: ".to_string() + reason)
            .await;
        self.kill_peer(&addr).await;

        let have_all = self
            .pieces_status
            .iter()
            .all(|status| *status == Status::Have);

        if have_all {
            if !self.files_extracted {
                self.spawn_extractor().await;
            }
            self.files_extracted = true;
        } else if self.candidates.is_empty() {
            self.spawn_tracker();
        } else {
            self.spawn_peer_handler();
        }

        Ok(true)
    }

    fn unchoked_num(&self) -> usize {
        self.peers
            .iter()
            .filter(|(_, peer)| peer.am_choked == false && peer.optimistic_unchoke == true)
            .count()
    }

    async fn choose_piece_index(&mut self, addr: &String) -> Option<usize> {
        let pieces = &self.peers[addr].pieces;

        // Count how many peers have specific piece
        let mut vec: Vec<u32> = vec![0; self.metainfo.pieces_num()];
        for (_, peer) in self.peers.iter() {
            for (piece_index, have) in peer.pieces.iter().enumerate() {
                if *have {
                    vec[piece_index] += 1;
                }
            }
        }

        let still_missing = vec
            .iter()
            .enumerate()
            .filter(|(piece_index, _)| self.pieces_status[*piece_index] != Status::Have)
            .count();

        let is_desired: Box<dyn Fn(usize) -> bool> = match still_missing < END_GAME_LIMIT {
            true => Box::new(|idx: usize| self.pieces_status[idx] != Status::Have),
            false => Box::new(|idx: usize| self.pieces_status[idx] == Status::Missing),
        };

        // Create pair (piece_index, count) for desired pieces
        let mut rarest: Vec<(usize, u32)> = vec
            .iter()
            .enumerate()
            .filter(|(piece_index, _)| is_desired(*piece_index))
            .map(|(piece_index, count)| (piece_index, *count))
            .collect();

        drop(is_desired);

        // Shuffle to get better distribution of pieces from peers
        rarest.shuffle(&mut rand::thread_rng());

        // Sort by rarest
        rarest.sort_by(|(_, count1), (_, count2)| count1.cmp(&count2));

        for (piece_index, count) in rarest.iter() {
            if count > &0 && pieces[*piece_index] == true {
                if still_missing < END_GAME_LIMIT {
                    self.log_peer(addr, format!("End game mode, piece: {}", piece_index))
                        .await;
                }
                return Some(*piece_index);
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

    async fn spawn_extractor(&mut self) {
        self.log("Starting file extractor".to_string()).await;
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

        let mut peer_handler = PeerHandler::new(
            addr.clone(),
            self.own_id,
            Some(peer_id),
            *self.metainfo.info_hash(),
            self.metainfo.pieces_num(),
            self.general_channels.tx.clone(),
            self.general_channels.broad.subscribe(),
        );

        let job = tokio::spawn(async move { peer_handler.run_incoming().await });

        let peer = Peer::new(Some(peer_id), self.metainfo.pieces_num(), job);
        self.peers.insert(addr, peer);
    }

    async fn spawn_peer_listener(&mut self, socket: TcpStream) {
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

        let mut peer_handler = PeerHandler::new(
            addr.clone(),
            self.own_id,
            None,
            *self.metainfo.info_hash(),
            self.metainfo.pieces_num(),
            self.general_channels.tx.clone(),
            self.general_channels.broad.subscribe(),
        );

        let job = tokio::spawn(async move { peer_handler.run_outgoing(socket).await });

        self.log("New peer connect from: ".to_string() + &addr.as_str())
            .await;
        let peer = Peer::new(None, self.metainfo.pieces_num(), job);
        self.peers.insert(addr, peer);
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
                if let Some(piece_index) = peer.piece_index {
                    if self.pieces_status[piece_index] != Status::Have {
                        self.pieces_status[piece_index] = Status::Missing
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

    async fn log_peer(&mut self, addr: &String, text: String) {
        if let Some(view) = &mut self.view {
            if let Some(peer) = self.peers.get(addr) {
                let cmd = ViewCmd::LogPeer {
                    addr: addr.clone(),
                    peer_id: peer.id,
                    text,
                };
                let _ = view.channel.send(cmd).await;
            }
        }
    }
}
