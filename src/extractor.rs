// Copyright 2020 Mateusz Janda.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::commands::ExtractorCmd;
use crate::{utils, Metainfo};
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, Write};
use tokio::sync::mpsc;

pub struct Extractor {
    metainfo: Metainfo,
    channel: mpsc::Sender<ExtractorCmd>,
}

impl Extractor {
    pub fn new(metainfo: Metainfo, channel: mpsc::Sender<ExtractorCmd>) -> Extractor {
        Extractor { metainfo, channel }
    }

    pub async fn run(&mut self) {
        let cmd = match self.extract_files() {
            Ok(()) => ExtractorCmd::Done,
            Err(e) => ExtractorCmd::Fail(e.to_string()),
        };

        self.channel
            .send(cmd)
            .await
            .expect("Can't communicate to manager")
    }

    fn extract_files(&self) -> Result<(), Box<dyn std::error::Error>> {
        for (path, start, end) in self.metainfo.file_piece_ranges().iter() {
            // Create directories if needed
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Create output file
            let mut writer = BufWriter::new(File::create(path)?);

            // Write pieces/chunks
            for piece_index in start.file_index..end.file_index {
                let name = utils::hash_to_string(&self.metainfo.piece(piece_index)) + ".piece";
                let reader = &mut BufReader::new(File::open(name)?);

                if piece_index == start.file_index {
                    reader.seek(std::io::SeekFrom::Start(start.byte_index as u64))?;
                }

                let mut buffer = vec![];
                reader.read_to_end(&mut buffer)?;
                writer.write_all(buffer.as_slice())?;
            }

            // Write last chunk
            if end.byte_index > 0 {
                let name = utils::hash_to_string(&self.metainfo.piece(end.file_index)) + ".piece";
                let reader = &mut BufReader::new(File::open(name)?);

                let mut buffer = vec![0; end.byte_index];
                reader.read_exact(buffer.as_mut_slice())?;
                writer.write_all(buffer.as_slice())?;
            }
        }

        Ok(())
    }
}
