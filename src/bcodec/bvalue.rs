use core::fmt;
use std::collections::HashMap;

pub enum Delimiter {
    Num,
    Int,
    List,
    Dict,
    End,
    Unknown,
}

impl From<&u8> for Delimiter {
    fn from(byte: &u8) -> Self {
        match byte {
            b'0'..=b'9' => Delimiter::Num,
            b'i' => Delimiter::Int,
            b'l' => Delimiter::List,
            b'd' => Delimiter::Dict,
            b'e' => Delimiter::End,
            _ => Delimiter::Unknown,
        }
    }
}

/// [Bencode](https://en.wikipedia.org/wiki/Bencode) representation. [BEP3](https://www.bittorrent.org/beps/bep_0003.html#bencoding)
/// specify four basic types: integer, string (but can be any byte array), list and dictionary.
#[derive(PartialEq, Clone, Debug)]
pub enum BValue {
    /// Integer representation. [BEP3](https://www.bittorrent.org/beps/bep_0003.html#bencoding) doesn't
    /// specify max/min limit, so in this implementation i64 was used, and should be sufficient.
    Int(i64),
    /// String representation, more precisely this can be any u8 array.
    ByteStr(Vec<u8>),
    /// List of `BValue` values
    List(Vec<BValue>),
    /// Dictionary where, key and value are both `BValue`s (key can be dictionary itself).
    Dict(HashMap<Vec<u8>, BValue>),
}

impl fmt::Display for BValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BValue::Int(i) => write!(f, "Int: {}", i),
            BValue::ByteStr(vec) => match String::from_utf8(vec.clone()) {
                Ok(s) => write!(f, "Str: {}", s),
                Err(_) => write!(f, "Str={:?}", vec),
            },
            BValue::List(list) => {
                let _ = write!(f, "List: [");
                for val in list {
                    let _ = write!(f, "{}, ", val);
                }
                write!(f, "]")
            }
            BValue::Dict(dict) => {
                let _ = write!(f, "Dict: [");
                for (key, val) in dict {
                    let _ = write!(f, "{} => {}, ", BValue::ByteStr(key.clone()), val);
                }
                write!(f, "]")
            }
        }
    }
}
