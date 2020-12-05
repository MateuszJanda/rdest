use crate::constant::{MSG_ID_SIZE, MSG_LEN_SIZE};
use crate::serializer::Serializer;
use crate::Error;
use std::io::Cursor;

#[derive(Debug)]
pub struct Cancel {
    index: u32,
    block_begin: u32,
    block_length: u32,
}

impl Cancel {
    const LEN: u32 = 13;
    pub const ID: u8 = 8;
    const LEN_SIZE: usize = MSG_LEN_SIZE;
    const ID_SIZE: usize = MSG_ID_SIZE;
    const INDEX_SIZE: usize = 4;
    const BEGIN_SIZE: usize = 4;
    const LENGTH_SIZE: usize = 4;
    const FULL_SIZE: usize = Cancel::LEN_SIZE + Cancel::LEN as usize;

    pub fn from(crs: &Cursor<&[u8]>) -> Cancel {
        let start = Cancel::LEN_SIZE + Cancel::ID_SIZE;
        let mut index = [0; Cancel::INDEX_SIZE];
        index.copy_from_slice(&crs.get_ref()[start..start + Cancel::INDEX_SIZE]);

        let start = start + Cancel::INDEX_SIZE;
        let mut block_begin = [0; Cancel::BEGIN_SIZE];
        block_begin.copy_from_slice(&crs.get_ref()[start..start + Cancel::BEGIN_SIZE]);

        let start = start + Cancel::BEGIN_SIZE;
        let mut block_length = [0; Cancel::LENGTH_SIZE];
        block_length.copy_from_slice(&crs.get_ref()[start..start + Cancel::LENGTH_SIZE]);

        Cancel {
            index: u32::from_be_bytes(index),
            block_begin: u32::from_be_bytes(block_begin),
            block_length: u32::from_be_bytes(block_length),
        }
    }

    pub fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        match length == Cancel::LEN as usize && available_data >= Cancel::LEN_SIZE + length {
            true => return Ok(Cancel::FULL_SIZE),
            false => Err(Error::Incomplete("Cancel".into())),
        }
    }
}

impl Serializer for Cancel {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&Cancel::LEN.to_be_bytes());
        vec.push(Cancel::ID);
        vec.extend_from_slice(&self.index.to_be_bytes());
        vec.extend_from_slice(&self.block_begin.to_be_bytes());
        vec.extend_from_slice(&self.block_length.to_be_bytes());

        vec
    }
}
