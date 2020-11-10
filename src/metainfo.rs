use crate::raw_finder::RawFinder;
use crate::Error;
use crate::{BDecoder, BValue, DeepFinder};
use sha1;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fs;

const HASH_SIZE: usize = 20;

#[derive(PartialEq, Debug)]
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
    pub fn from_file(path: String) -> Result<Metainfo, Error> {
        match &fs::read(path) {
            Ok(val) => Self::from_bencode(val),
            Err(_) => Err(Error::Meta("File not found".into())),
        }
    }

    pub fn from_bencode(data: &[u8]) -> Result<Metainfo, Error> {
        let bvalues = BDecoder::from_array(data)?;

        if bvalues.is_empty() {
            return Err(Error::Meta("Empty bencode".into()));
        }

        let mut err = Err(Error::Meta("Missing data".into()));
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
            return Err(Error::Meta(
                "Conflicting 'length' and 'files' values present. Only one is allowed".into(),
            ));
        } else if length.is_none() && multi_files.is_none() {
            return Err(Error::Meta("Missing 'length' or 'files'".into()));
        }

        let name = Self::find_name(dict)?;
        let mut files = vec![];
        if length.is_some() {
            files.push(File {
                length: length.unwrap(),
                path: name.clone(),
            });
        } else if multi_files.is_some() {
            files = multi_files.unwrap();
        }

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
            Some(BValue::ByteStr(val)) => String::from_utf8(val.to_vec())
                .or(Err(Error::Meta("Can't convert 'announce' to UTF-8".into()))),
            _ => Err(Error::Meta("Incorrect or missing 'announce' value".into())),
        }
    }

    pub fn find_name(dict: &HashMap<Vec<u8>, BValue>) -> Result<String, Error> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"name".to_vec()) {
                Some(BValue::ByteStr(val)) => String::from_utf8(val.to_vec())
                    .or(Err(Error::Meta("Can't convert 'name' to UTF-8".into()))),
                _ => Err(Error::Meta("Incorrect or missing 'name' value".into())),
            },
            _ => Err(Error::Meta("Incorrect or missing 'info' value".into())),
        }
    }

    pub fn find_piece_length(dict: &HashMap<Vec<u8>, BValue>) -> Result<u64, Error> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"piece length".to_vec()) {
                Some(BValue::Int(length)) => u64::try_from(*length).or(Err(Error::Meta(
                    "Can't convert 'piece length' to u64".into(),
                ))),
                _ => Err(Error::Meta(
                    "Incorrect or missing 'piece length' value".into(),
                )),
            },
            _ => Err(Error::Meta("Incorrect or missing 'info' value".into())),
        }
    }

    pub fn find_pieces(dict: &HashMap<Vec<u8>, BValue>) -> Result<Vec<[u8; HASH_SIZE]>, Error> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"pieces".to_vec()) {
                Some(BValue::ByteStr(pieces)) => {
                    if pieces.len() % HASH_SIZE != 0 {
                        return Err(Error::Meta("'pieces' not divisible by 20".into()));
                    }
                    Ok(pieces
                        .chunks(HASH_SIZE)
                        .map(|chunk| chunk.try_into().unwrap())
                        .collect())
                }
                _ => Err(Error::Meta("Incorrect or missing 'pieces' value".into())),
            },
            _ => Err(Error::Meta("Incorrect or missing 'info' value".into())),
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

        Err(Error::HashCalculation)
    }

    pub fn tracker_url(&self) -> String {
        self.announce.clone()
    }

    pub fn pieces(&self) -> Vec<[u8; HASH_SIZE]> {
        // TODO: maybe &Vec<>
        self.pieces.clone()
    }

    pub fn piece_length(&self) -> usize {
        self.piece_length as usize
    }

    pub fn total_length(&self) -> u64 {
        self.files.iter().map(|f| f.length).sum()
    }

    pub fn info_hash(&self) -> [u8; HASH_SIZE] {
        self.info_hash.clone()
    }

    pub fn file_piece_ranges(&self) -> Vec<(String, PiecePos, PiecePos)> {
        let dir = if self.files.len() > 1 {
            self.name.clone() + "/"
        } else {
            "".to_string()
        };

        let mut ranges: Vec<(String, PiecePos, PiecePos)> = vec![];
        let mut pos: usize = 0;

        for File { length, path } in self.files.iter() {
            ranges.push((
                dir.clone() + path,
                self.piece_pos(pos),
                self.piece_pos(pos + *length as usize),
            ));

            pos += *length as usize;
        }

        ranges
    }

    fn piece_pos(&self, pos: usize) -> PiecePos {
        let file_index = if pos % self.piece_length as usize != 0 {
            pos / self.piece_length as usize + 1
        } else {
            pos / self.piece_length as usize
        };

        let byte_index = pos % self.piece_length as usize;

        PiecePos {
            file_index,
            byte_index,
        }
    }
}
