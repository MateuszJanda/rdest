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

pub struct Manager {
    own_id: [u8; HASH_SIZE],
    pieces_status: Vec<Status>,
    peers: HashMap<String, Peer>,

    metainfo: Metainfo,
    tracker: TrackerResp,
    cmd_tx: mpsc::Sender<JobCmd>,
    cmd_rx: mpsc::Receiver<JobCmd>,

    b_tx: broadcast::Sender<BroadCmd>,
    view: Option<View>,
}

#[derive(Debug)]
struct Peer {
    pieces: Vec<bool>,
    job: Option<JoinHandle<()>>,
    index: usize,
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
        let (cmd_tx, cmd_rx) = mpsc::channel(64);
        let (b_tx, _) = broadcast::channel(32);

        Manager {
            own_id,
            pieces_status: vec![Status::Missing; metainfo.pieces().len()],
            peers: HashMap::new(),

            metainfo,
            tracker,
            cmd_tx,
            cmd_rx,

            b_tx,
            view: None,
        }
    }

    pub async fn run(&mut self) {
        self.spawn_progress_view();
        self.spawn_jobs();

        while let Some(cmd) = self.cmd_rx.recv().await {
            if self.event_loop(cmd).await == false {
                break;
            }
        }
    }

    fn spawn_progress_view(&mut self) {
        let (mut view, channel) = Progress::new();
        self.view = Some(View {
            channel,
            job: tokio::spawn(async move { view.run().await }),
        });
    }

    fn spawn_jobs(&mut self) {
        let (addr, peer_id) = self.tracker.peers()[2].clone();
        let a = addr.clone();
        let info_hash = *self.metainfo.info_hash();
        let own_id = self.own_id.clone();
        let pieces_count = self.metainfo.pieces().len();
        let cmd_tx = self.cmd_tx.clone();

        let b_rx = self.b_tx.subscribe();

        let job = tokio::spawn(async move {
            Handler::run(addr, own_id, peer_id, info_hash, pieces_count, cmd_tx, b_rx).await
        });

        let p = Peer {
            pieces: vec![],
            job: Some(job),
            index: 0,
        };

        self.peers.insert(a, p);
    }

    async fn event_loop(&mut self, cmd: JobCmd) -> bool {
        match cmd {
            JobCmd::RecvChoke { addr } => self.handle_choke(&addr),
            JobCmd::RecvUnchoke {
                addr,
                buffered_req,
                resp_ch,
            } => self.handle_unchoke(&addr, buffered_req, resp_ch),
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
                begin,
                length,
                resp_ch,
            } => self.handle_request(&addr, index, begin, length, resp_ch),
            JobCmd::PieceDone { addr, resp_ch } => self.handle_piece_done(&addr, resp_ch),
            JobCmd::SyncStats {
                addr,
                downloaded_rate,
                unexpected_piece,
                rejected_piece,
            } => true,
            JobCmd::KillReq {
                addr,
                index,
                reason,
            } => self.handle_kill_req(&addr, &index, &reason).await,
        }
    }

    fn handle_choke(&mut self, addr: &String) -> bool {
        true
    }

    fn handle_unchoke(
        &mut self,
        addr: &String,
        buffered_req: bool,
        resp_ch: oneshot::Sender<UnchokeCmd>,
    ) -> bool {
        let pieces = &self.peers[addr].pieces;
        let cmd = match self.choose_piece(pieces) {
            Err(_) => UnchokeCmd::SendNotInterested,
            Ok(idx) => {
                self.pieces_status[idx] = Status::Reserved;
                self.peers.get_mut(addr).unwrap().index = idx;

                UnchokeCmd::SendRequest {
                    index: idx,
                    piece_size: self.piece_size(idx),
                    piece_hash: self.metainfo.pieces()[idx],
                }
            }
        };

        let _ = &resp_ch.send(cmd);
        true
    }

    fn handle_interested(&mut self, addr: &String) -> bool {
        true
    }

    fn handle_not_interested(&mut self, addr: &String) -> bool {
        true
    }

    fn handle_have(&mut self, addr: &String, index: usize) -> bool {
        self.peers.get_mut(addr).unwrap().pieces[index] = true;
        true
    }

    fn handle_bitfield(
        &mut self,
        addr: &String,
        bitfield: &Bitfield,
        resp_ch: oneshot::Sender<BitfieldCmd>,
    ) -> bool {
        let p = bitfield.to_vec();
        self.peers
            .get_mut(addr)
            .unwrap()
            .pieces
            .resize(p.len(), false);

        self.peers.get_mut(addr).unwrap().pieces.copy_from_slice(&p);

        let pieces = &self.peers[addr].pieces;

        let interested = match self.choose_piece(pieces) {
            Err(_) => false,
            Ok(_) => true,
        };

        let bitfield = Bitfield::from_vec(
            &self
                .pieces_status
                .iter()
                .map(|status| *status == Status::Have)
                .collect(),
        );
        let _ = resp_ch.send(BitfieldCmd::SendBitfield {
            bitfield,
            interested,
        });

        true
    }

    fn handle_request(
        &mut self,
        addr: &String,
        index: usize,
        begin: usize,
        length: usize,
        resp_ch: oneshot::Sender<RequestCmd>,
    ) -> bool {
        let _ = resp_ch.send(RequestCmd::Ignore);
        true
    }

    fn handle_piece_done(&mut self, addr: &String, resp_ch: oneshot::Sender<PieceDoneCmd>) -> bool {
        for (key, peer) in self.peers.iter() {
            if key == addr {
                self.pieces_status[peer.index] = Status::Have;

                let _ = self.b_tx.send(BroadCmd::SendHave { index: peer.index });

                break;
            }
        }

        let _ = resp_ch.send(PieceDoneCmd::PrepareKill);
        true
    }

    async fn handle_kill_req(
        &mut self,
        addr: &String,
        index: &Option<usize>,
        reason: &String,
    ) -> bool {
        self.kill_job(&addr, &index).await;

        if self.peers.is_empty() {
            self.kill_progress_view().await;
            if let Err(_) = self.extract_files() {
                ()
            }
            return false;
        }

        true
    }

    fn choose_piece(&self, pieces: &Vec<bool>) -> Result<usize, Error> {
        let mut v: Vec<u32> = vec![0; self.metainfo.pieces().len()];

        for (_, peer) in self.peers.iter() {
            for (idx, have) in peer.pieces.iter().enumerate() {
                if *have {
                    v[idx] += 1;
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

        for (idx, count) in rarest.iter() {
            if count > &0 && pieces[*idx] == true {
                return Ok(*idx);
            }
        }

        Err(Error::NotFound)
    }

    fn piece_size(&self, index: usize) -> usize {
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

    async fn kill_progress_view(&mut self) {
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
