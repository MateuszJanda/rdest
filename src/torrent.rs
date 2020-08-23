use crate::BValue;
use std::collections::HashMap;

#[derive(PartialEq, Debug)]
pub struct Torrent {
    announce : String,
    info : Info,
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
        for val in BValue::parse(arg)? {
            if let BValue::Dict(d) = val {

            }
        }
        Err(format!("Nope"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        assert_eq!(Torrent::from_bytes(b""), Err(String::from("Nope")));
    }

    #[test]
    fn incorrect_bencode() {
        assert_eq!(Torrent::from_bytes(b"12"), Err(String::from("ByteStr [0]: Not enough characters")));
    }
}