use crate::constants::MSG_LEN_SIZE;
use crate::serializer::Serializer;
use crate::Error;

#[derive(Debug)]
pub struct NotInterested {}

impl NotInterested {
    const LEN: u32 = 1;
    pub const ID: u8 = 3;
    const LEN_SIZE: usize = MSG_LEN_SIZE;
    const FULL_SIZE: usize = NotInterested::LEN_SIZE + NotInterested::LEN as usize;

    pub fn new() -> NotInterested {
        NotInterested {}
    }

    pub fn check(length: usize) -> Result<usize, Error> {
        match length == NotInterested::LEN as usize {
            true => Ok(NotInterested::FULL_SIZE),
            false => Err(Error::Incomplete("NotInterested")),
        }
    }
}

impl Serializer for NotInterested {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&NotInterested::LEN.to_be_bytes());
        vec.push(NotInterested::ID);

        vec
    }
}
