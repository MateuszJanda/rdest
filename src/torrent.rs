#[cfg(test)]
use super::hashmap;
use crate::BValue;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs;

extern crate sha1;

#[derive(PartialEq, Debug)]
pub struct Torrent {
    announce: String,
    name: String,
    piece_length: u64,
    pieces: Vec<Vec<u8>>,
    length: Option<u64>,
    files: Option<Vec<File>>,
}

#[derive(PartialEq, Clone, Debug)]
pub struct File {
    length: u64,
    path: String,
}

impl Torrent {
    pub fn from_file(path: String) -> Result<Torrent, String> {
        match &fs::read(path) {
            Ok(val) => Self::from_bencode(val),
            Err(_) => Err(format!("File not found")),
        }
    }

    pub fn from_bencode(arg: &[u8]) -> Result<Torrent, String> {
        let bvalues = BValue::parse(arg)?;
        // let raw_info = BValue::cut_raw_info(arg)?;

        if bvalues.is_empty() {
            return Err(format!("Empty torrent"));
        }

        let mut err = Err(format!("Missing data"));
        for val in bvalues {
            match val {
                BValue::Dict(dict) => match Self::create_torrent(&dict) {
                    Ok(torrent) => return Ok(torrent),
                    Err(e) => err = Err(e),
                },
                _ => (),
            }
        }

        err
    }

    fn create_torrent(dict: &HashMap<Vec<u8>, BValue>) -> Result<Torrent, String> {
        let torrent = Torrent {
            announce: Self::find_announce(dict)?,
            name: Self::find_name(dict)?,
            piece_length: Self::find_piece_length(dict)?,
            pieces: Self::find_pieces(dict)?,
            length: Self::find_length(dict),
            files: Self::find_files(dict),
        };

        if !torrent.is_valid() {
            return Err(format!(
                "Conflicting values 'length' and 'files'. Only one is allowed"
            ));
        }


        Ok(torrent)
    }

    fn find_announce(dict: &HashMap<Vec<u8>, BValue>) -> Result<String, String> {
        match dict.get(&b"announce".to_vec()) {
            Some(BValue::ByteStr(val)) => String::from_utf8(val.to_vec())
                .or(Err(format!("Can't convert 'announce' to UTF-8"))),
            _ => Err(format!("Incorrect or missing 'announce' value")),
        }
    }

