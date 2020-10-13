#[cfg(test)]
use crate::hashmap;
use std::collections::HashMap;

type Key = Vec<u8>;

#[derive(PartialEq, Clone, Debug)]
pub enum BValue {
    Int(i64),
    ByteStr(Vec<u8>),
    List(Vec<BValue>),
    Dict(HashMap<Key, BValue>),
}

impl BValue {
    fn values_vector(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
        delimiter: Option<u8>,
    ) -> Result<Vec<BValue>, String> {
        let mut values = vec![];

        while let Some((pos, b)) = it.next() {
            match b {
                b'0'..=b'9' => values.push(Self::value_byte_str(it, pos, b)?),
                b'i' => values.push(Self::value_int(it, pos)?),
                b'l' => values.push(Self::value_list(it)?),
                b'd' => values.push(Self::value_dict(it, pos)?),
                d if delimiter.is_some() && delimiter.unwrap() == *d => return Ok(values),
                _ => return Err(format!("Loop [{}]: Incorrect character", pos)),
            }
        }

        Ok(values)
    }

    fn value_byte_str(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
        pos: usize,
        first_num: &u8,
    ) -> Result<BValue, String> {
        Ok(BValue::ByteStr(Self::parse_byte_str(it, pos, first_num)?.0))
    }

    fn value_int(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
        pos: usize,
    ) -> Result<BValue, String> {
        Ok(BValue::Int(Self::parse_int(it, pos)?.0))
    }

    fn value_list(it: &mut std::iter::Enumerate<std::slice::Iter<u8>>) -> Result<BValue, String> {
        return match Self::parse_list(it) {
            Ok(v) => Ok(BValue::List(v)),
            Err(e) => Err(e),
        };
    }

    fn value_dict(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
        pos: usize,
    ) -> Result<BValue, String> {
        return match Self::parse_dict(it, pos) {
            Ok(v) => Ok(BValue::Dict(v)),
            Err(e) => Err(e),
        };
    }

