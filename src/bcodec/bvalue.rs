use crate::Error;
use std::collections::HashMap;
use std::iter::Enumerate;
use std::slice::Iter;

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

type Key = Vec<u8>;

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
    /// Dictionary where, key and value are both `BValue` (key can be dictionary itself).
    Dict(HashMap<Key, BValue>),
}

impl BValue {
    /// Build vector with `BValues`'s
    pub fn values_vector(
        it: &mut Enumerate<Iter<u8>>,
        with_end: bool,
    ) -> Result<Vec<BValue>, Error> {
        let mut values = vec![];

        while let Some((pos, b)) = it.next() {
            match b.into() {
                Delimiter::Num => values.push(Self::value_byte_str(it, pos, b)?),
                Delimiter::Int => values.push(Self::value_int(it, pos)?),
                Delimiter::List => values.push(Self::value_list(it)?),
                Delimiter::Dict => values.push(Self::value_dict(it, pos)?),
                Delimiter::End if with_end => return Ok(values),
                Delimiter::End => return Err(Error::DecodeUnexpectedChar(file!(), line!(), pos)),
                Delimiter::Unknown => {
                    return Err(Error::DecodeIncorrectChar(file!(), line!(), pos))
                }
            }
        }

        Ok(values)
    }

    fn value_byte_str(
        it: &mut Enumerate<Iter<u8>>,
        pos: usize,
        first_num: &u8,
    ) -> Result<BValue, Error> {
        Ok(BValue::ByteStr(Self::parse_byte_str(it, pos, first_num)?.0))
    }

    fn value_int(it: &mut Enumerate<Iter<u8>>, pos: usize) -> Result<BValue, Error> {
        Ok(BValue::Int(Self::parse_int(it, pos)?.0))
    }

    fn value_list(it: &mut Enumerate<Iter<u8>>) -> Result<BValue, Error> {
        return match Self::parse_list(it) {
            Ok(v) => Ok(BValue::List(v)),
            Err(e) => Err(e),
        };
    }

    fn value_dict(it: &mut Enumerate<Iter<u8>>, pos: usize) -> Result<BValue, Error> {
        return match Self::parse_dict(it, pos) {
            Ok(v) => Ok(BValue::Dict(v)),
            Err(e) => Err(e),
        };
    }

    /// Parse [bncoded](https://en.wikipedia.org/wiki/Bencode) string. It assume that first digit
    /// was already consumed, so must be explicitly provided as an argument.
    pub fn parse_byte_str(
        it: &mut Enumerate<Iter<u8>>,
        pos: usize,
        first_num: &u8,
    ) -> Result<(Vec<u8>, Vec<u8>), Error> {
        let mut len_bytes = vec![*first_num];
        let mut rest_len_bytes: Vec<_> = it
            .take_while(|(_, &b)| b != b':')
            .map(|(_, &b)| b)
            .collect();
        len_bytes.append(&mut rest_len_bytes);
        let mut str_raw = len_bytes.clone();
        str_raw.push(b':');

        if !len_bytes.iter().all(|b| (b'0'..=b'9').contains(b)) {
            return Err(Error::DecodeIncorrectChar(file!(), line!(), pos));
        }

        let len_str = match String::from_utf8(len_bytes) {
            Ok(v) => v,
            Err(_) => return Err(Error::DecodeUnableConvert(file!(), line!(), &"string", pos)),
        };
        let len: usize = match len_str.parse() {
            Ok(v) => v,
            Err(_) => return Err(Error::DecodeUnableConvert(file!(), line!(), &"int", pos)),
        };

        let str_value: Vec<_> = it.take(len).map(|(_, &b)| b).collect();
        if str_value.len() != len {
            return Err(Error::DecodeNotEnoughChars(file!(), line!(), pos));
        }

        str_raw.append(&mut str_value.clone());
        return Ok((str_value, str_raw));
    }

    /// Parse [bencoded](https://en.wikipedia.org/wiki/Bencode) integer till delimiter "e".
    pub fn parse_int(it: &mut Enumerate<Iter<u8>>, pos: usize) -> Result<(i64, Vec<u8>), Error> {
        let mut it_start = it.clone();
        let num_as_bytes = Self::extract_int(it, pos)?;

        let mut raw_num = vec![b'i'];
        raw_num.append(&mut num_as_bytes.clone());
        raw_num.push(b'e');

        if let None = it_start.nth(num_as_bytes.len()) {
            return Err(Error::DecodeMissingTerminalChars(file!(), line!(), pos));
        }
        let num_as_str = match String::from_utf8(num_as_bytes) {
            Ok(v) => v,
            Err(_) => return Err(Error::DecodeUnableConvert(file!(), line!(), &"string", pos)),
        };

        if num_as_str.len() >= 2 && num_as_str.starts_with("0") || num_as_str.starts_with("-0") {
            return Err(Error::DecodeLeadingZero(file!(), line!(), pos));
        }

        let num = num_as_str
            .parse::<i64>()
            .or(Err(Error::DecodeUnableConvert(
                file!(),
                line!(),
                &"int",
                pos,
            )))?;

        Ok((num, raw_num))
    }

    fn parse_list(it: &mut Enumerate<Iter<u8>>) -> Result<Vec<BValue>, Error> {
        return Self::values_vector(it, true);
    }

    fn parse_dict(
        it: &mut Enumerate<Iter<u8>>,
        pos: usize,
    ) -> Result<HashMap<Vec<u8>, BValue>, Error> {
        let list = Self::values_vector(it, true)?;
        if list.len() % 2 != 0 {
            return Err(Error::DecodeOddNumOfElements(file!(), line!(), pos));
        }

        let keys = Self::keys_from_list(&list, pos)?;
        let dict: HashMap<_, _> = keys
            .iter()
            .map(|k| k.clone())
            .zip(list.iter().skip(1).step_by(2).map(|v| v.clone()))
            .collect();

        Ok(dict)
    }

    fn keys_from_list(list: &Vec<BValue>, pos: usize) -> Result<Vec<Key>, Error> {
        list.iter()
            .step_by(2)
            .map(|v| match v {
                BValue::ByteStr(vec) => Ok(vec.clone()),
                _ => Err(Error::DecodeKeyNotString(file!(), line!(), pos)),
            })
            .collect()
    }

    fn extract_int(it: &mut Enumerate<Iter<u8>>, pos: usize) -> Result<Vec<u8>, Error> {
        it.take_while(|(_, &b)| b != b'e')
            .map(|(_, b)| match (b'0'..=b'9').contains(b) || *b == b'-' {
                true => Ok(*b),
                false => Err(Error::DecodeIncorrectChar(file!(), line!(), pos)),
            })
            .collect()
    }
}
