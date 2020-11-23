use crate::constant::HASH_SIZE;
use crate::frame::Bitfield;
use crate::handler::{
    BitfieldCmd, BroadCmd, Handler, JobCmd, PieceDoneCmd, RequestCmd, UnchokeCmd,
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
    am_choke: bool,
    interested: bool,
    choke: bool,
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
            pieces_status: vec![Status::Missing; metainfo.pieces().len()],
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

        while let Some(cmd) = self.job_rx_ch.recv().await {
            if self.event_loop(cmd).await.expect("Can't handle event") == false {
                break;
            }
        }
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
        let pieces_num = self.metainfo.pieces().len();
        let job_ch = self.job_tx_ch.clone();
        let broad_ch = self.broad_ch.subscribe();

        let job = tokio::spawn(async move {
            Handler::run(
                addr, own_id, peer_id, info_hash, pieces_num, job_ch, broad_ch,
            )
            .await
        });

        let peer = Peer {
            pieces: vec![false; self.metainfo.pieces().len()],
            job: Some(job),
            index: None,
            am_interested: false,
            am_choke: true,
            interested: false,
            choke: true,
        };

        let (addr, _) = self.tracker.peers()[2].clone();
        self.peers.insert(addr, peer);
    }

    async fn event_loop(&mut self, cmd: JobCmd) -> Result<bool, Error> {
        match cmd {
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
                unexpected_piece,
                rejected_piece,
            } => Ok(true),
            JobCmd::KillReq {
                addr,
                index,
                reason,
            } => self.handle_kill_req(&addr, &index, &reason).await,
        }
    }

    fn handle_choke(&mut self, addr: &String) -> Result<bool, Error> {
        let peer = self.peers.get_mut(addr).ok_or(Error::NotFound)?;
        peer.choke = true;
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
                    piece_length: self.piece_length(index),
                    piece_hash: self.metainfo.pieces()[index],
                }
            }
            None => UnchokeCmd::SendNotInterested,
        };

        let peer = self.peers.get_mut(addr).ok_or(Error::NotFound)?;
        peer.index = index;
        peer.am_interested = index.is_some();

        let _ = &resp_ch.send(cmd);
        Ok(true)
    }

    fn handle_interested(&mut self, addr: &String) -> Result<bool, Error> {
        let peer = self.peers.get_mut(addr).ok_or(Error::NotFound)?;
        peer.interested = true;
        Ok(true)
    }

    fn handle_not_interested(&mut self, addr: &String) -> Result<bool, Error> {
        let peer = self.peers.get_mut(addr).ok_or(Error::NotFound)?;
        peer.interested = false;
        Ok(true)
    }

    fn handle_have(&mut self, addr: &String, index: usize) -> Result<bool, Error> {
        let peer = self.peers.get_mut(addr).ok_or(Error::NotFound)?;
        peer.pieces[index] = true;
        Ok(true)
    }

    fn handle_bitfield(
        &mut self,
        addr: &String,
        bitfield: &Bitfield,
        resp_ch: oneshot::Sender<BitfieldCmd>,
    ) -> Result<bool, Error> {
        let peer = self.peers.get_mut(addr).ok_or(Error::NotFound)?;
        peer.pieces.copy_from_slice(&bitfield.to_vec());

        let bitfield = Bitfield::from_vec(
            &self
                .pieces_status
                .iter()
                .map(|status| *status == Status::Have)
                .collect(),
        );

        let _ = resp_ch.send(BitfieldCmd::SendBitfield { bitfield });

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
        let _ = resp_ch.send(RequestCmd::Ignore);
        Ok(true)
    }

    fn handle_piece_done(
        &mut self,
        addr: &String,
        resp_ch: oneshot::Sender<PieceDoneCmd>,
    ) -> Result<bool, Error> {
        match self.peers.get(addr).ok_or(Error::NotFound)?.index {
            Some(index) => {
                self.pieces_status[index] = Status::Have;
                let _ = self.broad_ch.send(BroadCmd::SendHave { index });
            }
            None => (),
        }

        let _ = resp_ch.send(PieceDoneCmd::PrepareKill);
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
        let mut v: Vec<u32> = vec![0; self.metainfo.pieces().len()];

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

    fn piece_length(&self, index: usize) -> usize {
        if index < self.metainfo.pieces().len() - 1 {
            return self.metainfo.piece_length();
        }

        self.metainfo.total_length() as usize % self.metainfo.piece_length()
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
                let _ = view.channel.send(ViewCmd::Kill {}).await;
                let job = &mut view.job;
                job.await.unwrap();
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
                let name = utils::hash_to_string(&self.metainfo.pieces()[idx]) + ".piece";
                let reader = &mut BufReader::new(File::open(name)?);

                if idx == start.file_index {
                    reader.seek(std::io::SeekFrom::Start(start.byte_index as u64))?;
                }

                let mut buffer = vec![];
                reader.read_to_end(&mut buffer)?;
                writer.write_all(buffer.as_slice())?;
            }

            // Write last chunk
            let name = utils::hash_to_string(&self.metainfo.pieces()[end.file_index]) + ".piece";
            let reader = &mut BufReader::new(File::open(name)?);

            let mut buffer = vec![0; end.byte_index];
            reader.read_exact(buffer.as_mut_slice())?;
            writer.write_all(buffer.as_slice())?;
        }

        Ok(())
    }
}
