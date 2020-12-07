use crate::constant::HASH_SIZE;
use crate::raw_finder::RawFinder;
use crate::Error;
use crate::{BDecoder, BValue, DeepFinder};
use sha1;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fs;
use std::path::PathBuf;

#[derive(PartialEq, Clone, Debug)]
pub struct Metainfo {
    announce: String,
    name: String,
    piece_length: u64,
    pieces: Vec<[u8; HASH_SIZE]>,
    files: Vec<File>,
    info_hash: [u8; HASH_SIZE],
}

#[derive(PartialEq, Clone, Debug)]
pub struct File {
    pub length: u64,
    pub path: String,
}

pub struct PiecePos {
    pub file_index: usize,
    pub byte_index: usize,
}

impl Metainfo {
    pub fn from_file(path: PathBuf) -> Result<Metainfo, Error> {
        match &fs::read(path) {
            Ok(val) => Self::from_bencode(val),
            Err(_) => Err(Error::MetaFileNotFound),
        }
    }

    pub fn from_bencode(data: &[u8]) -> Result<Metainfo, Error> {
        let bvalues = BDecoder::from_array(data)?;

        if bvalues.is_empty() {
            return Err(Error::MetaBEncodeMissing);
        }

        let mut err = Err(Error::MetaDataMissing);
        for val in bvalues {
            match val {
                BValue::Dict(dict) => match Self::parse(data, &dict) {
                    Ok(torrent) => return Ok(torrent),
                    Err(e) => err = Err(e),
                },
                _ => (),
            }
        }

        err
    }

    fn parse(data: &[u8], dict: &HashMap<Vec<u8>, BValue>) -> Result<Metainfo, Error> {
        let length = Self::find_length(dict);
        let multi_files = Self::find_files(dict);

        if length.is_some() && multi_files.is_some() {
            return Err(Error::MetaLenAndFilesConflict);
        } else if length.is_none() && multi_files.is_none() {
            return Err(Error::MetaLenOrFilesMissing);
        }

        let name = Self::find_name(dict)?;
        let files = match length {
            Some(length) => vec![File {
                length,
                path: name.clone(),
            }],
            None => match multi_files {
                Some(multi_files) => multi_files,
                None => vec![],
            },
        };

        let metainfo = Metainfo {
            announce: Self::find_announce(dict)?,
            name,
            piece_length: Self::find_piece_length(dict)?,
            pieces: Self::find_pieces(dict)?,
            files,
            info_hash: Self::calculate_hash(data)?,
        };

        Ok(metainfo)
    }

    pub fn find_announce(dict: &HashMap<Vec<u8>, BValue>) -> Result<String, Error> {
        match dict.get(&b"announce".to_vec()) {
            Some(BValue::ByteStr(val)) => {
                String::from_utf8(val.to_vec()).or(Err(Error::MetaInvalidUtf8("announce".into())))
            }
            _ => Err(Error::MetaIncorrectOrMissing("announce".into())),
        }
    }

