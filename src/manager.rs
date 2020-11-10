use crate::handler::{BroadcastCommand, Done, RecvBitfield, RecvHave, RecvUnchoke, VerifyFail};
use crate::progress::{ProCmd, Progress};
use crate::{utils, Bitfield, Command, Error, Handler, Metainfo, TrackerResp};
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, Write};
use std::path::Path;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

pub struct Manager {
    own_id: [u8; 20],
    pieces_status: Vec<Status>,
    peers: HashMap<String, Peer>,

    metainfo: Metainfo,
    tracker: TrackerResp,
    cmd_tx: mpsc::Sender<Command>,
    cmd_rx: mpsc::Receiver<Command>,

    b_tx: broadcast::Sender<BroadcastCommand>,

    pro_cmd: Option<mpsc::Sender<ProCmd>>,
    progress_job: Option<JoinHandle<()>>,
}

#[derive(Debug)]
struct Peer {
    pieces: Vec<bool>,
    job: Option<JoinHandle<()>>,
    index: usize,
}

#[derive(PartialEq, Clone, Debug)]
pub enum Status {
    Missing,
    Reserved,
    Have,
}

impl Manager {
    pub fn new(metainfo: Metainfo, tracker: TrackerResp, own_id: [u8; 20]) -> Manager {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let (b_tx, _) = broadcast::channel(16);

        Manager {
            own_id,
            pieces_status: vec![Status::Missing; metainfo.pieces().len()],
            peers: HashMap::new(),

            metainfo,
            tracker,
            cmd_tx,
            cmd_rx,

            b_tx,
            pro_cmd: None,
            progress_job: None,
        }
    }

    pub async fn run(&mut self) {
        // self.spawn_progress();

        self.spawn_jobs();

        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                Command::RecvBitfield(r) => {
                    self.recv_bitfield(r);
                }
                Command::RecvUnchoke(cmd) => {
                    self.recv_unchoke(cmd);
                }
                Command::RecvHave(cmd) => {
                    self.recv_have(cmd);
                }
                Command::Done(cmd) => {
                    self.recv_done(cmd);
                }
                Command::VerifyFail(cmd) => {
                    self.recv_verify_fail(cmd);
                }
                Command::KillReq { key, index } => {
                    self.kill_job(&key, &index).await;

                    if self.peers.is_empty() {
                        self.kill_progress().await;
                        if let Err(_) = self.extract_files() {
                            ()
                        }
                        break;
                    }
                }
                _ => (),
            }
        }
    }

    fn recv_bitfield(&mut self, msg: RecvBitfield) {
        let p = msg.bitfield.to_vec();
        self.peers
            .get_mut(&msg.key)
            .unwrap()
            .pieces
            .resize(p.len(), false);

        self.peers
            .get_mut(&msg.key)
            .unwrap()
            .pieces
            .copy_from_slice(&p);

        let pieces = &self.peers[&msg.key].pieces;

        let interested = match self.choose_piece(pieces) {
            Err(_) => false,
            Ok(_) => true,
        };

        let bitfield = Bitfield::from_vec(
            &self
                .pieces_status
                .iter()
                .map(|x| *x == Status::Have)
                .collect(),
        );
        let _ = msg.channel.send(Command::SendBitfield {
            bitfield,
            interested,
        });
    }

    fn recv_unchoke(&mut self, msg: RecvUnchoke) {
        let pieces = &self.peers[&msg.key].pieces;

        let cmd = match self.choose_piece(pieces) {
            Err(_) => Command::SendNotInterested,
            Ok(idx) => {
                self.pieces_status[idx] = Status::Reserved;
                self.peers.get_mut(&msg.key).unwrap().index = idx;

                Command::SendRequest {
                    index: idx,
                    piece_size: self.piece_size(idx),
                    piece_hash: self.metainfo.pieces()[idx],
                }
            }
        };

        let _ = msg.channel.send(cmd);
    }

    fn choose_piece(&self, pieces: &Vec<bool>) -> Result<usize, Error> {
        let mut v: Vec<u32> = vec![0; self.metainfo.pieces().len()];

        for (_, p) in self.peers.iter() {
            for (idx, have) in p.pieces.iter().enumerate() {
                if *have {
                    v[idx] += 1;
                }
            }
        }

        v.shuffle(&mut rand::thread_rng());

        let mut x: Vec<(usize, u32)> = v
            .iter()
            .enumerate()
            .filter(|val| self.pieces_status[val.0] == Status::Missing)
            .map(|y| (y.0, *y.1))
            .collect();

        // Sort by rarest
        x.sort_by(|a, b| a.1.cmp(&b.1));

        for (idx, count) in x.iter() {
            if count > &0 && pieces[*idx] == true {
                return Ok(*idx);
            }
        }

        Err(Error::NotFound)
    }

    fn recv_have(&mut self, msg: RecvHave) {
        self.peers.get_mut(&msg.key).unwrap().pieces[msg.index] = true;
    }

    fn recv_done(&mut self, msg: Done) {
        for (key, peer) in self.peers.iter() {
            if key == &msg.key {
                self.pieces_status[peer.index] = Status::Have;

                let _ = self.b_tx.send(BroadcastCommand::SendHave {
                    key: msg.key.clone(),
                    index: peer.index,
                });

                break;
            }
        }

        let _ = msg.channel.send(Command::End);
    }

    fn recv_verify_fail(&mut self, msg: VerifyFail) {
        let _ = msg.channel.send(Command::End);
    }

    fn piece_size(&self, index: usize) -> usize {
        if index < self.metainfo.pieces().len() - 1 {
            return self.metainfo.piece_length();
        }

        self.metainfo.total_length() as usize % self.metainfo.piece_length()
    }

    fn spawn_progress(&mut self) {
        let (mut p, r) = Progress::new();

        self.progress_job = Some(tokio::spawn(async move { p.run().await }));

        self.pro_cmd = Some(r);
    }

    fn spawn_jobs(&mut self) {
        let addr = self.tracker.peers()[2].clone();
        let info_hash = *self.metainfo.info_hash();
        let own_id = self.own_id.clone();
        let pieces_count = self.metainfo.pieces().len();
        let cmd_tx = self.cmd_tx.clone();

        let b_rx = self.b_tx.subscribe();

        let job = tokio::spawn(async move {
            Handler::run(addr, own_id, info_hash, pieces_count, cmd_tx, b_rx).await
        });

        let p = Peer {
            pieces: vec![],
            job: Some(job),
            index: 0,
        };

        self.peers.insert(self.tracker.peers()[2].clone(), p);
    }

    async fn kill_job(&mut self, key: &String, index: &Option<usize>) {
        if index.is_some() && self.pieces_status[index.unwrap()] != Status::Have {
            self.pieces_status[index.unwrap()] = Status::Missing;
        }

        let j = self.peers.get_mut(key).unwrap().job.take();
        j.unwrap().await.unwrap();

        self.peers.remove(key);

        println!("Job killed");
    }

    async fn kill_progress(&mut self) {
        match &mut self.pro_cmd {
            Some(r) => {
                let _ = r.send(ProCmd::Kill {}).await;
            }
            _ => (),
        }

        match &mut self.progress_job {
            Some(j) => {
                j.await.unwrap();
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
