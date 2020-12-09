use crate::bvalue::BValue;
use crate::Error;

#[derive(PartialEq, Clone, Debug)]
pub struct BDecoder {}

impl BDecoder {
    pub fn from_array(arg: &[u8]) -> Result<Vec<BValue>, Error> {
        let mut it = arg.iter().enumerate();
        BValue::values_vector(&mut it, false)
    }
}