    pub fn find_name(dict: &HashMap<Vec<u8>, BValue>) -> Result<String, Error> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"name".to_vec()) {
                Some(BValue::ByteStr(val)) => {
                    String::from_utf8(val.to_vec()).or(Err(Error::MetaInvalidUtf8("name".into())))
                }
                _ => Err(Error::MetaIncorrectOrMissing("name".into())),
            },
            _ => Err(Error::MetaIncorrectOrMissing("info".into())),
        }
    }

    pub fn find_piece_length(dict: &HashMap<Vec<u8>, BValue>) -> Result<u64, Error> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"piece length".to_vec()) {
                Some(BValue::Int(length)) => {
                    u64::try_from(*length).or(Err(Error::MetaInvalidU64("piece length".into())))
                }
                _ => Err(Error::MetaIncorrectOrMissing("piece length".into())),
            },
            _ => Err(Error::MetaIncorrectOrMissing("info".into())),
        }
    }

    pub fn find_pieces(dict: &HashMap<Vec<u8>, BValue>) -> Result<Vec<[u8; HASH_SIZE]>, Error> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"pieces".to_vec()) {
                Some(BValue::ByteStr(pieces)) => {
                    if pieces.len() % HASH_SIZE != 0 {
                        return Err(Error::MetaNotDivisible("pieces".into()));
                    }
                    Ok(pieces
                        .chunks(HASH_SIZE)
                        .map(|chunk| chunk.try_into().unwrap())
                        .collect())
                }
                _ => Err(Error::MetaIncorrectOrMissing("pieces".into())),
            },
            _ => Err(Error::MetaIncorrectOrMissing("info".into())),
        }
    }

    pub fn find_length(dict: &HashMap<Vec<u8>, BValue>) -> Option<u64> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"length".to_vec()) {
                Some(BValue::Int(length)) => u64::try_from(*length).ok(),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn find_files(dict: &HashMap<Vec<u8>, BValue>) -> Option<Vec<File>> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"files".to_vec()) {
                Some(BValue::List(list)) => Some(Self::file_list(list)),
                _ => None,
            },
            _ => None,
        }
    }

    fn file_list(list: &Vec<BValue>) -> Vec<File> {
        list.iter()
            .filter_map(|elem| match elem {
                BValue::Dict(dict) => Some(dict),
                _ => None,
            })
            .filter_map(
                |dict| match (dict.get(&b"length".to_vec()), dict.get(&b"path".to_vec())) {
                    (Some(BValue::Int(length)), Some(BValue::ByteStr(path))) => {
                        Some((length, path))
                    }
                    _ => None,
                },
            )
            .filter_map(|(length, path)| {
                match (u64::try_from(*length), String::from_utf8(path.to_vec())) {
                    (Ok(l), Ok(p)) => Some(File { length: l, path: p }),
                    _ => None,
                }
            })
            .collect()
    }

    fn calculate_hash(data: &[u8]) -> Result<[u8; HASH_SIZE], Error> {
        if let Some(info) = DeepFinder::find_first("4:info", data) {
            let mut m = sha1::Sha1::new();
            m.update(info.as_ref());
            return Ok(m.digest().bytes());
        }

        Err(Error::InfoMissing)
    }

    pub fn tracker_url(&self) -> &String {
        &self.announce
    }

    pub fn piece(&self, index: usize) -> &[u8; HASH_SIZE] {
        &self.pieces[index]
    }

    pub fn pieces_num(&self) -> usize {
        self.pieces.len()
    }

    pub fn piece_length(&self, index: usize) -> usize {
        if index < self.pieces.len() - 1 {
            return self.piece_length as usize;
        }

        let last = self.total_length() as usize % self.piece_length as usize;
        if last != 0 {
            return last;
        }

        return self.piece_length as usize;
    }

    pub fn total_length(&self) -> u64 {
        self.files.iter().map(|f| f.length).sum()
    }

    pub fn info_hash(&self) -> &[u8; HASH_SIZE] {
        &self.info_hash
    }

    pub fn file_piece_ranges(&self) -> Vec<(PathBuf, PiecePos, PiecePos)> {
        let dir = match self.files.len() > 1 {
            true => PathBuf::from(&self.name),
            false => PathBuf::new(),
        };

        let mut ranges: Vec<(PathBuf, PiecePos, PiecePos)> = vec![];
        let mut pos: usize = 0;

        for File { length, path } in self.files.iter() {
            ranges.push((
                dir.join(path),
                self.piece_pos(pos),
                self.piece_pos(pos + *length as usize),
            ));

            pos += *length as usize;
        }

        ranges
    }

    fn piece_pos(&self, pos: usize) -> PiecePos {
        let file_index = match pos % self.piece_length as usize != 0 {
            true => pos / self.piece_length as usize + 1,
            false => pos / self.piece_length as usize,
        };

        let byte_index = pos % self.piece_length as usize;

        PiecePos {
            file_index,
            byte_index,
        }
    }
}
