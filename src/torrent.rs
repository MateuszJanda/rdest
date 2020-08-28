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
                    return Ok(Torrent{
                        announce : Self::get_announce(dict)?,
                    })
                },
                _ => ()
            }
        }

        Err(format!("Torrent data not found"))
    }

    fn get_announce(dict : HashMap<Vec<u8>, BValue>) -> Result<String, String> {
        Err(format!("nope"))
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
}