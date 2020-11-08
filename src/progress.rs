use tokio::time::{interval_at, Duration, Instant};
use std::io;
use std::io::Write;

pub(crate) struct Progress {
    pub(crate) pos: usize,
    pub(crate) dir: i32,
}

impl Progress {
    pub async fn run(&mut self) {
        let start = Instant::now() + Duration::from_millis(0);
        let mut interval = interval_at(start, Duration::from_millis(100));

        loop {
            tokio::select! {
                 _ = interval.tick() => self.animation().await,
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