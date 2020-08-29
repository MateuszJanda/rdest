use crate::BValue;
use std::collections::HashMap;

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

        for val in bvalues {
            match val {
                BValue::Dict(dict) => {
                    match Self::create_torrent(&dict) {
                        Some(torrent) => return Ok(torrent),
                        None => ()
                    }
                },
                _ => ()
            }
        }

        Err(format!("Missing data"))
    }

    fn create_torrent(dict : &HashMap<Vec<u8>, BValue>) -> Option<Torrent> {
        Some(Torrent{
            announce : Self::get_announce(dict)?,
            name : Self::get_name(dict)?,
            piece_length : Self::get_piece_length(dict)?,
            pieces : Self::get_pieces(dict)?,
        })
    }

    fn get_announce(dict : &HashMap<Vec<u8>, BValue>) -> Option<String> {
        match dict.get(&b"announce".to_vec()) {
            Some(BValue::ByteStr(val)) => String::from_utf8(val.to_vec()).ok(),
            _ => None
        }
    }

    fn get_name(dict : &HashMap<Vec<u8>, BValue>) -> Option<String> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"name".to_vec()) {
                Some(BValue::ByteStr(val)) => String::from_utf8(val.to_vec()).ok(),
                _ => None
            }
            _ => None
        }
    }

    fn get_piece_length(dict : &HashMap<Vec<u8>, BValue>) -> Option<i32> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"piece length".to_vec()) {
                Some(BValue::Int(length)) => Some(*length),
                _ => None
            }
            _ => None
        }
    }

    fn get_pieces(dict : &HashMap<Vec<u8>, BValue>) -> Option<Vec<Vec<u8>>> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"pieces".to_vec()) {
                Some(BValue::ByteStr(pieces)) => {
                    if pieces.len() % 20 != 0 {
                        return None
                    }
                    None
                },
                _ => None
            }
            _ => None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        assert_eq!(Torrent::from_bencode(b""), Err(String::from("Empty torrent")));
    }

    #[test]
    fn incorrect_bencode() {
        assert_eq!(Torrent::from_bencode(b"12"), Err(String::from("ByteStr [0]: Not enough characters")));
    }

    #[test]
    fn torrent_incorrect_announce() {
        assert_eq!(Torrent::from_bencode(b"d8:announcei1ee"),
                   Err(String::from("Missing data")));
    }

    #[test]
    fn torrent_correct() {
        assert_eq!(Torrent::from_bencode(b"d8:announce3:URL4:infod4:name4:NAME12:piece lengthi999eee"),
                   Ok(Torrent {
                       announce: "URL".to_string(),
                       name : "NAME".to_string(),
                       piece_length : 999,
                       pieces : vec![b"aaaaabbbbbcccccddddd".to_vec()],
                   }));
    }
}