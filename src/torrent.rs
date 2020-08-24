use crate::BValue;
use std::collections::HashMap;

#[derive(PartialEq, Debug)]
pub struct Torrent {
    bvalues : Vec<BValue>
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
    pub fn from_bytes(arg: &[u8]) -> Result<Torrent, String> {
        let bvalues = BValue::parse(arg)?;

        if bvalues.is_empty() {
            return Err(format!("Empty torrent"));
        }

        Ok(Torrent {
            bvalues
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        assert_eq!(Torrent::from_bytes(b""), Err(String::from("Empty torrent")));
    }

    #[test]
    fn incorrect_bencode() {
        assert_eq!(Torrent::from_bytes(b"12"), Err(String::from("ByteStr [0]: Not enough characters")));
    }
}