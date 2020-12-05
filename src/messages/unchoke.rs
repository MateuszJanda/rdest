use crate::constant::MSG_LEN_SIZE;
use crate::Error;
use crate::serializer::Serializer;

#[derive(Debug)]
pub struct Unchoke {}

impl Unchoke {
    const LEN: u32 = 1;
    pub const ID: u8 = 1;
    const LEN_SIZE: usize = MSG_LEN_SIZE;
    const FULL_SIZE: usize = Unchoke::LEN_SIZE + Unchoke::LEN as usize;

    pub fn new() -> Unchoke {
        Unchoke {}
    }

    pub fn check(length: usize) -> Result<usize, Error> {
        if length == Unchoke::LEN as usize {
            return Ok(Unchoke::FULL_SIZE);
        }

        Err(Error::Incomplete("Unchoke".into()))
    }
}

impl Serializer for Unchoke {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&Unchoke::LEN.to_be_bytes());
        vec.push(Unchoke::ID);

        vec
    }
}