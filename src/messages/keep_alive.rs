use crate::constant::MSG_LEN_SIZE;
use crate::serializer::Serializer;

#[derive(Debug)]
pub struct KeepAlive {}

impl KeepAlive {
    pub const LEN: u32 = 0;
    const LEN_SIZE: usize = MSG_LEN_SIZE;
    pub const FULL_SIZE: usize = KeepAlive::LEN_SIZE;

    pub fn new() -> KeepAlive {
        KeepAlive {}
    }
}

impl Serializer for KeepAlive {
    fn data(&self) -> Vec<u8> {
        KeepAlive::LEN.to_be_bytes().to_vec()
    }
}
