use crate::BValue;
use std::collections::HashMap;
use super::hashmap;

#[derive(PartialEq, Debug)]
pub struct Torrent {
    announce : String,
    name : String,
    piece_length : i32,
    pieces : Vec<Vec<u8>>,
}

#[derive(PartialEq, Clone, Debug)]
pub struct Info {
    name : String,
    piece_length : i32,
    pieces : String,
    length : Option<i32>,
    files : Option<Vec<File>>,
}

#[derive(PartialEq, Clone, Debug)]
pub struct File {
    length : i32,
    path : Vec<String>,
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
                BValue::Dict(dict) => {
                    match Self::create_torrent(&dict) {
                        Ok(torrent) => return Ok(torrent),
                        Err(e) => err = Err(e)
                    }
                },
                _ => ()
            }
        }

        err
    }

    fn create_torrent(dict : &HashMap<Vec<u8>, BValue>) -> Result<Torrent, String> {
        Ok(Torrent{
            announce : Self::get_announce(dict)?,
            name : Self::get_name(dict)?,
            piece_length : Self::get_piece_length(dict)?,
            pieces : Self::get_pieces(dict)?,
        })
    }

    fn get_announce(dict : &HashMap<Vec<u8>, BValue>) -> Result<String, String> {
        match dict.get(&b"announce".to_vec()) {
            Some(BValue::ByteStr(val)) => match String::from_utf8(val.to_vec()) {
                Ok(s) => Ok(s),
                Err(_) => Err(format!("Can't convert 'announce' to UTF-8"))
            }
            _ => Err(format!("Incorrect or missing 'announce' value"))
        }
    }

    fn get_name(dict : &HashMap<Vec<u8>, BValue>) -> Result<String, String> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"name".to_vec()) {
               Some(BValue::ByteStr(val)) => match String::from_utf8(val.to_vec()) {
                    Ok(s) => Ok(s),
                    Err(_) => Err(format!("Can't convert 'name' to UTF-8"))
                }
                _ => Err(format!("Incorrect or missing 'name' value"))
            }
            _ => Err(format!("Incorrect or missing 'info/name') value"))
        }
    }

    fn get_piece_length(dict : &HashMap<Vec<u8>, BValue>) -> Result<i32, String> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"piece length".to_vec()) {
                Some(BValue::Int(length)) => Ok(*length),
                _ => Err(format!("Incorrect or missing 'piece length' value"))
            }
            _ => Err(format!("Incorrect or missing 'info/piece length' value"))
        }
    }

    fn get_pieces(dict : &HashMap<Vec<u8>, BValue>) -> Result<Vec<Vec<u8>>, String> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"pieces".to_vec()) {
                Some(BValue::ByteStr(pieces)) => {
                    if pieces.len() % 20 != 0 {
                        return Err(format!("'pieces' not divisible by 20"))
                    }
                    Ok(pieces.chunks(20).map(|chunk| chunk.to_vec()).collect())
                },
                _ => Err(format!("Incorrect or missing 'pieces' value"))
            }
            _ => Err(format!("Incorrect or missing 'info/pieces' value"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_announce_missing() {
        assert_eq!(Torrent::get_announce(&hashmap![b"announce".to_vec() => BValue::Int(5)]),
                   Err(String::from("Incorrect or missing 'announce' value")));
    }

    #[test]
    fn get_announce_ok() {
        assert_eq!(Torrent::get_announce(&hashmap![b"announce".to_vec() => BValue::ByteStr(b"asdf".to_vec())]),
                   Ok(format!("asdf")));
    }

    #[test]
    fn empty_input() {
        assert_eq!(Torrent::from_bencode(b""), Err(String::from("Empty torrent")));
    }

    #[test]
    fn incorrect_bencode() {
        assert_eq!(Torrent::from_bencode(b"12"), Err(String::from("ByteStr [0]: Not enough characters")));
    }

    #[test]
    fn missing_announce() {
        assert_eq!(Torrent::from_bencode(b"d8:announcei1ee"),
                   Err(String::from("Incorrect or missing 'announce' value")));
    }

    #[test]
    fn torrent_incorrect() {
        assert_eq!(Torrent::from_bencode(b"i12e"),
                   Err(String::from("Missing data")));
    }

    #[test]
    fn torrent_correct() {
        assert_eq!(Torrent::from_bencode(b"d8:announce3:URL4:infod4:name4:NAME12:piece lengthi999e6:pieces20:aaaaabbbbbcccccdddddee"),
                   Ok(Torrent {
                       announce: "URL".to_string(),
                       name : "NAME".to_string(),
                       piece_length : 999,
                       pieces : vec![b"aaaaabbbbbcccccddddd".to_vec()],
                   }));
    }
}