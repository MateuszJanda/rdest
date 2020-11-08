use std::io;
use std::io::Write;
use tokio::sync::mpsc;
use tokio::time::{interval_at, Duration, Instant};

pub enum ProCmd {
    Kill,
}

pub struct Progress {
    pos: usize,
    dir: i32,
    cmd_rx: mpsc::Receiver<ProCmd>,
}

impl Progress {
    pub fn new() -> (Progress, mpsc::Sender<ProCmd>) {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);

        let p = Progress {
            pos: 1,
            dir: 1,
            cmd_rx,
        };

        (p, cmd_tx)
    }

    pub async fn run(&mut self) {
        let start = Instant::now() + Duration::from_millis(0);
        let mut interval = interval_at(start, Duration::from_millis(100));

        loop {
            tokio::select! {
                 _ = interval.tick() => self.animation().await,
                 cmd = self.cmd_rx.recv() => {
                     match cmd {
                        Some(ProCmd::Kill) => break,
                        _ => (),
                    }
                }
            }
        }
    }

    async fn animation(&mut self) {
        let text = " ".repeat(self.pos) + "a";
        print!("\r{}", text);

        io::stdout().flush().unwrap();

        if self.dir > 0 && self.pos + 1 > 10 {
            self.dir = -1;
        } else if self.dir < 0 && self.pos - 1 < 1 {
            self.dir = 1;
        }

        if self.dir > 0 {
            self.pos += 1;
        } else {
            self.pos -= 1;
        }
    }
}
