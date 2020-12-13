use crate::bcodec::bvalue::BValue;
use crate::Error;

/// [Bencode](https://en.wikipedia.org/wiki/Bencode) decoder used by metafile/torrent files and
/// BitTorrent protocol.
#[derive(PartialEq, Clone, Debug)]
pub struct BDecoder {}

impl BDecoder {
    /// Decode [bencoded](https://en.wikipedia.org/wiki/Bencode) values.
    ///
    /// # Example
    /// ```
    /// use rdest::{BDecoder, BValue};
    /// let val = BDecoder::from_array("i44e".as_bytes()).unwrap();
    ///
    /// assert_eq!(val, vec![BValue::Int(44)]);
    /// ```
    pub fn from_array(arg: &[u8]) -> Result<Vec<BValue>, Error> {
        let mut it = arg.iter().enumerate();
        BValue::values_vector(&mut it, false)
    }
}
