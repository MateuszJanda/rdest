use crate::handler::{BroadcastCommand, Done, RecvBitfield, RecvHave, RecvUnchoke, VerifyFail};
use crate::{Bitfield, Command, Handler, Metainfo, TrackerResp};
use std::collections::HashMap;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

pub struct Manager {
    own_id: [u8; 20],
    bitfield_size: usize,
    pieces_status: Vec<Status>,
    peers: HashMap<String, Peer>,

    metainfo: Metainfo,
    tracker: TrackerResp,
    cmd_tx: mpsc::Sender<Command>,
    cmd_rx: mpsc::Receiver<Command>,

    b_tx: broadcast::Sender<BroadcastCommand>,
}

#[derive(PartialEq, Clone, Debug)]
enum Status {
    Missing,
    Reserved,
    Have,
}

#[derive(Debug)]
struct Peer {
    pieces: Vec<bool>,
    job: Option<JoinHandle<()>>,
    index: usize,
}

impl Manager {
    pub fn new(metainfo: Metainfo, tracker: TrackerResp, own_id: [u8; 20]) -> Manager {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);

        let (b_tx, _) = broadcast::channel(16);

        Manager {
            own_id,
            bitfield_size: Self::bitfield_size(&metainfo),
            pieces_status: vec![Status::Missing; metainfo.pieces().len()],
            peers: HashMap::new(),

            metainfo,
            tracker,
            cmd_tx,
            cmd_rx,

            b_tx,
        }
    }

    pub async fn run(&mut self) {
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
                        break;
                    }
                }
                _ => (),
            }
        }
    }

    fn recv_bitfield(&mut self, msg: RecvBitfield) {
        let p = msg.bitfield.available_pieces();
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

        for idx in 0..self.pieces_status.len() {
            if self.pieces_status[idx] == Status::Missing && pieces[idx] == true {
                let bitfield = Bitfield::new(vec![0; self.bitfield_size]);
                let _ = msg.channel.send(Command::SendBitfield {
                    bitfield,
                    interested: true,
                });

                break;
            }
        }
    }

    fn recv_unchoke(&mut self, msg: RecvUnchoke) {
        let pieces = &self.peers[&msg.key].pieces;

        for idx in 0..self.pieces_status.len() {
            if self.pieces_status[idx] == Status::Missing && pieces[idx] == true {
                self.pieces_status[idx] = Status::Reserved;

                self.peers.get_mut(&msg.key).unwrap().index = idx;

                let _ = msg.channel.send(Command::SendRequest {
                    index: idx,
                    piece_size: self.piece_size(idx),
                    piece_hash: self.metainfo.pieces()[idx],
                });
                break;
            }
        }
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

    fn bitfield_size(metainfo: &Metainfo) -> usize {
        let mut size = metainfo.pieces().len() / 8;
        if metainfo.pieces().len() % 8 != 0 {
            size += 1;
        }

        size
    }

    fn spawn_jobs(&mut self) {
        let addr = self.tracker.peers()[2].clone();
        let info_hash = self.metainfo.info_hash();
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
}
