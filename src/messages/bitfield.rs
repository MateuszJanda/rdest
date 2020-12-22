use crate::constants::{MSG_ID_SIZE, MSG_LEN_SIZE};
use crate::serializer::Serializer;
use crate::Error;
use std::io::Cursor;

#[derive(Debug)]
pub struct Bitfield {
    pieces_bytes: Vec<u8>,
}

impl Bitfield {
    pub const ID: u8 = 5;
    const LEN_SIZE: usize = MSG_LEN_SIZE;
    const ID_SIZE: usize = MSG_ID_SIZE;
    const BITS_IN_BYTE: usize = 8;
    const BYTE_MASK: u8 = 0b1000_0000;

    pub fn from_vec(pieces: &Vec<bool>) -> Bitfield {
        let mut pieces_bytes = vec![];

        for piece in pieces.chunks(Bitfield::BITS_IN_BYTE) {
            let mut byte: u8 = 0;
            for (idx, present) in piece.iter().enumerate() {
                if *present {
                    byte |= Bitfield::BYTE_MASK >> idx;
                }
            }

            pieces_bytes.push(byte);
        }

        Bitfield { pieces_bytes }
    }

    pub fn from(crs: &Cursor<&[u8]>) -> Bitfield {
        let start = Bitfield::LEN_SIZE + Bitfield::ID_SIZE;
        let end = crs.position() as usize;
        let mut pieces_bytes = vec![];
        pieces_bytes.extend_from_slice(&crs.get_ref()[start..end]);

        Bitfield { pieces_bytes }
    }

    pub fn to_vec(&self, pieces_num: usize) -> Result<Vec<bool>, Error> {
        let bytes_num = match pieces_num % Bitfield::BITS_IN_BYTE == 0 {
            true => pieces_num / Bitfield::BITS_IN_BYTE,
            false => pieces_num / Bitfield::BITS_IN_BYTE + 1,
        };

        if self.pieces_bytes.len() != bytes_num {
            return Err(Error::InvalidLength("Bitfield".into()));
        }

        let mut pieces = vec![];
        for b in self.pieces_bytes.iter() {
            let mut byte = *b;
            for _ in 0..Bitfield::BITS_IN_BYTE {
                pieces.push(byte & Bitfield::BYTE_MASK != 0);
                byte = byte << 1;

                if pieces.len() == pieces_num {
                    return Ok(pieces);
                }
            }
        }

        Ok(pieces)
    }

    pub fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        match available_data >= Bitfield::LEN_SIZE + length {
            true => Ok(Bitfield::LEN_SIZE + length),
            false => Err(Error::Incomplete("Bitfield".into())),
        }
    }

    pub fn validate(&self, pieces_num: usize) -> Result<(), Error> {
        let bytes_num = match pieces_num % Bitfield::BITS_IN_BYTE == 0 {
            true => pieces_num / Bitfield::BITS_IN_BYTE,
            false => pieces_num / Bitfield::BITS_IN_BYTE + 1,
        };

        match self.pieces_bytes.len() == bytes_num {
            true => Ok(()),
            false => Err(Error::InvalidLength("Bitfield".into())),
        }
    }
}

impl Serializer for Bitfield {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(
            &((Bitfield::ID_SIZE + self.pieces_bytes.len()) as u32).to_be_bytes(),
        );
        vec.push(Bitfield::ID);
        vec.extend_from_slice(self.pieces_bytes.as_slice());

        vec
    }
}
