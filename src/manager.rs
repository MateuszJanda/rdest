use crate::{Bitfield, Command, Handler, Metainfo, Request, TrackerResp};
use tokio::sync::mpsc;

pub struct Manager {
    own_id: [u8; 20],
    metainfo: Metainfo,
    tracker: TrackerResp,
    cmd_tx: mpsc::Sender<Command>,
    cmd_rx: mpsc::Receiver<Command>,
}

impl Manager {
    pub fn new(metainfo: Metainfo, tracker: TrackerResp, own_id: [u8; 20]) -> Manager {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);

        Manager {
            own_id,
            metainfo,
            tracker,
            cmd_tx,
            cmd_rx,
        }
    }

    pub async fn run(&mut self) {
        self.spawn_jobs();

        let mut peer_bitfield = vec![false; self.metainfo.pieces().len()];

        let my_pieces = vec![false; self.metainfo.pieces().len()];
        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                Command::RecvBitfield {
                    key,
                    bitfield,
                    channel,
                } => {
                    peer_bitfield = bitfield.available_pieces();

                    for i in 0..my_pieces.len() {
                        if my_pieces[i] == false && peer_bitfield[i] == true {
                            let size = self.bitfield_size();

                            let my = Bitfield::new(vec![0; size]);
                            channel.send(Command::SendBitfield {
                                bitfield: my,
                                interested: true,
                            });
                            break;
                        }
                    }
                }
                Command::RecvUnchoke { key, channel } => {
                    for i in 0..my_pieces.len() {
                        if my_pieces[i] == false && peer_bitfield[i] == true {
                            let my = Request::new(i, 0, 0x4000 as usize);
                            channel.send(Command::SendRequest { req: my });
                            break;
                        }
                    }
                }
                _ => (),
            }
        }
    }

    fn bitfield_size(&self) -> usize {
        let mut size = self.metainfo.pieces().len() / 8;
        if self.metainfo.pieces().len() % 8 != 0 {
            size += 1;
        }

        size
    }

    fn spawn_jobs(&self) {
        let addr = self.tracker.peers()[2].clone();
        let info_hash = self.metainfo.info_hash();
        let own_id = self.own_id.clone();
        let cmd_tx = self.cmd_tx.clone();

        let job = tokio::spawn(async move { Handler::fff(addr, info_hash, own_id, cmd_tx).await });
    }
}
