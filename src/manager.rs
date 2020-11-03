use crate::handler::{Done, RecvBitfield, RecvUnchoke, VerifyFail};
use crate::{Bitfield, Command, Handler, Metainfo, TrackerResp};
use std::collections::HashMap;
use tokio::sync::mpsc;
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
    job: JoinHandle<()>,
}

impl Manager {
    pub fn new(metainfo: Metainfo, tracker: TrackerResp, own_id: [u8; 20]) -> Manager {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);

        Manager {
            own_id,
            bitfield_size: Self::bitfield_size(&metainfo),
            pieces_status: vec![Status::Missing; metainfo.pieces().len()],
            peers: HashMap::new(),

            metainfo,
            tracker,
            cmd_tx,
            cmd_rx,
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
                Command::Done(cmd) => {
                    self.recv_done(cmd);
                }
                Command::VerifyFail(cmd) => {
                    self.recv_verify_fail(cmd);
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
                let _ = msg.channel.send(Command::SendRequest {
                    index: idx,
                    piece_size: self.piece_size(idx),
                    piece_hash: self.metainfo.pieces()[idx],
                });
                break;
            }
        }
    }

    fn recv_done(&mut self, msg: Done) {
        let _ = msg.channel.send(Command::Kill);
    }

    fn recv_verify_fail(&mut self, msg: VerifyFail) {
        let _ = msg.channel.send(Command::Kill);
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
        let cmd_tx = self.cmd_tx.clone();

        let job = tokio::spawn(async move { Handler::run(addr, own_id, info_hash, cmd_tx).await });

        let p = Peer {
            pieces: vec![],
            job,
        };

        self.peers.insert(self.tracker.peers()[2].clone(), p);
    }
}
