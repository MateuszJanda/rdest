use crate::constants::MSG_LEN_SIZE;
use crate::serializer::Serializer;
use crate::Error;

#[derive(Debug)]
pub struct Choke {}

impl Choke {
    const LEN: u32 = 1;
    pub const ID: u8 = 0;
    const LEN_SIZE: usize = MSG_LEN_SIZE;
    const FULL_SIZE: usize = Choke::LEN_SIZE + Choke::LEN as usize;

    pub fn new() -> Choke {
        Choke {}
    }

    pub fn check(length: usize) -> Result<usize, Error> {
        match length == Choke::LEN as usize {
            true => Ok(Choke::FULL_SIZE),
            false => Err(Error::Incomplete("Choke".into())),
        }
    }
}

impl Serializer for Choke {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&Choke::LEN.to_be_bytes());
        vec.push(Choke::ID);

        vec
    }
}
