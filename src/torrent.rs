use crate::BValue;
use std::collections::HashMap;

#[derive(PartialEq, Debug)]
pub struct Torrent {
    announce : String,
    name : String,
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

        Err(format!("Torrent data not found"))
    }

    fn create_torrent(dict : &HashMap<Vec<u8>, BValue>) -> Option<Torrent> {
        Some(Torrent{
            announce : Self::get_announce(dict)?,
            name : Self::get_name(dict)?
        })
    }

    fn get_announce(dict : &HashMap<Vec<u8>, BValue>) -> Option<String> {
        match dict.get(&b"announce".to_vec()) {
            Some(BValue::ByteStr(b)) => String::from_utf8(b.to_vec()).ok(),
            _ => None
        }
    }

    fn get_name(dict : &HashMap<Vec<u8>, BValue>) -> Option<String> {
        match dict.get(&b"info".to_vec()) {
            Some(BValue::Dict(info)) => match info.get(&b"name".to_vec()) {
                Some(BValue::ByteStr(b)) => String::from_utf8(b.to_vec()).ok(),
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
                   Err(String::from("Torrent data not found")));
    }

    #[test]
    fn torrent_correct() {
        assert_eq!(Torrent::from_bencode(b"d8:announce3:URL4:infod4:name4:NAMEee"),
                   Ok(Torrent {
                       announce: "URL".to_string(),
                       name : "NAME".to_string(),
                   }));
    }
}