    pub fn parse_byte_str(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
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

    pub fn parse_int(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
        pos: usize,
    ) -> Result<(i64, Vec<u8>), String> {
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

    fn parse_list(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
    ) -> Result<Vec<BValue>, String> {
        return Self::values_vector(it, Some(b'e'));
    }

    fn parse_dict(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
        pos: usize,
    ) -> Result<HashMap<Vec<u8>, BValue>, String> {
        let list = Self::values_vector(it, Some(b'e'))?;
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

    fn extract_int(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
        pos: usize,
    ) -> Result<Vec<u8>, String> {
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

pub struct BDecoder {
}

impl BDecoder {
    pub fn from_array(arg: &[u8]) -> Result<Vec<BValue>, String> {
        let mut it = arg.iter().enumerate();
        BValue::values_vector(&mut it, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        assert_eq!(BDecoder::from_array(b""), Ok(vec![]));
    }

    #[test]
    fn incorrect_character() {
        assert_eq!(
            BDecoder::from_array(b"x"),
            Err(String::from("Loop [0]: Incorrect character"))
        );
    }

    #[test]
    fn byte_str() {
        assert_eq!(
            BDecoder::from_array(b"9:spamIsLoL"),
            Ok(vec![BValue::ByteStr(b"spamIsLoL".to_vec())])
        );
    }

    #[test]
    fn byte_str_unexpected_end() {
        assert_eq!(
            BDecoder::from_array(b"4"),
            Err(String::from("ByteStr [0]: Not enough characters"))
        );
    }

    #[test]
    fn byte_str_missing_value() {
        assert_eq!(
            BDecoder::from_array(b"4:"),
            Err(String::from("ByteStr [0]: Not enough characters"))
        );
    }

    #[test]
    fn byte_str_not_enough_characters() {
        assert_eq!(
            BDecoder::from_array(b"4:spa"),
            Err(String::from("ByteStr [0]: Not enough characters"))
        );
    }

    #[test]
    fn byte_str_invalid_len_character() {
        assert_eq!(
            BDecoder::from_array(b"4+3:spa"),
            Err(String::from("ByteStr [0]: Incorrect character"))
        );
    }

    #[test]
    fn byte_str_zero_length() {
        assert_eq!(BDecoder::from_array(b"0:"), Ok(vec![BValue::ByteStr(vec![])]));
    }

    #[test]
    fn int_missing_e() {
        assert_eq!(
            BDecoder::from_array(b"i"),
            Err(String::from("Int [0]: Missing terminate character 'e'"))
        );
    }

    #[test]
    fn int_missing_value() {
        assert_eq!(
            BDecoder::from_array(b"ie"),
            Err(String::from("Int [0]: Unable convert to int"))
        );
    }

    #[test]
    fn int_incorrect_format1() {
        assert_eq!(
            BDecoder::from_array(b"i-e"),
            Err(String::from("Int [0]: Unable convert to int"))
        );
    }

    #[test]
    fn int_incorrect_format2() {
        assert_eq!(
            BDecoder::from_array(b"i--4e"),
            Err(String::from("Int [0]: Unable convert to int"))
        );
    }

    #[test]
    fn int_incorrect_format3() {
        assert_eq!(
            BDecoder::from_array(b"i-4-e"),
            Err(String::from("Int [0]: Unable convert to int"))
        );
    }

    #[test]
    fn int_incorrect_character() {
        assert_eq!(
            BDecoder::from_array(b"i+4e"),
            Err(String::from("Int [0]: Incorrect character"))
        );
    }

    #[test]
    fn int_leading_zero() {
        assert_eq!(
            BDecoder::from_array(b"i01e"),
            Err(String::from("Int [0]: Leading zero"))
        );
    }

    #[test]
    fn int_leading_zero_for_negative() {
        assert_eq!(
            BDecoder::from_array(b"i-01e"),
            Err(String::from("Int [0]: Leading zero"))
        );
    }

    #[test]
    fn int_zero() {
        assert_eq!(BDecoder::from_array(b"i0e"), Ok(vec![BValue::Int(0)]));
    }

    #[test]
    fn int_positive() {
        assert_eq!(BDecoder::from_array(b"i4e"), Ok(vec![BValue::Int(4)]));
    }

    #[test]
    fn int_negative() {
        assert_eq!(BDecoder::from_array(b"i-4e"), Ok(vec![BValue::Int(-4)]));
    }

    #[test]
    fn int_above_u32() {
        assert_eq!(
            BDecoder::from_array(b"i4294967297e"),
            Ok(vec![BValue::Int(4294967297)])
        );
    }

    // TODO: bit int support needed
    //    fn int_above_i64() {
    //        assert_eq!(BDecoder::from_array(b"i9223372036854775808e"), Ok(vec![BValue::Int(9223372036854775808)]));
    //    }

    #[test]
    fn list_of_strings() {
        assert_eq!(
            BDecoder::from_array(b"l4:spam4:eggse"),
            Ok(vec![BValue::List(vec![
                BValue::ByteStr(b"spam".to_vec()),
                BValue::ByteStr(b"eggs".to_vec())
            ])])
        );
    }

    #[test]
    fn list_of_ints() {
        assert_eq!(
            BDecoder::from_array(b"li1ei9ee"),
            Ok(vec![BValue::List(vec![BValue::Int(1), BValue::Int(9)])])
        );
    }

    #[test]
    fn list_of_nested_values() {
        assert_eq!(
            BDecoder::from_array(b"lli1ei5ee3:abce"),
            Ok(vec![BValue::List(vec![
                BValue::List(vec![BValue::Int(1), BValue::Int(5)]),
                BValue::ByteStr(b"abc".to_vec())
            ])])
        );
    }

    #[test]
    fn dict_odd_number_of_elements() {
        assert_eq!(
            BDecoder::from_array(b"di1ee"),
            Err(String::from("Dict [0]: Odd number of elements"))
        );
    }

    #[test]
    fn dict_key_not_string() {
        assert_eq!(
            BDecoder::from_array(b"di1ei1ee"),
            Err(String::from("Dict [0]: Key not string"))
        );
    }

    #[test]
    fn dict() {
        assert_eq!(
            BDecoder::from_array(b"d1:ki5ee"),
            Ok(vec![BValue::Dict(hashmap![vec![b'k'] => BValue::Int(5)]),])
        );
    }

    #[test]
    fn two_ints() {
        assert_eq!(
            BDecoder::from_array(b"i2ei-3e"),
            Ok(vec![BValue::Int(2), BValue::Int(-3)])
        );
    }

    #[test]
    fn empty_string_and_int() {
        assert_eq!(
            BDecoder::from_array(b"0:i4e"),
            Ok(vec![BValue::ByteStr(vec![]), BValue::Int(4)])
        );
    }

    #[test]
    fn incorrect_value_char_pointer_change() {
        assert_eq!(
            BDecoder::from_array(b"i1ei2ei01e"),
            Err(String::from("Int [6]: Leading zero"))
        );
    }
}
