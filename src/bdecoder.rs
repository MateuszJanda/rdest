use std::collections::HashMap;
use std::iter::Enumerate;
use std::slice::Iter;

type Key = Vec<u8>;

#[derive(PartialEq, Clone, Debug)]
pub enum BValue {
    Int(i64),
    ByteStr(Vec<u8>),
    List(Vec<BValue>),
    Dict(HashMap<Key, BValue>),
}

impl BValue {
    fn values_vector(it: &mut Enumerate<Iter<u8>>, with_end: bool) -> Result<Vec<BValue>, String> {
        let mut values = vec![];

        while let Some((pos, b)) = it.next() {
            match b {
                b'0'..=b'9' => values.push(Self::value_byte_str(it, pos, b)?),
                b'i' => values.push(Self::value_int(it, pos)?),
                b'l' => values.push(Self::value_list(it)?),
                b'd' => values.push(Self::value_dict(it, pos)?),
                b'e' if with_end => return Ok(values),
                _ => return Err(format!("Loop [{}]: Incorrect character", pos)),
            }
        }

        Ok(values)
    }

    fn value_byte_str(
        it: &mut Enumerate<Iter<u8>>,
        pos: usize,
        first_num: &u8,
    ) -> Result<BValue, String> {
        Ok(BValue::ByteStr(Self::parse_byte_str(it, pos, first_num)?.0))
    }

    fn value_int(it: &mut Enumerate<Iter<u8>>, pos: usize) -> Result<BValue, String> {
        Ok(BValue::Int(Self::parse_int(it, pos)?.0))
    }

    fn value_list(it: &mut Enumerate<Iter<u8>>) -> Result<BValue, String> {
        return match Self::parse_list(it) {
            Ok(v) => Ok(BValue::List(v)),
            Err(e) => Err(e),
        };
    }

    fn value_dict(it: &mut Enumerate<Iter<u8>>, pos: usize) -> Result<BValue, String> {
        return match Self::parse_dict(it, pos) {
            Ok(v) => Ok(BValue::Dict(v)),
            Err(e) => Err(e),
        };
    }

    pub fn parse_byte_str(
        it: &mut Enumerate<Iter<u8>>,
        pos: usize,
        first_num: &u8,
    ) -> Result<(Vec<u8>, Vec<u8>), String> {
        let mut len_bytes = vec![*first_num];
        let mut rest_len_bytes: Vec<_> = it
            .take_while(|(_, &b)| b != b':')
            .map(|(_, &b)| b)
            .collect();
        len_bytes.append(&mut rest_len_bytes);
        let mut str_raw = len_bytes.clone();
        str_raw.push(b':');

        if !len_bytes.iter().all(|b| (b'0'..=b'9').contains(b)) {
            return Err(format!("ByteStr [{}]: Incorrect character", pos));
        }

        let len_str = match String::from_utf8(len_bytes) {
            Ok(v) => v,
            Err(_) => return Err(format!("ByteStr [{}]: Unable convert to string", pos)),
        };
        let len: usize = match len_str.parse() {
            Ok(v) => v,
            Err(_) => return Err(format!("ByteStr [{}]: Unable convert to int", pos)),
        };

        let str_value: Vec<_> = it.take(len).map(|(_, &b)| b).collect();
        if str_value.len() != len {
            return Err(format!("ByteStr [{}]: Not enough characters", pos));
        }

        str_raw.append(&mut str_value.clone());
        return Ok((str_value, str_raw));
    }

    pub fn parse_int(it: &mut Enumerate<Iter<u8>>, pos: usize) -> Result<(i64, Vec<u8>), String> {
        let mut it_start = it.clone();
        let num_as_bytes = Self::extract_int(it, pos)?;

        let mut raw_num = vec![b'i'];
        raw_num.append(&mut num_as_bytes.clone());
        raw_num.push(b'e');

        if let None = it_start.nth(num_as_bytes.len()) {
            return Err(format!("Int [{}]: Missing terminate character 'e'", pos));
        }
        let num_as_str = match String::from_utf8(num_as_bytes) {
            Ok(v) => v,
            Err(_) => return Err(format!("Int [{}]: Unable convert to string", pos)),
        };

        if num_as_str.len() >= 2 && num_as_str.starts_with("0") || num_as_str.starts_with("-0") {
            return Err(format!("Int [{}]: Leading zero", pos));
        }

        let num = num_as_str
            .parse::<i64>()
            .or(Err(format!("Int [{}]: Unable convert to int", pos)))?;

        Ok((num, raw_num))
    }

    fn parse_list(it: &mut Enumerate<Iter<u8>>) -> Result<Vec<BValue>, String> {
        return Self::values_vector(it, true);
    }

    fn parse_dict(
        it: &mut Enumerate<Iter<u8>>,
        pos: usize,
    ) -> Result<HashMap<Vec<u8>, BValue>, String> {
        let list = Self::values_vector(it, true)?;
        if list.len() % 2 != 0 {
            return Err(format!("Dict [{}]: Odd number of elements", pos));
        }

        let keys = Self::keys_from_list(&list, pos)?;
        let dict: HashMap<_, _> = keys
            .iter()
            .map(|k| k.clone())
            .zip(list.iter().skip(1).step_by(2).map(|v| v.clone()))
            .collect();

        Ok(dict)
    }

    fn keys_from_list(list: &Vec<BValue>, pos: usize) -> Result<Vec<Key>, String> {
        list.iter()
            .step_by(2)
            .map(|v| match v {
                BValue::ByteStr(vec) => Ok(vec.clone()),
                _ => Err(format!("Dict [{}]: Key not string", pos)),
            })
            .collect()
    }

    fn extract_int(it: &mut Enumerate<Iter<u8>>, pos: usize) -> Result<Vec<u8>, String> {
        it.take_while(|(_, &b)| b != b'e')
            .map(|(_, b)| {
                if (b'0'..=b'9').contains(b) || *b == b'-' {
                    Ok(*b)
                } else {
                    Err(format!("Int [{}]: Incorrect character", pos))
                }
            })
            .collect()
    }
}

pub struct BDecoder {}

impl BDecoder {
    pub fn from_array(arg: &[u8]) -> Result<Vec<BValue>, String> {
        let mut it = arg.iter().enumerate();
        BValue::values_vector(&mut it, false)
    }
}
