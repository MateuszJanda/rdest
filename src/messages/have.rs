use crate::constants::{MSG_ID_SIZE, MSG_LEN_SIZE};
use crate::serializer::Serializer;
use crate::Error;
use std::io::Cursor;

#[derive(Debug)]
pub struct Have {
    piece_index: u32,
}

impl Have {
    const LEN: u32 = 5;
    pub const ID: u8 = 4;
    const LEN_SIZE: usize = MSG_LEN_SIZE;
    const ID_SIZE: usize = MSG_ID_SIZE;
    const INDEX_SIZE: usize = 4;
    const FULL_SIZE: usize = Have::LEN_SIZE + Have::LEN as usize;

    pub fn new(piece_index: usize) -> Have {
        Have {
            piece_index: piece_index as u32,
        }
    }

    pub fn from(crs: &Cursor<&[u8]>) -> Have {
        let start = Have::LEN_SIZE + Have::ID_SIZE;
        let mut piece_index = [0; Have::INDEX_SIZE];
        piece_index.copy_from_slice(&crs.get_ref()[start..start + Have::INDEX_SIZE]);

        Have {
            piece_index: u32::from_be_bytes(piece_index),
        }
    }

    pub fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        match length == Have::LEN as usize && available_data >= Have::LEN_SIZE + length {
            true => Ok(Have::FULL_SIZE),
            false => Err(Error::Incomplete("Have")),
        }
    }

    pub fn piece_index(&self) -> usize {
        self.piece_index as usize
    }

    pub fn validate(&self, pieces_num: usize) -> Result<(), Error> {
        match (self.piece_index as usize) < pieces_num {
            true => Ok(()),
            false => Err(Error::InvalidPieceIndex("Have")),
        }
    }
}

impl Serializer for Have {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&Have::LEN.to_be_bytes());
        vec.push(Have::ID);
        vec.extend_from_slice(&self.piece_index.to_be_bytes());

        vec
    }
}
