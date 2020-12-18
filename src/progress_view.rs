use crate::commands::ViewCmd;
use std::io;
use std::io::Write;
use tokio::io::Error;
use tokio::sync::mpsc;
use tokio::time;
use tokio::time::{Duration, Instant, Interval};

const CHANNEL_SIZE: usize = 32;
const DELAY_MS: u64 = 100;
const PROGRESS_SIZE: usize = 10;
const PROGRESS_START_POS: usize = 1;

pub struct ProgressView {
    pos: usize,
    pieces_num: usize,
    pieces: usize,
    direction: Direction,
    channel: mpsc::Receiver<ViewCmd>,
}

#[derive(PartialEq)]
enum Direction {
    Left,
    Right,
}

impl ProgressView {
    pub fn new(pieces_num: usize) -> (ProgressView, mpsc::Sender<ViewCmd>) {
        let (channel_tx, channel_rx) = mpsc::channel(CHANNEL_SIZE);

        let view = ProgressView {
            pos: PROGRESS_START_POS,
            pieces_num,
            pieces: 0,
            direction: Direction::Right,
            channel: channel_rx,
        };

        (view, channel_tx)
    }

    pub async fn run(&mut self) {
        let mut animation_timer = self.start_animation_timer();

        loop {
            tokio::select! {
                 _ = animation_timer.tick() => self.animation().await,
                 cmd = self.channel.recv() => {
                    if !self.handle_cmd(cmd) {
                        break;
                    }
                }
            }
        }
    }

    fn start_animation_timer(&self) -> Interval {
        let start = Instant::now() + Duration::from_millis(0);
        time::interval_at(start, Duration::from_millis(DELAY_MS))
    }

    fn handle_cmd(&self, cmd: Option<ViewCmd>) -> bool {
        match cmd {
            Some(ViewCmd::Log(text)) => self.log(&text),
            Some(ViewCmd::Kill) => return false,
            None => (),
        }

        true
    }

    async fn animation(&mut self) {
        let text = " ".repeat(self.pos) + "a";
        print!("\r[{}/{}]:{}", self.pieces, self.pieces_num, text);

        match io::stdout().flush() {
            Ok(_) => (),
            Err(_) => (),
        }

        if self.direction == Direction::Right && self.pos + 1 > PROGRESS_START_POS + PROGRESS_SIZE {
            self.direction = Direction::Left;
        } else if self.direction == Direction::Left && self.pos - 1 < PROGRESS_START_POS {
            self.direction = Direction::Right;
        }

        match self.direction {
            Direction::Right => self.pos += 1,
            Direction::Left => self.pos -= 1,
        }
    }

    fn log(&self, text: &String) {
        println!("\r[+] {}", text);
    }
}
