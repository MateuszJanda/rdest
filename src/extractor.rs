use crate::commands::ExtractorCmd;
use crate::{utils, Metainfo};
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, Write};
use std::path::Path;
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
            fs::create_dir_all(Path::new(path).parent().unwrap())?;

            // Create output file
            let mut writer = BufWriter::new(File::create(path)?);

            // Write pieces/chunks
            for index in start.file_index..end.file_index {
                let name = utils::hash_to_string(&self.metainfo.piece(index)) + ".piece";
                let reader = &mut BufReader::new(File::open(name)?);

                if index == start.file_index {
                    reader.seek(std::io::SeekFrom::Start(start.byte_index as u64))?;
                }

                let mut buffer = vec![];
                reader.read_to_end(&mut buffer)?;
                writer.write_all(buffer.as_slice())?;
            }

            // Write last chunk
            let name = utils::hash_to_string(&self.metainfo.piece(end.file_index)) + ".piece";
            let reader = &mut BufReader::new(File::open(name)?);

            let mut buffer = vec![0; end.byte_index];
            reader.read_exact(buffer.as_mut_slice())?;
            writer.write_all(buffer.as_slice())?;
        }

        Ok(())
    }
}
