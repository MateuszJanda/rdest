use crate::BValue;

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
}