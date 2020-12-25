use crate::constants::{MSG_ID_SIZE, MSG_LEN_SIZE, PIECE_BLOCK_SIZE};
use crate::serializer::Serializer;
use crate::Error;
use std::io::Cursor;

#[derive(Debug)]
pub struct Request {
    piece_index: u32,
    block_begin: u32,
    block_length: u32,
}

impl Request {
    const LEN: u32 = 13;
    pub const ID: u8 = 6;
    const LEN_SIZE: usize = MSG_LEN_SIZE;
    const ID_SIZE: usize = MSG_ID_SIZE;
    const INDEX_SIZE: usize = 4;
    const BEGIN_SIZE: usize = 4;
    const LENGTH_SIZE: usize = 4;
    const FULL_SIZE: usize = Request::LEN_SIZE + Request::LEN as usize;

    pub fn new(piece_index: usize, block_begin: usize, block_length: usize) -> Request {
        Request {
            piece_index: piece_index as u32,
            block_begin: block_begin as u32,
            block_length: block_length as u32,
        }
    }

    pub fn from(crs: &Cursor<&[u8]>) -> Request {
        let start = Request::LEN_SIZE + Request::ID_SIZE;
        let mut piece_index = [0; Request::INDEX_SIZE];
        piece_index.copy_from_slice(&crs.get_ref()[start..start + Request::INDEX_SIZE]);

        let start = start + Request::INDEX_SIZE;
        let mut block_begin = [0; Request::BEGIN_SIZE];
        block_begin.copy_from_slice(&crs.get_ref()[start..start + Request::BEGIN_SIZE]);

        let start = start + Request::BEGIN_SIZE;
        let mut block_length = [0; Request::LENGTH_SIZE];
        block_length.copy_from_slice(&crs.get_ref()[start..start + Request::LENGTH_SIZE]);

        Request {
            piece_index: u32::from_be_bytes(piece_index),
            block_begin: u32::from_be_bytes(block_begin),
            block_length: u32::from_be_bytes(block_length),
        }
    }

    pub fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        if length == Request::LEN as usize && available_data >= Request::LEN_SIZE + length {
            return Ok(Request::FULL_SIZE);
        }

        Err(Error::Incomplete("Request"))
    }

    pub fn piece_index(&self) -> usize {
        self.piece_index as usize
    }

    pub fn block_begin(&self) -> usize {
        self.block_begin as usize
    }

    pub fn block_length(&self) -> usize {
        self.block_length as usize
    }

    pub fn validate(
        &self,
        piece_index: usize,
        pieces_num: usize,
        piece_length: usize,
    ) -> Result<(), Error> {
        if self.piece_index >= pieces_num as u32 || self.piece_index != piece_index as u32 {
            return Err(Error::InvalidPieceIndex("Request"));
        }

        if self.block_length > PIECE_BLOCK_SIZE as u32 {
            return Err(Error::InvalidLength("Request"));
        }

        if self.block_begin + self.block_length > piece_length as u32 {
            return Err(Error::InvalidLength("Request"));
        }

        Ok(())
    }
}

impl Serializer for Request {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&Request::LEN.to_be_bytes());
        vec.push(Request::ID);
        vec.extend_from_slice(&self.piece_index.to_be_bytes());
        vec.extend_from_slice(&self.block_begin.to_be_bytes());
        vec.extend_from_slice(&self.block_length.to_be_bytes());

        vec
    }
}
