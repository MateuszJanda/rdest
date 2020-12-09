use crate::bcodec::bvalue::BValue;
use crate::Error;

#[derive(PartialEq, Clone, Debug)]
pub struct BEncoder {
    data: Vec<u8>,
}

impl BEncoder {
    pub fn new() -> BEncoder {
        BEncoder {
            data: vec![],
        }
    }
}
