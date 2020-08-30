use super::hashmap;
use crate::BValue;
use std::collections::HashMap;
use std::convert::TryFrom;

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
    pub fn from_bencode(arg: &[u8]) -> Result<Torrent, String> {
        let bvalues = BValue::parse(arg)?;

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
        Ok(Torrent {
            announce: Self::get_announce(dict)?,
            name: Self::get_name(dict)?,
            piece_length: Self::get_piece_length(dict)?,
            pieces: Self::get_pieces(dict)?,
            length: Self::get_length(dict),
            files: Self::get_files(dict),
        })
    }

    fn get_announce(dict: &HashMap<Vec<u8>, BValue>) -> Result<String, String> {
        match dict.get(&b"announce".to_vec()) {
            Some(BValue::ByteStr(val)) => String::from_utf8(val.to_vec())
                .or(Err(format!("Can't convert 'announce' to UTF-8"))),
            _ => Err(format!("Incorrect or missing 'announce' value")),
        }
    }

    fn get_name(dict: &HashMap<Vec<u8>, BValue>) -> Result<String, String> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"name".to_vec()) {
                Some(BValue::ByteStr(val)) => String::from_utf8(val.to_vec())
                    .or(Err(format!("Can't convert 'name' to UTF-8"))),
                _ => Err(format!("Incorrect or missing 'name' value")),
            },
            _ => Err(format!("Incorrect or missing 'info' value")),
        }
    }

    fn get_piece_length(dict: &HashMap<Vec<u8>, BValue>) -> Result<u64, String> {
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

    fn get_pieces(dict: &HashMap<Vec<u8>, BValue>) -> Result<Vec<Vec<u8>>, String> {
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

    fn get_length(dict: &HashMap<Vec<u8>, BValue>) -> Option<u64> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"length".to_vec()) {
                Some(BValue::Int(length)) => u64::try_from(*length).ok(),
                _ => None,
            },
            _ => None,
        }
    }

    fn get_files(dict: &HashMap<Vec<u8>, BValue>) -> Option<Vec<File>> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"files".to_vec()) {
                Some(BValue::List(list)) => Some(Self::get_files_list(list)),
                _ => None,
            },
            _ => None,
        }
    }

    fn get_files_list(list: &Vec<BValue>) -> Vec<File> {
        let mut res = vec![];
        for elem in list {
            match elem {
                BValue::Dict(dict) => {
                    match (dict.get(&b"length".to_vec()), dict.get(&b"path".to_vec())) {
                        (Some(BValue::Int(length)), Some(BValue::ByteStr(path))) => {
                            match (u64::try_from(*length), String::from_utf8(path.to_vec())) {
                                (Ok(l), Ok(p)) => res.push(File { length: l, path: p }),
                                _ => (),
                            }
                        }
                        _ => (),
                    }
                }
                _ => (),
            }
        }

        return res;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_announce_incorrect() {
        assert_eq!(
            Torrent::get_announce(&hashmap![b"announce".to_vec() => BValue::Int(5)]),
            Err(String::from("Incorrect or missing 'announce' value"))
        );
    }

    #[test]
    fn get_announce_ok() {
        assert_eq!(
            Torrent::get_announce(
                &hashmap![b"announce".to_vec() => BValue::ByteStr(b"ANN".to_vec())]
            ),
            Ok(format!("ANN"))
        );
    }

    #[test]
    fn get_name_incorrect() {
        assert_eq!(
            Torrent::get_name(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"name".to_vec() => BValue::Int(12)])]
            ),
            Err(String::from("Incorrect or missing 'name' value"))
        );
    }

    #[test]
    fn get_name_incorrect_info() {
        assert_eq!(
            Torrent::get_name(&hashmap![b"info".to_vec() => BValue::Int(12)]),
            Err(String::from("Incorrect or missing 'info' value"))
        );
    }

    #[test]
    fn get_name_ok() {
        assert_eq!(
            Torrent::get_name(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"name".to_vec() => BValue::ByteStr(b"INFO".to_vec())])]
            ),
            Ok(format!("INFO"))
        );
    }

    #[test]
    fn get_piece_length_incorrect() {
        assert_eq!(
            Torrent::get_piece_length(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"piece length".to_vec() => BValue::ByteStr(b"BAD".to_vec())])]
            ),
            Err(String::from("Incorrect or missing 'piece length' value"))
        );
    }

    #[test]
    fn get_piece_length_negative() {
        assert_eq!(
            Torrent::get_piece_length(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"piece length".to_vec() => BValue::Int(-12)])]
            ),
            Err(String::from("Can't convert 'piece length' to u64"))
        );
    }

    #[test]
    fn get_piece_length_incorrect_info() {
        assert_eq!(
            Torrent::get_piece_length(&hashmap![b"info".to_vec() => BValue::Int(12)]),
            Err(String::from("Incorrect or missing 'info' value"))
        );
    }

    #[test]
    fn get_piece_length_ok() {
        assert_eq!(
            Torrent::get_piece_length(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"piece length".to_vec() => BValue::Int(12)])]
            ),
            Ok(12)
        );
    }

    #[test]
    fn get_pieces_incorrect() {
        assert_eq!(
            Torrent::get_pieces(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"pieces".to_vec() => BValue::Int(12)])]
            ),
            Err(String::from("Incorrect or missing 'pieces' value"))
        );
    }

    #[test]
    fn get_pieces_not_divisible() {
        assert_eq!(
            Torrent::get_pieces(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"pieces".to_vec() => BValue::ByteStr(b"aaa".to_vec())])]
            ),
            Err(String::from("'pieces' not divisible by 20"))
        );
    }

    #[test]
    fn get_pieces_incorrect_info() {
        assert_eq!(
            Torrent::get_pieces(&hashmap![b"info".to_vec() => BValue::Int(12)]),
            Err(String::from("Incorrect or missing 'info' value"))
        );
    }

    #[test]
    fn get_pieces_ok() {
        assert_eq!(
            Torrent::get_pieces(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"pieces".to_vec() => BValue::ByteStr(b"aaaaabbbbbcccccdddddAAAAABBBBBCCCCCDDDDD".to_vec())])]
            ),
            Ok(vec![
                b"aaaaabbbbbcccccddddd".to_vec(),
                b"AAAAABBBBBCCCCCDDDDD".to_vec()
            ])
        );
    }

    #[test]
    fn get_length_incorrect() {
        assert_eq!(
            Torrent::get_length(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"length".to_vec() => BValue::ByteStr(b"BAD".to_vec())])]
            ),
            None
        );
    }

    #[test]
    fn get_length_negative() {
        assert_eq!(
            Torrent::get_length(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"length".to_vec() => BValue::Int(-12)])]
            ),
            None
        );
    }

    #[test]
    fn get_length_incorrect_info() {
        assert_eq!(
            Torrent::get_length(&hashmap![b"info".to_vec() => BValue::Int(12)]),
            None
        );
    }

    #[test]
    fn get_length_ok() {
        assert_eq!(
            Torrent::get_length(
                &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"length".to_vec() => BValue::Int(12)])]
            ),
            Some(12)
        );
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
    fn torrent_correct() {
        assert_eq!(
            Torrent::from_bencode(b"d8:announce3:URL4:infod4:name4:NAME12:piece lengthi999e6:pieces20:aaaaabbbbbcccccdddddee"),
            Ok(Torrent {
                announce: "URL".to_string(),
                name : "NAME".to_string(),
                piece_length : 999,
                pieces : vec![b"aaaaabbbbbcccccddddd".to_vec()],
                length : None,
                files: None,
            }));
    }
}
