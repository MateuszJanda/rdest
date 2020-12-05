use crate::constant::{MSG_ID_SIZE, MSG_LEN_SIZE};
use crate::serializer::Serializer;
use crate::Error;
use std::io::Cursor;

#[derive(Debug)]
pub struct Piece {
    index: u32,
    block_begin: u32,
    block: Vec<u8>,
}

impl Piece {
    pub const ID: u8 = 7;
    const LEN_SIZE: usize = MSG_LEN_SIZE;
    const ID_SIZE: usize = MSG_ID_SIZE;
    const INDEX_SIZE: usize = 4;
    const BEGIN_SIZE: usize = 4;
    const MIN_LEN: usize = 9;

    pub fn new(index: usize, block_begin: usize, block: Vec<u8>) -> Piece {
        Piece {
            index: index as u32,
            block_begin: block_begin as u32,
            block,
        }
    }

    pub fn from(crs: &Cursor<&[u8]>) -> Piece {
        let start = Piece::LEN_SIZE + Piece::ID_SIZE;
        let mut index = [0; Piece::INDEX_SIZE];
        index.copy_from_slice(&crs.get_ref()[start..start + Piece::INDEX_SIZE]);

        let start = start + Piece::INDEX_SIZE;
        let mut block_begin = [0; Piece::BEGIN_SIZE];
        block_begin.copy_from_slice(&crs.get_ref()[start..start + Piece::BEGIN_SIZE]);

        let start = start + Piece::BEGIN_SIZE;
        let end = crs.position() as usize;
        let mut block = vec![];
        block.extend_from_slice(&crs.get_ref()[start..end]);

        Piece {
            index: u32::from_be_bytes(index),
            block_begin: u32::from_be_bytes(block_begin),
            block,
        }
    }

    pub fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        if length >= Piece::MIN_LEN && available_data >= Piece::LEN_SIZE + length {
            return Ok(Piece::LEN_SIZE + length);
        }

        Err(Error::Incomplete("Piece".into()))
    }

    pub fn block_begin(&self) -> usize {
        self.block_begin as usize
    }

    pub fn block_length(&self) -> usize {
        self.block.len()
    }

    pub fn block(&self) -> &Vec<u8> {
        &self.block
    }

    pub fn validate(
        &self,
        index: usize,
        block_begin: usize,
        block_length: usize,
    ) -> Result<(), Error> {
        if self.index as usize != index {
            return Err(Error::InvalidIndex("Piece".into()));
        }

        if self.block_begin as usize != block_begin {
            return Err(Error::InvalidIndex("Piece".into()));
        }

        if self.block.len() as usize != block_length {
            return Err(Error::InvalidLength("Piece".into()));
        }

        Ok(())
    }
}

impl Serializer for Piece {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&((Piece::ID_SIZE + self.block.len()) as u32).to_be_bytes());
        vec.push(Piece::ID);
        vec.extend_from_slice(&self.index.to_be_bytes());
        vec.extend_from_slice(&self.block_begin.to_be_bytes());
        vec.extend_from_slice(self.block.as_slice());

        vec
    }
}
