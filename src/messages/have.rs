use crate::constant::{MSG_LEN_SIZE, MSG_ID_SIZE};
use std::io::Cursor;
use crate::Error;
use crate::serializer::Serializer;

#[derive(Debug)]
pub struct Have {
    index: u32,
}

impl Have {
    const LEN: u32 = 5;
    pub const ID: u8 = 4;
    const LEN_SIZE: usize = MSG_LEN_SIZE;
    const ID_SIZE: usize = MSG_ID_SIZE;
    const INDEX_SIZE: usize = 4;
    const FULL_SIZE: usize = Have::LEN_SIZE + Have::LEN as usize;

    pub fn new(index: usize) -> Have {
        Have {
            index: index as u32,
        }
    }

    pub fn from(crs: &Cursor<&[u8]>) -> Have {
        let start = Have::LEN_SIZE + Have::ID_SIZE;
        let mut index = [0; Have::INDEX_SIZE];
        index.copy_from_slice(&crs.get_ref()[start..start + Have::INDEX_SIZE]);

        Have {
            index: u32::from_be_bytes(index),
        }
    }

    pub fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        if length == Have::LEN as usize && available_data >= Have::LEN_SIZE + length {
            return Ok(Have::FULL_SIZE);
        }

        Err(Error::Incomplete("Have".into()))
    }

    pub fn index(&self) -> usize {
        self.index as usize
    }

    pub fn validate(&self, pieces_num: usize) -> Result<(), Error> {
        if self.index as usize >= pieces_num {
            return Err(Error::InvalidIndex("Have".into()));
        }

        Ok(())
    }
}

impl Serializer for Have {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&Have::LEN.to_be_bytes());
        vec.push(Have::ID);
        vec.extend_from_slice(&self.index.to_be_bytes());

        vec
    }
}