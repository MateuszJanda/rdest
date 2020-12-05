use crate::constant::{MSG_ID_SIZE, MSG_LEN_SIZE};
use crate::serializer::Serializer;
use crate::Error;
use std::io::Cursor;

#[derive(Debug)]
pub struct Bitfield {
    pieces: Vec<u8>,
}

impl Bitfield {
    pub const ID: u8 = 5;
    const LEN_SIZE: usize = MSG_LEN_SIZE;
    const ID_SIZE: usize = MSG_ID_SIZE;

    pub fn from_vec(pieces: &Vec<bool>) -> Bitfield {
        let mut v = vec![];

        for p in pieces.chunks(8) {
            let mut byte: u8 = 0;
            for (idx, vv) in p.iter().enumerate() {
                if *vv {
                    byte |= 0b1000_0000 >> idx;
                }
            }

            v.push(byte);
        }

        Bitfield { pieces: v }
    }

    pub fn from(crs: &Cursor<&[u8]>) -> Bitfield {
        let start = Bitfield::LEN_SIZE + Bitfield::ID_SIZE;
        let end = crs.position() as usize;
        let mut pieces = vec![];
        pieces.extend_from_slice(&crs.get_ref()[start..end]);

        Bitfield { pieces }
    }

    pub fn to_vec(&self) -> Vec<bool> {
        let mut pieces = vec![];
        for b in self.pieces.iter() {
            let mut byte = *b;
            for _ in 0..8 {
                if byte & 0b1000_0000 != 0 {
                    pieces.push(true);
                } else {
                    pieces.push(false);
                }

                byte = byte << 1;
            }
        }

        pieces
    }

    pub fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        if available_data >= Bitfield::LEN_SIZE + length {
            return Ok(Bitfield::LEN_SIZE + length);
        }

        Err(Error::Incomplete("Bitfield".into()))
    }

    pub fn validate(&self, pieces_num: usize) -> Result<(), Error> {
        if self.to_vec().len() != pieces_num {
            return Err(Error::InvalidLength("Bitfield".into()));
        }

        Ok(())
    }
}

impl Serializer for Bitfield {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&((Bitfield::ID_SIZE + self.pieces.len()) as u32).to_be_bytes());
        vec.push(Bitfield::ID);
        vec.extend_from_slice(self.pieces.as_slice());

        vec
    }
}
