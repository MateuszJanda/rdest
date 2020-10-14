#[cfg(test)]
use crate::hashmap;
use crate::Error;
use crate::raw_finder::RawFinder;
use crate::{BDecoder, BValue, DeepFinder};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs;

extern crate sha1;

#[derive(PartialEq, Debug)]
pub struct Metainfo {
    announce: String,
    name: String,
    piece_length: u64,
    pieces: Vec<Vec<u8>>,
    length: Option<u64>,
    files: Option<Vec<File>>,
    pub hash: [u8; 20],
}

#[derive(PartialEq, Clone, Debug)]
pub struct File {
    length: u64,
    path: String,
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

    fn find_announce(dict: &HashMap<Vec<u8>, BValue>) -> Result<String, Error> {
        match dict.get(&b"announce".to_vec()) {
            Some(BValue::ByteStr(val)) => String::from_utf8(val.to_vec())
                .or(Err(Error::Meta("Can't convert 'announce' to UTF-8".into()))),
            _ => Err(Error::Meta("Incorrect or missing 'announce' value".into())),
        }
    }

    fn find_name(dict: &HashMap<Vec<u8>, BValue>) -> Result<String, Error> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"name".to_vec()) {
                Some(BValue::ByteStr(val)) => String::from_utf8(val.to_vec())
                    .or(Err(Error::Meta("Can't convert 'name' to UTF-8".into()))),
                _ => Err(Error::Meta("Incorrect or missing 'name' value".into())),
            },
            _ => Err(Error::Meta("Incorrect or missing 'info' value".into())),
        }
    }

    fn find_piece_length(dict: &HashMap<Vec<u8>, BValue>) -> Result<u64, Error> {
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

    fn find_pieces(dict: &HashMap<Vec<u8>, BValue>) -> Result<Vec<Vec<u8>>, Error> {
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

    fn find_length(dict: &HashMap<Vec<u8>, BValue>) -> Option<u64> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"length".to_vec()) {
                Some(BValue::Int(length)) => u64::try_from(*length).ok(),
                _ => None,
            },
            _ => None,
        }
    }

    fn find_files(dict: &HashMap<Vec<u8>, BValue>) -> Option<Vec<File>> {
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

    fn is_valid(&self) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_announce_incorrect() {
        assert!(
            Metainfo::find_announce(&hashmap![b"announce".to_vec() => BValue::Int(5)]).is_err(),
            "Incorrect or missing 'announce' value df"
        );
    }

    #[test]
    fn find_announce_ok() {
        assert_eq!(
            Metainfo::find_announce(
                &hashmap![b"announce".to_vec() => BValue::ByteStr(b"ANN".to_vec())]
            ),
            Ok(format!("ANN"))
        );
    }

    #[test]
    fn find_name_incorrect() {
        assert_eq!(
            Metainfo::find_name(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"name".to_vec() => BValue::Int(12)])]
            ),
            Err(Error::Meta("Incorrect or missing 'name' value".into()))
        );
    }

    #[test]
    fn find_name_incorrect_info() {
        assert_eq!(
            Metainfo::find_name(&hashmap![b"info".to_vec() => BValue::Int(12)]),
            Err(Error::Meta("Incorrect or missing 'info' value".into()))
        );
    }

    #[test]
    fn find_name_ok() {
        assert_eq!(
            Metainfo::find_name(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"name".to_vec() => BValue::ByteStr(b"INFO".to_vec())])]
            ),
            Ok(format!("INFO"))
        );
    }

    #[test]
    fn find_piece_length_incorrect() {
        assert_eq!(
            Metainfo::find_piece_length(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"piece length".to_vec() => BValue::ByteStr(b"BAD".to_vec())])]
            ),
            Err(Error::Meta("Incorrect or missing 'piece length' value".into()))
        );
    }

    #[test]
    fn find_piece_length_negative() {
        assert_eq!(
            Metainfo::find_piece_length(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"piece length".to_vec() => BValue::Int(-12)])]
            ),
            Err(Error::Meta("Can't convert 'piece length' to u64".into()))
        );
    }

    #[test]
    fn find_piece_length_incorrect_info() {
        assert_eq!(
            Metainfo::find_piece_length(&hashmap![b"info".to_vec() => BValue::Int(12)]),
            Err(Error::Meta("Incorrect or missing 'info' value".into()))
        );
    }

    #[test]
    fn find_piece_length_ok() {
        assert_eq!(
            Metainfo::find_piece_length(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"piece length".to_vec() => BValue::Int(12)])]
            ),
            Ok(12)
        );
    }

    #[test]
    fn find_pieces_incorrect() {
        assert_eq!(
            Metainfo::find_pieces(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"pieces".to_vec() => BValue::Int(12)])]
            ),
            Err(Error::Meta("Incorrect or missing 'pieces' value".into()))
        );
    }

    #[test]
    fn find_pieces_not_divisible() {
        assert_eq!(
            Metainfo::find_pieces(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"pieces".to_vec() => BValue::ByteStr(b"aaa".to_vec())])]
            ),
            Err(Error::Meta("'pieces' not divisible by 20".into()))
        );
    }

    #[test]
    fn find_pieces_incorrect_info() {
        assert_eq!(
            Metainfo::find_pieces(&hashmap![b"info".to_vec() => BValue::Int(12)]),
            Err(Error::Meta("Incorrect or missing 'info' value".into()))
        );
    }

    #[test]
    fn find_pieces_ok() {
        assert_eq!(
            Metainfo::find_pieces(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"pieces".to_vec() => BValue::ByteStr(b"aaaaabbbbbcccccdddddAAAAABBBBBCCCCCDDDDD".to_vec())])]
            ),
            Ok(vec![
                b"aaaaabbbbbcccccddddd".to_vec(),
                b"AAAAABBBBBCCCCCDDDDD".to_vec()
            ])
        );
    }

    #[test]
    fn find_length_incorrect() {
        assert_eq!(
            Metainfo::find_length(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"length".to_vec() => BValue::ByteStr(b"BAD".to_vec())])]
            ),
            None
        );
    }

    #[test]
    fn find_length_negative() {
        assert_eq!(
            Metainfo::find_length(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"length".to_vec() => BValue::Int(-12)])]
            ),
            None
        );
    }

    #[test]
    fn find_length_incorrect_info() {
        assert_eq!(
            Metainfo::find_length(&hashmap![b"info".to_vec() => BValue::Int(12)]),
            None
        );
    }

    #[test]
    fn find_length_ok() {
        assert_eq!(
            Metainfo::find_length(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"length".to_vec() => BValue::Int(12)])]
            ),
            Some(12)
        );
    }

    #[test]
    fn find_files_incorrect() {
        assert_eq!(
            Metainfo::find_files(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"files".to_vec() => BValue::ByteStr(b"BAD".to_vec())])]
            ),
            None
        );
    }

    #[test]
    fn find_files_empty_list() {
        assert_eq!(
            Metainfo::find_files(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"files".to_vec() => BValue::List(vec![])])]
            ),
            Some(vec![])
        );
    }

    #[test]
    fn find_files_invalid_dict() {
        assert_eq!(
            Metainfo::find_files(&hashmap![b"info".to_vec() =>
                BValue::Dict(hashmap![b"files".to_vec() =>
                    BValue::List(vec![
                        BValue::Dict(hashmap![b"a".to_vec() => BValue::Int(12),
                                              b"b".to_vec() => BValue::ByteStr(b"PATH".to_vec())])
                    ])
                ])
            ]),
            Some(vec![])
        );
    }

    #[test]
    fn find_files_invalid_dict_length() {
        assert_eq!(
            Metainfo::find_files(&hashmap![b"info".to_vec() =>
                BValue::Dict(hashmap![b"files".to_vec() =>
                    BValue::List(vec![
                        BValue::Dict(hashmap![b"length".to_vec() => BValue::Int(-12),
                                              b"path".to_vec() => BValue::ByteStr(b"PATH".to_vec())])
                    ])
                ])
            ]),
            Some(vec![])
        );
    }

    #[test]
    fn find_files_invalid_dict_path() {
        assert_eq!(
            Metainfo::find_files(&hashmap![b"info".to_vec() =>
                BValue::Dict(hashmap![b"files".to_vec() =>
                    BValue::List(vec![
                        BValue::Dict(hashmap![b"length".to_vec() => BValue::Int(1),
                                              b"path".to_vec() => BValue::Int(2)])
                    ])
                ])
            ]),
            Some(vec![])
        );
    }

    #[test]
    fn find_files_valid_and_invalid_dict() {
        assert_eq!(
            Metainfo::find_files(&hashmap![b"info".to_vec() =>
                BValue::Dict(hashmap![b"files".to_vec() =>
                    BValue::List(vec![
                        BValue::Dict(hashmap![b"length".to_vec() => BValue::Int(1),
                                              b"path".to_vec() => BValue::Int(2)]),
                        BValue::Dict(hashmap![b"length".to_vec() => BValue::Int(12),
                                              b"path".to_vec() => BValue::ByteStr(b"PATH".to_vec())]),
                    ])
                ])
            ]),
            Some(vec![File {
                length: 12,
                path: format!("PATH")
            }])
        );
    }

    #[test]
    fn length_only() {
        let torrent = Metainfo {
            announce: "URL".to_string(),
            name: "NAME".to_string(),
            piece_length: 999,
            pieces: vec![b"AAAAABBBBBCCCCCDDDDD".to_vec()],
            length: Some(111),
            files: None,
            hash: *b"AAAAABBBBBCCCCCDDDDD",
        };
        assert_eq!(torrent.is_valid(), true);
    }

    #[test]
    fn missing_length_and_files() {
        let torrent = Metainfo {
            announce: "URL".to_string(),
            name: "NAME".to_string(),
            piece_length: 999,
            pieces: vec![b"AAAAABBBBBCCCCCDDDDD".to_vec()],
            length: None,
            files: None,
            hash: *b"AAAAABBBBBCCCCCDDDDD",
        };
        assert_eq!(torrent.is_valid(), false);
    }

    #[test]
    fn files_only() {
        let torrent = Metainfo {
            announce: "URL".to_string(),
            name: "NAME".to_string(),
            piece_length: 999,
            pieces: vec![b"AAAAABBBBBCCCCCDDDDD".to_vec()],
            length: None,
            files: Some(vec![]),
            hash: *b"AAAAABBBBBCCCCCDDDDD",
        };
        assert_eq!(torrent.is_valid(), true);
    }

    #[test]
    fn both_length_and_files() {
        let torrent = Metainfo {
            announce: "URL".to_string(),
            name: "NAME".to_string(),
            piece_length: 999,
            pieces: vec![b"AAAAABBBBBCCCCCDDDDD".to_vec()],
            length: Some(111),
            files: Some(vec![]),
            hash: *b"AAAAABBBBBCCCCCDDDDD",
        };
        assert_eq!(torrent.is_valid(), false);
    }

    #[test]
    fn empty_input_incorrect() {
        assert_eq!(
            Metainfo::from_bencode(b""),
            Err(Error::Meta("Empty torrent".into()))
        );
    }

    #[test]
    fn incorrect_bencode() {
        assert_eq!(
            Metainfo::from_bencode(b"12"),
            Err(Error::Decode("ByteStr [0]: Not enough characters".into()))
        );
    }

    #[test]
    fn missing_announce() {
        assert_eq!(
            Metainfo::from_bencode(b"d8:announcei1ee"),
            Err(Error::Meta("Incorrect or missing 'announce' value".into()))
        );
    }

    #[test]
    fn torrent_incorrect() {
        assert_eq!(
            Metainfo::from_bencode(b"i12e"),
            Err(Error::Meta("Missing data".into()))
        );
    }

    #[test]
    fn torrent_missing_length_and_files() {
        assert_eq!(
            Metainfo::from_bencode(b"d8:announce3:URL4:infod4:name4:NAME12:piece lengthi999e6:pieces20:AAAAABBBBBCCCCCDDDDDee"),
            Err(Error::Meta("Conflicting values 'length' and 'files'. Only one is allowed".into()))
        );
    }

    #[test]
    fn torrent_correct() {
        assert_eq!(
            Metainfo::from_bencode(b"d8:announce3:URL4:infod4:name4:NAME12:piece lengthi999e6:pieces20:AAAAABBBBBCCCCCDDDDD6:lengthi111eee"),
            Ok(Metainfo {
                announce: "URL".to_string(),
                name : "NAME".to_string(),
                piece_length : 999,
                pieces : vec![b"AAAAABBBBBCCCCCDDDDD".to_vec()],
                length : Some(111),
                files: None,
                hash: *b"AAAAABBBBBCCCCCDDDDD",
            }));
    }
}
