use crate::Error;
use crate::raw_finder::RawFinder;
use crate::{BDecoder, BValue, DeepFinder};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs;

extern crate sha1;

#[derive(PartialEq, Debug)]
pub struct Metainfo {
    pub announce: String,
    pub name: String,
    pub piece_length: u64,
    pub pieces: Vec<Vec<u8>>,
    pub length: Option<u64>,
    pub files: Option<Vec<File>>,
    pub hash: [u8; 20],
}

#[derive(PartialEq, Clone, Debug)]
pub struct File {
    pub length: u64,
    pub path: String,
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
        // let raw_info = BValue::cut_raw_info(arg)?;

        if bvalues.is_empty() {
            return Err(Error::Meta(format!("Empty torrent")));
        }

        let mut err = Err(Error::Meta(format!("Missing data")));
        for val in bvalues {
            match val {
                BValue::Dict(dict) => match Self::create(data, &dict) {
                    Ok(torrent) => return Ok(torrent),
                    Err(e) => err = Err(e),
                },
                _ => (),
            }
        }

        err
    }

    fn create(data: &[u8], dict: &HashMap<Vec<u8>, BValue>) -> Result<Metainfo, Error> {
        let torrent = Metainfo {
            announce: Self::find_announce(dict)?,
            name: Self::find_name(dict)?,
            piece_length: Self::find_piece_length(dict)?,
            pieces: Self::find_pieces(dict)?,
            length: Self::find_length(dict),
            files: Self::find_files(dict),
            hash: Self::info_hash(data),
        };

        if !torrent.is_valid() {
            return Err(Error::Meta(format!(
                "Conflicting values 'length' and 'files'. Only one is allowed"
            )));
        }

        Ok(torrent)
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
                Some(BValue::Int(length)) => {
                    u64::try_from(*length).or(Err(Error::Meta("Can't convert 'piece length' to u64".into())))
                }
                _ => Err(Error::Meta("Incorrect or missing 'piece length' value".into())),
            },
            _ => Err(Error::Meta("Incorrect or missing 'info' value".into())),
        }
    }

    pub fn find_pieces(dict: &HashMap<Vec<u8>, BValue>) -> Result<Vec<Vec<u8>>, Error> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"pieces".to_vec()) {
                Some(BValue::ByteStr(pieces)) => {
                    if pieces.len() % 20 != 0 {
                        return Err(Error::Meta("'pieces' not divisible by 20".into()));
                    }
                    Ok(pieces.chunks(20).map(|chunk| chunk.to_vec()).collect())
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

    pub fn is_valid(&self) -> bool {
        if self.length.is_some() && self.files.is_some() {
            return false;
        } else if self.length.is_none() && self.files.is_none() {
            return false;
        }

        return true;
    }

    pub fn url(&self) -> String {
        self.announce.clone()
    }

    pub fn length(&self) -> u64 {
        // TODO
        return self.length.unwrap();
    }

    fn info_hash(data: &[u8]) -> [u8; 20] {
        let info = DeepFinder::find_first("4:info", data).unwrap(); // TODO
        let mut m = sha1::Sha1::new();

        // let v: Vec<u8> = vec![1, 2, 3];

        // m.update(b"Hello World!");
        m.update(info.as_ref());
        println!("{:?}", m.digest().to_string());

        m.digest().bytes()
    }
}