    fn find_name(dict: &HashMap<Vec<u8>, BValue>) -> Result<String, String> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"name".to_vec()) {
                Some(BValue::ByteStr(val)) => String::from_utf8(val.to_vec())
                    .or(Err(format!("Can't convert 'name' to UTF-8"))),
                _ => Err(format!("Incorrect or missing 'name' value")),
            },
            _ => Err(format!("Incorrect or missing 'info' value")),
        }
    }

    fn find_piece_length(dict: &HashMap<Vec<u8>, BValue>) -> Result<u64, String> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"piece length".to_vec()) {
                Some(BValue::Int(length)) => {
                    u64::try_from(*length).or(Err(format!("Can't convert 'piece length' to u64")))
                }
                _ => Err(format!("Incorrect or missing 'piece length' value")),
            },
            _ => Err(format!("Incorrect or missing 'info' value")),
        }
    }

    fn find_pieces(dict: &HashMap<Vec<u8>, BValue>) -> Result<Vec<Vec<u8>>, String> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"pieces".to_vec()) {
                Some(BValue::ByteStr(pieces)) => {
                    if pieces.len() % 20 != 0 {
                        return Err(format!("'pieces' not divisible by 20"));
                    }
                    Ok(pieces.chunks(20).map(|chunk| chunk.to_vec()).collect())
                }
                _ => Err(format!("Incorrect or missing 'pieces' value")),
            },
            _ => Err(format!("Incorrect or missing 'info' value")),
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
                Some(BValue::List(list)) => Some(Self::find_files_list(list)),
                _ => None,
            },
            _ => None,
        }
    }

    fn find_files_list(list: &Vec<BValue>) -> Vec<File> {
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

    pub fn get_url(&self) -> String {
        self.announce.clone()
    }

    pub fn get_info_hash(&self) -> String {

        let mut m = sha1::Sha1::new();

        let v: Vec<u8> = vec![1, 2, 3];

        // m.update(b"Hello World!");
        m.update(v.as_ref());
        println!("{:?}", m.digest().to_string());
        String::from("asdf")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_announce_incorrect() {
        assert_eq!(
            Torrent::find_announce(&hashmap![b"announce".to_vec() => BValue::Int(5)]),
            Err(String::from("Incorrect or missing 'announce' value"))
        );
    }

    #[test]
    fn find_announce_ok() {
        assert_eq!(
            Torrent::find_announce(
                &hashmap![b"announce".to_vec() => BValue::ByteStr(b"ANN".to_vec())]
            ),
            Ok(format!("ANN"))
        );
    }

    #[test]
    fn find_name_incorrect() {
        assert_eq!(
            Torrent::find_name(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"name".to_vec() => BValue::Int(12)])]
            ),
            Err(String::from("Incorrect or missing 'name' value"))
        );
    }

    #[test]
    fn find_name_incorrect_info() {
        assert_eq!(
            Torrent::find_name(&hashmap![b"info".to_vec() => BValue::Int(12)]),
            Err(String::from("Incorrect or missing 'info' value"))
        );
    }

    #[test]
    fn find_name_ok() {
        assert_eq!(
            Torrent::find_name(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"name".to_vec() => BValue::ByteStr(b"INFO".to_vec())])]
            ),
            Ok(format!("INFO"))
        );
    }

    #[test]
    fn find_piece_length_incorrect() {
        assert_eq!(
            Torrent::find_piece_length(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"piece length".to_vec() => BValue::ByteStr(b"BAD".to_vec())])]
            ),
            Err(String::from("Incorrect or missing 'piece length' value"))
        );
    }

    #[test]
    fn find_piece_length_negative() {
        assert_eq!(
            Torrent::find_piece_length(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"piece length".to_vec() => BValue::Int(-12)])]
            ),
            Err(String::from("Can't convert 'piece length' to u64"))
        );
    }

    #[test]
    fn find_piece_length_incorrect_info() {
        assert_eq!(
            Torrent::find_piece_length(&hashmap![b"info".to_vec() => BValue::Int(12)]),
            Err(String::from("Incorrect or missing 'info' value"))
        );
    }

    #[test]
    fn find_piece_length_ok() {
        assert_eq!(
            Torrent::find_piece_length(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"piece length".to_vec() => BValue::Int(12)])]
            ),
            Ok(12)
        );
    }

    #[test]
    fn find_pieces_incorrect() {
        assert_eq!(
            Torrent::find_pieces(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"pieces".to_vec() => BValue::Int(12)])]
            ),
            Err(String::from("Incorrect or missing 'pieces' value"))
        );
    }

    #[test]
    fn find_pieces_not_divisible() {
        assert_eq!(
            Torrent::find_pieces(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"pieces".to_vec() => BValue::ByteStr(b"aaa".to_vec())])]
            ),
            Err(String::from("'pieces' not divisible by 20"))
        );
    }

    #[test]
    fn find_pieces_incorrect_info() {
        assert_eq!(
            Torrent::find_pieces(&hashmap![b"info".to_vec() => BValue::Int(12)]),
            Err(String::from("Incorrect or missing 'info' value"))
        );
    }

    #[test]
    fn find_pieces_ok() {
        assert_eq!(
            Torrent::find_pieces(
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
            Torrent::find_length(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"length".to_vec() => BValue::ByteStr(b"BAD".to_vec())])]
            ),
            None
        );
    }

    #[test]
    fn find_length_negative() {
        assert_eq!(
            Torrent::find_length(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"length".to_vec() => BValue::Int(-12)])]
            ),
            None
        );
    }

    #[test]
    fn find_length_incorrect_info() {
        assert_eq!(
            Torrent::find_length(&hashmap![b"info".to_vec() => BValue::Int(12)]),
            None
        );
    }

    #[test]
    fn find_length_ok() {
        assert_eq!(
            Torrent::find_length(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"length".to_vec() => BValue::Int(12)])]
            ),
            Some(12)
        );
    }

    #[test]
    fn find_files_incorrect() {
        assert_eq!(
            Torrent::find_files(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"files".to_vec() => BValue::ByteStr(b"BAD".to_vec())])]
            ),
            None
        );
    }

    #[test]
    fn find_files_empty_list() {
        assert_eq!(
            Torrent::find_files(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"files".to_vec() => BValue::List(vec![])])]
            ),
            Some(vec![])
        );
    }

    #[test]
    fn find_files_invalid_dict() {
        assert_eq!(
            Torrent::find_files(&hashmap![b"info".to_vec() =>
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
            Torrent::find_files(&hashmap![b"info".to_vec() =>
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
            Torrent::find_files(&hashmap![b"info".to_vec() =>
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
            Torrent::find_files(&hashmap![b"info".to_vec() =>
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
        let torrent = Torrent {
            announce: "URL".to_string(),
            name: "NAME".to_string(),
            piece_length: 999,
            pieces: vec![b"AAAAABBBBBCCCCCDDDDD".to_vec()],
            length: Some(111),
            files: None,
        };
        assert_eq!(torrent.is_valid(), true);
    }

    #[test]
    fn missing_length_and_files() {
        let torrent = Torrent {
            announce: "URL".to_string(),
            name: "NAME".to_string(),
            piece_length: 999,
            pieces: vec![b"AAAAABBBBBCCCCCDDDDD".to_vec()],
            length: None,
            files: None,
        };
        assert_eq!(torrent.is_valid(), false);
    }

    #[test]
    fn files_only() {
        let torrent = Torrent {
            announce: "URL".to_string(),
            name: "NAME".to_string(),
            piece_length: 999,
            pieces: vec![b"AAAAABBBBBCCCCCDDDDD".to_vec()],
            length: None,
            files: Some(vec![]),
        };
        assert_eq!(torrent.is_valid(), true);
    }

    #[test]
    fn both_length_and_files() {
        let torrent = Torrent {
            announce: "URL".to_string(),
            name: "NAME".to_string(),
            piece_length: 999,
            pieces: vec![b"AAAAABBBBBCCCCCDDDDD".to_vec()],
            length: Some(111),
            files: Some(vec![]),
        };
        assert_eq!(torrent.is_valid(), false);
    }

    #[test]
    fn empty_input_incorrect() {
        assert_eq!(
            Torrent::from_bencode(b""),
            Err(String::from("Empty torrent"))
        );
    }

    #[test]
    fn incorrect_bencode() {
        assert_eq!(
            Torrent::from_bencode(b"12"),
            Err(String::from("ByteStr [0]: Not enough characters"))
        );
    }

    #[test]
    fn missing_announce() {
        assert_eq!(
            Torrent::from_bencode(b"d8:announcei1ee"),
            Err(String::from("Incorrect or missing 'announce' value"))
        );
    }

    #[test]
    fn torrent_incorrect() {
        assert_eq!(
            Torrent::from_bencode(b"i12e"),
            Err(String::from("Missing data"))
        );
    }

    #[test]
    fn torrent_missing_length_and_files() {
        assert_eq!(
            Torrent::from_bencode(b"d8:announce3:URL4:infod4:name4:NAME12:piece lengthi999e6:pieces20:AAAAABBBBBCCCCCDDDDDee"),
            Err(String::from("Conflicting values 'length' and 'files'. Only one is allowed"))
        );
    }

    #[test]
    fn torrent_correct() {
        assert_eq!(
            Torrent::from_bencode(b"d8:announce3:URL4:infod4:name4:NAME12:piece lengthi999e6:pieces20:AAAAABBBBBCCCCCDDDDD6:lengthi111eee"),
            Ok(Torrent {
                announce: "URL".to_string(),
                name : "NAME".to_string(),
                piece_length : 999,
                pieces : vec![b"AAAAABBBBBCCCCCDDDDD".to_vec()],
                length : Some(111),
                files: None,
            }));
    }
}
