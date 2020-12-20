use crate::commands::{BroadCmd, ViewCmd};
use num_traits::abs;
use std::io;
use std::io::Write;
use termion::color;
use termion::color::Color;
use tokio::sync::{broadcast, mpsc};
use tokio::time;
use tokio::time::{Duration, Instant, Interval};

const CHANNEL_SIZE: usize = 32;
const DELAY_MS: u64 = 100;
const TAIL_SIZE: usize = 4;
const PROGRESS_SIZE: usize = 10;

pub struct ProgressView {
    pos: usize,
    pieces: Vec<bool>,
    direction: Direction,
    channel: mpsc::Receiver<ViewCmd>,
    broad_ch: broadcast::Receiver<BroadCmd>,
}

#[derive(PartialEq)]
enum Direction {
    Left,
    Right,
}

impl ProgressView {
    pub fn new(
        pieces_num: usize,
        broad_ch: broadcast::Receiver<BroadCmd>,
    ) -> (ProgressView, mpsc::Sender<ViewCmd>) {
        let (channel_tx, channel_rx) = mpsc::channel(CHANNEL_SIZE);

        let view = ProgressView {
            pos: 0,
            pieces: vec![false; pieces_num],
            direction: Direction::Right,
            channel: channel_rx,
            broad_ch,
        };

        (view, channel_tx)
    }

    pub async fn run(&mut self) {
        // println!("{}", cursor::Hide);
        println!(
            r#"
   _i_i_     .----
⸝⸍/     \⸌⸜  / Ok, let's go with it...
||\  ¬  /||                  ~rdest~
\_,"" ""._/
"#
        );

        let mut animation_timer = self.start_animation_timer();

        loop {
            tokio::select! {
                 _ = animation_timer.tick() => self.animation().await,
                 Ok(cmd) = self.broad_ch.recv() => self.handle_manager_cmd(cmd),
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

    fn handle_manager_cmd(&mut self, cmd: BroadCmd) {
        match cmd {
            BroadCmd::SendHave { index } => self.pieces[index] = true,
            _ => (),
        }
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
        let downloaded = self.pieces.iter().filter(|&val| *val).count();
        print!(
            "\r{}{}[{}/{}]: ",
            color::Fg(color::Red),
            color::Bg(color::Reset),
            downloaded,
            self.pieces.len(),
        );

        for (ch, fg, bg) in self.snake() {
            print!("{}{}{}", color::Fg(fg.as_ref()), color::Bg(bg.as_ref()), ch)
        }

        match io::stdout().flush() {
            Ok(_) => (),
            Err(_) => (),
        }

        if self.direction == Direction::Right && self.pos >= PROGRESS_SIZE - 1 {
            self.direction = Direction::Left;
        } else if self.direction == Direction::Left && self.pos <= 0 {
            self.direction = Direction::Right;
        }

        match self.direction {
            Direction::Right => self.pos += 1,
            Direction::Left => self.pos -= 1,
        }
    }

    fn snake(&self) -> Vec<(char, Box<dyn Color>, Box<dyn Color>)> {
        let mut bar: Vec<(char, Box<dyn Color>, Box<dyn Color>)> = vec![];
        for _ in 0..PROGRESS_SIZE {
            bar.push((' ', Box::new(color::Reset), Box::new(color::Reset)));
        }

        for segment in 1..=TAIL_SIZE {
            let pos = self.segment_pos(segment as i32);
            bar[pos] = (
                '▄',
                Box::new(color::Rgb(255 / (segment as u8 + 1), 0, 0)),
                Box::new(color::Rgb(255 / (segment as u8 + 1), 0, 0)),
            );
        }

        bar[self.pos] = (
            '█',
            Box::new(color::Rgb(255, 0, 0)),
            Box::new(color::Rgb(255, 0, 0)),
        );

        bar
    }

    fn segment_pos(&self, segment: i32) -> usize {
        let mut pos = match self.direction {
            Direction::Left => self.pos as i32 + segment,
            Direction::Right => self.pos as i32 - segment,
        };

        if pos >= PROGRESS_SIZE as i32 {
            pos = PROGRESS_SIZE as i32 - (pos % (PROGRESS_SIZE - 1) as i32)
        }
        if pos < 0 {
            pos = abs(pos) - 1
        }

        pos as usize
    }

    fn log(&self, text: &String) {
        println!(
            "\r{}{}[+] {}",
            color::Fg(color::Reset),
            color::Bg(color::Reset),
            text
        );
    }
}
