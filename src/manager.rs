use crate::{Bitfield, Command, Handler, Metainfo, Request, TrackerResp};
use tokio::sync::mpsc;

pub struct Manager {
    tx: mpsc::Sender<Command>,
    rx: mpsc::Receiver<Command>,
    t: Metainfo,
    r: TrackerResp,
    pieces_len: usize,
}

impl Manager {
    pub fn new(t: Metainfo, r: TrackerResp) -> Manager {
        let (tx, rx) = mpsc::channel(32);

        let pieces_len = t.pieces().len();
        Manager {
            tx,
            rx,
            t,
            r,
            pieces_len,
        }
    }

    pub fn rrr(&self) {
        println!("Spawning new job");

        let addr = self.r.peers()[2].clone();
        let info_hash = self.t.info_hash();
        let peer_id = b"ABCDEFGHIJKLMNOPQRST";
        let tx2 = self.tx.clone();

        let job = tokio::spawn(async move { Handler::fff(addr, info_hash, *peer_id, tx2).await });

        // job.await.unwrap();
    }

    pub async fn run(&mut self) {
        self.rrr();

        let mut peer_bitfield = vec![false; self.pieces_len];

        let my_pieces = vec![false; self.pieces_len];
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                Command::RecvBitfield {
                    key,
                    bitfield,
                    channel,
                } => {
                    peer_bitfield = bitfield.available_pieces();

                    for i in 0..my_pieces.len() {
                        if my_pieces[i] == false && peer_bitfield[i] == true {
                            let mut size = self.pieces_len / 8;
                            if self.pieces_len % 8 != 0 {
                                size += 1;
                            }

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
}
