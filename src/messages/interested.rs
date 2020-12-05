use crate::constant::MSG_LEN_SIZE;
use crate::serializer::Serializer;
use crate::Error;

#[derive(Debug)]
pub struct Interested {}

impl Interested {
    const LEN: u32 = 1;
    pub const ID: u8 = 2;
    const LEN_SIZE: usize = MSG_LEN_SIZE;
    const FULL_SIZE: usize = Interested::LEN_SIZE + Interested::LEN as usize;

    pub fn new() -> Interested {
        Interested {}
    }

    pub fn check(length: usize) -> Result<usize, Error> {
        match length == Interested::LEN as usize {
            true => Ok(Interested::FULL_SIZE),
            false => Err(Error::Incomplete("Interested".into())),
        }
    }
}

impl Serializer for Interested {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&Interested::LEN.to_be_bytes());
        vec.push(Interested::ID);

        vec
    }
}
