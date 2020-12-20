use crate::commands::{BroadCmd, ViewCmd};
use std::io;
use std::io::Write;
use termion::{color, cursor};
use tokio::sync::{broadcast, mpsc};
use tokio::time;
use tokio::time::{Duration, Instant, Interval};
use termion::color::Color;
use std::convert::TryInto;
use num_traits::abs;

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
        // let text = "▄".repeat(self.pos) + "a";

        let downloaded = self.pieces.iter().filter(|&val| *val).count();

        print!(
            "\r{}{}[{}/{}]: ",
            color::Fg(color::Red),
            color::Bg(color::Reset),
            downloaded,
            self.pieces.len(),
        );

        for (ch, fg, bg) in self.ttt2() {
            // let c = fg.try_into();
            // let c = fg.into();
            // let c: Color = fg;
            // let c= fg.as_ref();
            // print!("{} x", color::Fg(c));
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

    fn ttt(&self) -> Vec<(char, Box<    dyn Color>, Box<dyn Color>)> {
        let mut r:Vec<(char, Box<dyn Color>, Box<dyn Color>)> = vec![];
        for i in 0..self.pos {
            r.push((' ', Box::new(color::Reset), Box::new(color::Reset)));
        }

        r.push(('█', Box::new(color::Rgb(255, 0, 0)), Box::new(color::Rgb(255, 0, 0))));


        for i in (self.pos + 1)..PROGRESS_SIZE {
            r.push((' ', Box::new(color::Reset), Box::new(color::Reset)));
        }

        r
    }

    fn ttt2(&self) -> Vec<(char, Box<    dyn Color>, Box<dyn Color>)> {
        let mut r:Vec<(char, Box<dyn Color>, Box<dyn Color>)> = vec![];
        for i in 0..PROGRESS_SIZE {
            r.push((' ', Box::new(color::Reset), Box::new(color::Reset)));
        }



        // match self.direction {
        //     Direction::Left => {
        //         if self.pos >= PROGRESS_SIZE - TAIL_SIZE {
        //
        //         } else {
        //             for _ self.pos
        //         }
        //     }
        //     Direction::Right => {}
        // }

        for p in (1..=TAIL_SIZE).rev() {
            let pp = self.dir(p as i32);
            r[pp] = ('█', Box::new(color::Rgb(255/ (p as u8+ 1), 0, 0)), Box::new(color::Rgb(255/(p as u8+ 1), 0, 0)));
        }

        r[self.pos] = ('█', Box::new(color::Rgb(255, 0, 0)), Box::new(color::Rgb(255, 0, 0)));

        r
    }

    fn dir(&self, p : i32) -> usize {
        let mut r = match self.direction {
            Direction::Left => { self.pos as i32 + p}
            Direction::Right => { self.pos as i32 - p}
        };

        if r >= PROGRESS_SIZE as i32 {
            r = PROGRESS_SIZE as i32 - (r % (PROGRESS_SIZE - 1) as i32)
            // r = PROGRESS_SIZE as i32 - 1
        }
        if r < 0{
            r = abs(r) - 1
            // r = 0
        }

        r as usize
    }

    fn aaa(&self) {
        let chunk_size = (self.pieces.len() as f32) / ((PROGRESS_SIZE * 2) as f32);

        let start = 0 as f32;
        let end = start + chunk_size;


        // self.pieces[start as usize..end as usize].iter().



    }


    fn log(&self, text: &String) {
        println!("\r{}{}[+] {}", color::Fg(color::Reset), color::Bg(color::Reset), text);
    }
}
