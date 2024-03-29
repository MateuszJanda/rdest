// Copyright 2020 Mateusz Janda.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::constants::{MSG_ID_SIZE, MSG_LEN_SIZE};
use crate::serializer::Serializer;
use crate::Error;
use std::io::Cursor;

#[derive(Debug)]
pub struct Piece {
    piece_index: u32,
    block_begin: u32,
    block: Vec<u8>,
}

impl Piece {
    pub const ID: u8 = 7;
    const LEN_SIZE: usize = MSG_LEN_SIZE;
    const ID_SIZE: usize = MSG_ID_SIZE;
    const INDEX_SIZE: usize = 4;
    const BEGIN_SIZE: usize = 4;
    const MIN_LEN: usize = Piece::ID_SIZE + Piece::INDEX_SIZE + Piece::BEGIN_SIZE;

    pub fn new(piece_index: usize, block_begin: usize, block: Vec<u8>) -> Piece {
        Piece {
            piece_index: piece_index as u32,
            block_begin: block_begin as u32,
            block,
        }
    }

    pub fn from(crs: &Cursor<&[u8]>) -> Piece {
        let start = Piece::LEN_SIZE + Piece::ID_SIZE;
        let mut piece_index = [0; Piece::INDEX_SIZE];
        piece_index.copy_from_slice(&crs.get_ref()[start..start + Piece::INDEX_SIZE]);

        let start = start + Piece::INDEX_SIZE;
        let mut block_begin = [0; Piece::BEGIN_SIZE];
        block_begin.copy_from_slice(&crs.get_ref()[start..start + Piece::BEGIN_SIZE]);

        let start = start + Piece::BEGIN_SIZE;
        let end = crs.position() as usize;
        let mut block = vec![];
        block.extend_from_slice(&crs.get_ref()[start..end]);

        Piece {
            piece_index: u32::from_be_bytes(piece_index),
            block_begin: u32::from_be_bytes(block_begin),
            block,
        }
    }

    pub fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        match length >= Piece::MIN_LEN && available_data >= Piece::LEN_SIZE + length {
            true => Ok(Piece::LEN_SIZE + length),
            false => Err(Error::Incomplete("Piece")),
        }
    }

    pub fn piece_index(&self) -> usize {
        self.piece_index as usize
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
        piece_index: usize,
        block_begin: usize,
        block_length: usize,
    ) -> Result<(), Error> {
        if self.piece_index as usize != piece_index {
            return Err(Error::InvalidPieceIndex("Piece"));
        }

        if self.block_begin as usize != block_begin {
            return Err(Error::InvalidPieceIndex("Piece"));
        }

        if self.block.len() as usize != block_length {
            return Err(Error::InvalidLength("Piece"));
        }

        Ok(())
    }
}

impl Serializer for Piece {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(
            &((Piece::ID_SIZE + Piece::INDEX_SIZE + Piece::BEGIN_SIZE + self.block.len()) as u32)
                .to_be_bytes(),
        );
        vec.push(Piece::ID);
        vec.extend_from_slice(&self.piece_index.to_be_bytes());
        vec.extend_from_slice(&self.block_begin.to_be_bytes());
        vec.extend_from_slice(self.block.as_slice());

        vec
    }
}
