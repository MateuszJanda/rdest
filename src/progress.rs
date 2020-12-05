use crate::commands::ViewCmd;
use std::io;
use std::io::Write;
use tokio::sync::mpsc;
use tokio::time::{interval_at, Duration, Instant};

pub struct Progress {
    pos: usize,
    dir: i32,
    cmd_rx: mpsc::Receiver<ViewCmd>,
}

impl Progress {
    pub fn new() -> (Progress, mpsc::Sender<ViewCmd>) {
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
                        Some(ViewCmd::Log(text)) => self.log(&text),
                        Some(ViewCmd::Kill) => break,
                        _ => (),
                    }
                }
            }
        }
    }

    async fn animation(&mut self) {
        let _text = " ".repeat(self.pos) + "a";
        // print!("\r{}", text);

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

    fn log(&self, text: &String) {
        println!("[+] {}", text);
    }
}
