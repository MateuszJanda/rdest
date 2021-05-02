// Copyright 2020 Mateusz Janda.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::commands::{BroadCmd, ViewCmd};
use crate::constants::PEER_ID_SIZE;
use crate::utils::hash_to_string;
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
const SNAKE_TAIL_SIZE: usize = 4;
const BAR_SIZE: usize = 10;

pub struct ProgressView {
    head: usize,
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
            head: 0,
            pieces: vec![false; pieces_num],
            direction: Direction::Right,
            channel: channel_rx,
            broad_ch,
        };

        (view, channel_tx)
    }

    pub async fn run(&mut self) {
        println!(
            r#"
   _i_i_     .----
⸝⸍/     \⸌⸜  / Ok, let's go with this...
||\  ¬  /||                  ~rdest~
\_,"" ""._/
"#
        );

        let mut animation_timer = self.start_animation_timer();

        loop {
            tokio::select! {
                 _ = animation_timer.tick() => self.progress_animation().await,
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
            BroadCmd::SendHave { piece_index } => self.pieces[piece_index] = true,
            _ => (),
        }
    }

    fn handle_cmd(&self, cmd: Option<ViewCmd>) -> bool {
        match cmd {
            Some(ViewCmd::LogPeer {
                addr,
                peer_id,
                text,
            }) => self.log_peer(&addr, &peer_id, &text),
            Some(ViewCmd::Log(text)) => self.log(&text),
            Some(ViewCmd::Kill) => return false,
            None => (),
        }

        true
    }

    async fn progress_animation(&mut self) {
        let downloaded = self.pieces.iter().filter(|&val| *val).count();
        print!(
            "\r{}{}[{}/{}]: ",
            color::Fg(color::Red),
            color::Bg(color::Reset),
            downloaded,
            self.pieces.len(),
        );

        for (ch, fg, bg) in self.snake_chars() {
            print!("{}{}{}", color::Fg(fg.as_ref()), color::Bg(bg.as_ref()), ch)
        }
        print!("{}{}", color::Fg(color::Reset), color::Bg(color::Reset),);

        io::stdout().flush().unwrap();
        self.move_snake();
    }

    fn move_snake(&mut self) {
        if self.direction == Direction::Right && self.head >= BAR_SIZE - 1 {
            self.direction = Direction::Left;
        } else if self.direction == Direction::Left && self.head <= 0 {
            self.direction = Direction::Right;
        }

        match self.direction {
            Direction::Right => self.head += 1,
            Direction::Left => self.head -= 1,
        }
    }

    fn snake_chars(&self) -> Vec<(char, Box<dyn Color>, Box<dyn Color>)> {
        let mut bar: Vec<(char, Box<dyn Color>, Box<dyn Color>)> = vec![];
        for _ in 0..BAR_SIZE {
            bar.push((' ', Box::new(color::Reset), Box::new(color::Reset)));
        }

        for segment in (0..=SNAKE_TAIL_SIZE).rev() {
            let pos = self.snake_segment_to_pos(segment as i32);
            let color = color::Rgb(255 / (segment as u8 + 1), 0, 0);
            bar[pos] = ('▄', Box::new(color), Box::new(color));
        }

        bar
    }

    fn snake_segment_to_pos(&self, segment: i32) -> usize {
        let mut pos = match self.direction {
            Direction::Left => self.head as i32 + segment,
            Direction::Right => self.head as i32 - segment,
        };

        if pos >= BAR_SIZE as i32 {
            pos = BAR_SIZE as i32 - (pos % (BAR_SIZE - 1) as i32)
        }
        if pos < 0 {
            pos = abs(pos) - 1
        }

        pos as usize
    }

    fn log_peer(&self, addr: &String, peer_id: &Option<[u8; PEER_ID_SIZE]>, text: &String) {
        let peer_id = match peer_id {
            None => "".to_string(),
            Some(peer_id) => match String::from_utf8(peer_id.to_vec()) {
                Ok(s) => s,
                Err(_) => hash_to_string(&peer_id),
            },
        };

        println!("\r[{}]:[{}] {}", peer_id, addr, text);
    }

    fn log(&self, text: &String) {
        println!("\r{}", text);
    }
}
