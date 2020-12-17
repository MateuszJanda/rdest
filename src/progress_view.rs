use crate::commands::ViewCmd;
use std::io;
use std::io::Write;
use tokio::sync::mpsc;
use tokio::time;
use tokio::time::{Duration, Instant, Interval};

const CHANNEL_SIZE: usize = 32;
const DELAY_MS: u64 = 100;
const PROGRESS_SIZE: usize = 10;

pub struct ProgressView {
    pos: usize,
    direction: Direction,
    channel: mpsc::Receiver<ViewCmd>,
}

#[derive(PartialEq)]
enum Direction {
    Left,
    Right,
}

impl ProgressView {
    pub fn new() -> (ProgressView, mpsc::Sender<ViewCmd>) {
        let (channel_tx, channel_rx) = mpsc::channel(CHANNEL_SIZE);

        let view = ProgressView {
            pos: 1,
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
            _ => (),
        }

        true
    }

    async fn animation(&mut self) {
        let _text = " ".repeat(self.pos) + "a";
        // print!("\r{}", text);

        io::stdout().flush().unwrap();

        if self.direction == Direction::Right && self.pos + 1 > PROGRESS_SIZE {
            self.direction = Direction::Left;
        } else if self.direction == Direction::Left && self.pos - 1 < 1 {
            self.direction = Direction::Right;
        }

        match self.direction {
            Direction::Right => self.pos += 1,
            Direction::Left => self.pos -= 1,
        }
    }

    fn log(&self, text: &String) {
        println!("[+] {}", text);
    }
}
