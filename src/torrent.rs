use crate::BValue;
use std::collections::HashMap;

#[derive(PartialEq, Debug)]
pub struct Torrent {
    announce : String,
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
                    match Self::create_torrent(dict) {
                        Ok(torrent) => return Ok(torrent),
                        Err(_) => ()
                    }
                },
                _ => ()
            }
        }

        Err(format!("Torrent data not found"))
    }

    fn create_torrent(dict : HashMap<Vec<u8>, BValue>) -> Result<Torrent, String> {
        Ok(Torrent{
            // announce : format!("asdf"),
            announce : Self::get_announce(dict)?,
        })
    }

    fn get_announce(dict : HashMap<Vec<u8>, BValue>) -> Result<String, String> {
        match dict.get(&b"announce".to_vec()) {
            Some(BValue::ByteStr(b)) => Ok(format!("asdf")),
            _ => Err(format!("aaa"))
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
    fn torrent1() {
        assert_eq!(Torrent::from_bencode(b"d8:announce3:abce"),
                   Err(String::from("ByteStr [0]: Not enough characters")));
    }

    #[test]
    fn torrent2() {
        assert_eq!(Torrent::from_bencode(b"d8:announcei1ee"),
                   Err(String::from("ByteStr [0]: Not enough characters")));
    }
}