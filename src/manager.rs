use crate::constant::HASH_SIZE;
use crate::frame::Bitfield;
use crate::handler::{
    BitfieldCmd, BroadCmd, Handler, InitCmd, JobCmd, PieceDoneCmd, RequestCmd, UnchokeCmd,
};
use crate::progress::{Progress, ViewCmd};
use crate::{utils, Error, Metainfo, TrackerResp};
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, Write};
use std::path::Path;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;

const JOB_CHANNEL_SIZE: usize = 64;
const BROADCAST_CHANNEL_SIZE: usize = 32;

pub struct Manager {
    own_id: [u8; HASH_SIZE],
    pieces_status: Vec<Status>,
    peers: HashMap<String, Peer>,
    metainfo: Metainfo,
    tracker: TrackerResp,
    view: Option<View>,
    job_tx_ch: mpsc::Sender<JobCmd>,
    job_rx_ch: mpsc::Receiver<JobCmd>,
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
    download_rate: f32,
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

impl Manager {
    pub fn new(metainfo: Metainfo, tracker: TrackerResp, own_id: [u8; HASH_SIZE]) -> Manager {
        let (job_tx_ch, job_rx_ch) = mpsc::channel(JOB_CHANNEL_SIZE);
        let (broad_ch, _) = broadcast::channel(BROADCAST_CHANNEL_SIZE);

        Manager {
            own_id,
            pieces_status: vec![Status::Missing; metainfo.pieces_num()],
            peers: HashMap::new(),
            metainfo,
            tracker,
            view: None,
            job_tx_ch,
            job_rx_ch,
            broad_ch,
        }
    }

    pub async fn run(&mut self) {
        self.spawn_view();
        self.spawn_jobs();

        self.event_loop().await;
    }

    fn spawn_view(&mut self) {
        let (mut view, channel) = Progress::new();
        self.view = Some(View {
            channel,
            job: tokio::spawn(async move { view.run().await }),
        });
    }

    fn spawn_jobs(&mut self) {
        let (addr, peer_id) = self.tracker.peers()[2].clone();
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

        let peer = Peer {
            pieces: vec![false; self.metainfo.pieces_num()],
            job: Some(job),
            index: None,
            am_interested: false,
            am_choked: true,
            interested: false,
            choked: true,
            download_rate: 0.0,
            rejected_piece: 0,
        };

        let (addr, _) = self.tracker.peers()[2].clone();
        self.peers.insert(addr, peer);
    }

    async fn event_loop(&mut self) {
        loop {
            tokio::select! {
                Some(cmd) = self.job_rx_ch.recv() => {
                    if self.handle_job_cmd(cmd).await.expect("Can't handle command") == false {
                        break;
                    }
                }
            }
        }
    }

    async fn handle_job_cmd(&mut self, cmd: JobCmd) -> Result<bool, Error> {
        match cmd {
            JobCmd::Init { addr, resp_ch } => self.handle_init(&addr, resp_ch).await,
            JobCmd::RecvChoke { addr } => self.handle_choke(&addr),
            JobCmd::RecvUnchoke { addr, resp_ch } => self.handle_unchoke(&addr, resp_ch),
            JobCmd::RecvInterested { addr } => self.handle_interested(&addr),
            JobCmd::RecvNotInterested { addr } => self.handle_not_interested(&addr),
            JobCmd::RecvHave { addr, index } => self.handle_have(&addr, index),
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
                rejected_piece,
            } => self.handle_sync_stats(&addr, downloaded_rate, rejected_piece),
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
        let cmd = match index {
            Some(index) => {
                self.pieces_status[index] = Status::Reserved;
                UnchokeCmd::SendInterestedAndRequest {
                    index,
                    piece_length: self.metainfo.piece_length(index),
                    piece_hash: *self.metainfo.piece(index),
                }
            }
            None => UnchokeCmd::SendNotInterested,
        };

        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
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

    fn handle_not_interested(&mut self, addr: &String) -> Result<bool, Error> {
        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        peer.interested = false;
        Ok(true)
    }

    fn handle_have(&mut self, addr: &String, index: usize) -> Result<bool, Error> {
        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        peer.pieces[index] = true;
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
        {
            let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
            peer.pieces.copy_from_slice(&bitfield.to_vec());
        }

        // BEP3 "whenever a downloader doesn't have something they currently would ask a peer for in
        // unchoked, they must express lack of interest, despite being choked"
        let index = self.choose_piece(&bitfield.to_vec());
        let am_interested = match index {
            Some(_) => true,
            None => false,
        };

        let am_choked = false; // TODO

        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        peer.am_interested = am_interested;
        peer.am_choked = am_choked;

        let cmd = BitfieldCmd::SendState {
            am_choked: Some(am_choked),
            am_interested,
        };

        let _ = &resp_ch.send(cmd);

        Ok(true)
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
            None => (),
        }

        let _ = resp_ch.send(PieceDoneCmd::PrepareKill);
        Ok(true)
    }

    fn handle_sync_stats(
        &mut self,
        addr: &String,
        downloaded_rate: f32,
        rejected_piece: u32,
    ) -> Result<bool, Error> {
        let peer = self.peers.get_mut(addr).ok_or(Error::PeerNotFound)?;
        peer.download_rate = downloaded_rate;
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
        let mut v: Vec<u32> = vec![0; self.metainfo.pieces_num()];

        for (_, peer) in self.peers.iter() {
            for (index, have) in peer.pieces.iter().enumerate() {
                if *have {
                    v[index] += 1;
                }
            }
        }

        // Shuffle to get better distribution of pieces from peers
        v.shuffle(&mut rand::thread_rng());

        let mut rarest: Vec<(usize, u32)> = v
            .iter()
            .enumerate()
            .filter(|(idx, _)| self.pieces_status[*idx] == Status::Missing)
            .map(|(idx, count)| (idx, *count))
            .collect();

        // Sort by rarest
        rarest.sort_by(|(_, a_count), (_, b_count)| a_count.cmp(&b_count));

        for (index, count) in rarest.iter() {
            if count > &0 && pieces[*index] == true {
                return Some(*index);
            }
        }

        None
    }

    async fn kill_job(&mut self, addr: &String, index: &Option<usize>) {
        if index.is_some() && self.pieces_status[index.unwrap()] != Status::Have {
            self.pieces_status[index.unwrap()] = Status::Missing;
        }

        let j = self.peers.get_mut(addr).unwrap().job.take();
        j.unwrap().await.unwrap();

        self.peers.remove(addr);

        println!("Job killed");
    }

    async fn kill_view(&mut self) {
        match &mut self.view.take() {
            Some(view) => {
                let _ = view.channel.send(ViewCmd::Kill).await;
                let job = &mut view.job;
                job.await.unwrap();
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
            for idx in start.file_index..end.file_index {
                let name = utils::hash_to_string(&self.metainfo.piece(idx)) + ".piece";
                let reader = &mut BufReader::new(File::open(name)?);

                if idx == start.file_index {
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
