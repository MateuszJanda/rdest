use std::collections::HashMap;
// #[allow(unused_imports)]
use super::hashmap;

type Key = Vec<u8>;

#[derive(PartialEq, Clone, Debug)]
pub enum BValue {
    Int(i64),
    ByteStr(Vec<u8>),
    List(Vec<BValue>),
    Dict(HashMap<Key, BValue>),
}

impl BValue {
    pub fn parse(arg: &[u8]) -> Result<Vec<BValue>, String> {
        let mut it = arg.iter().enumerate();
        Self::parse_values(&mut it, None)
    }

    fn parse_values(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
        delimiter: Option<u8>,
    ) -> Result<Vec<BValue>, String> {
        let mut values = vec![];
        let (is_delim, delim) = delimiter.map_or((false, b' '), |v| (true, v));

        while let Some((pos, b)) = it.next() {
            if *b >= b'0' && *b <= b'9' {
                values.push(Self::parse_byte_str(it, pos, b)?);
            } else if *b == b'i' {
                values.push(Self::parse_int(it, pos)?);
            } else if *b == b'l' {
                values.push(Self::parse_list(it)?);
            } else if *b == b'd' {
                values.push(Self::parse_dict(it, pos)?);
            } else if is_delim && *b == delim {
                return Ok(values);
            } else {
                return Err(format!("Loop [{}]: Incorrect character", pos));
            }
        }

        Ok(values)
    }

    fn parse_byte_str(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
        pos: usize,
        first_num: &u8,
    ) -> Result<BValue, String> {
        let mut len_bytes = vec![*first_num];
        let mut rest_len_bytes: Vec<_> = it
            .take_while(|(_, &b)| b != b':')
            .map(|(_, &b)| b)
            .collect();
        len_bytes.append(&mut rest_len_bytes);

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

        if len == 0 {
            return Ok(BValue::ByteStr(vec![]));
        }

        let str_value: Vec<_> = it.take(len).map(|(_, &b)| b).collect();
        if str_value.len() != len {
            return Err(format!("ByteStr [{}]: Not enough characters", pos));
        }

        return Ok(BValue::ByteStr(str_value));
    }

    fn parse_int(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
        pos: usize,
    ) -> Result<BValue, String> {
        let mut it_start = it.clone();
        let num_bytes = Self::extract_int(it, pos)?;

        if let None = it_start.nth(num_bytes.len()) {
            return Err(format!("Int [{}]: Missing terminate character 'e'", pos));
        }
        let num_str = match String::from_utf8(num_bytes) {
            Ok(v) => v,
            Err(_) => return Err(format!("Int [{}]: Unable convert to string", pos)),
        };

        if num_str.len() >= 2 && num_str.starts_with("0") || num_str.starts_with("-0") {
            return Err(format!("Int [{}]: Leading zero", pos));
        }

        num_str
            .parse::<i64>()
            .map(|num| BValue::Int(num))
            .or(Err(format!("Int [{}]: Unable convert to int", pos)))
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

    fn parse_list(it: &mut std::iter::Enumerate<std::slice::Iter<u8>>) -> Result<BValue, String> {
        return match Self::parse_values(it, Some(b'e')) {
            Ok(v) => Ok(BValue::List(v)),
            Err(e) => Err(e),
        };
    }

    fn parse_dict(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
        pos: usize,
    ) -> Result<BValue, String> {
        let list = Self::parse_values(it, Some(b'e'))?;
        if list.len() % 2 != 0 {
            return Err(format!("Dict [{}]: Odd number of elements", pos));
        }

        let keys = Self::get_keys_from_list(&list, pos)?;
        let dict: HashMap<_, _> = keys
            .iter()
            .map(|k| k.clone())
            .zip(list.iter().skip(1).step_by(2).map(|v| v.clone()))
            .collect();

        Ok(BValue::Dict(dict))
    }

    fn get_keys_from_list(list: &Vec<BValue>, pos: usize) -> Result<Vec<Key>, String> {
        list.iter()
            .step_by(2)
            .map(|v| match v {
                BValue::ByteStr(vec) => Ok(vec.clone()),
                _ => Err(format!("Dict [{}]: Key not string", pos)),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        assert_eq!(BValue::parse(b""), Ok(vec![]));
    }

    #[test]
    fn incorrect_character() {
        assert_eq!(
            BValue::parse(b"x"),
            Err(String::from("Loop [0]: Incorrect character"))
        );
    }

    #[test]
    fn byte_str() {
        assert_eq!(
            BValue::parse(b"9:spamIsLoL"),
            Ok(vec![BValue::ByteStr(vec![
                b's', b'p', b'a', b'm', b'I', b's', b'L', b'o', b'L'
            ])])
        );
    }

    #[test]
    fn byte_str_unexpected_nd() {
        assert_eq!(
            BValue::parse(b"4"),
            Err(String::from("ByteStr [0]: Not enough characters"))
        );
    }

    #[test]
    fn byte_str_missing_value() {
        assert_eq!(
            BValue::parse(b"4:"),
            Err(String::from("ByteStr [0]: Not enough characters"))
        );
    }

    #[test]
    fn byte_str_not_enough_characters() {
        assert_eq!(
            BValue::parse(b"4:spa"),
            Err(String::from("ByteStr [0]: Not enough characters"))
        );
    }

    #[test]
    fn byte_str_invalid_len_character() {
        assert_eq!(
            BValue::parse(b"4+3:spa"),
            Err(String::from("ByteStr [0]: Incorrect character"))
        );
    }

    #[test]
    fn byte_str_zero_length() {
        assert_eq!(BValue::parse(b"0:"), Ok(vec![BValue::ByteStr(vec![])]));
    }

    #[test]
    fn int_missing_e() {
        assert_eq!(
            BValue::parse(b"i"),
            Err(String::from("Int [0]: Missing terminate character 'e'"))
        );
    }

    #[test]
    fn int_missing_value() {
        assert_eq!(
            BValue::parse(b"ie"),
            Err(String::from("Int [0]: Unable convert to int"))
        );
    }

    #[test]
    fn int_incorrect_format1() {
        assert_eq!(
            BValue::parse(b"i-e"),
            Err(String::from("Int [0]: Unable convert to int"))
        );
    }

    #[test]
    fn int_incorrect_format2() {
        assert_eq!(
            BValue::parse(b"i--4e"),
            Err(String::from("Int [0]: Unable convert to int"))
        );
    }

    #[test]
    fn int_incorrect_format3() {
        assert_eq!(
            BValue::parse(b"i-4-e"),
            Err(String::from("Int [0]: Unable convert to int"))
        );
    }

    #[test]
    fn int_incorrect_character() {
        assert_eq!(
            BValue::parse(b"i+4e"),
            Err(String::from("Int [0]: Incorrect character"))
        );
    }

    #[test]
    fn int_leading_zero() {
        assert_eq!(
            BValue::parse(b"i01e"),
            Err(String::from("Int [0]: Leading zero"))
        );
    }

    #[test]
    fn int_leading_zero_for_negative() {
        assert_eq!(
            BValue::parse(b"i-01e"),
            Err(String::from("Int [0]: Leading zero"))
        );
    }

    #[test]
    fn int_zero() {
        assert_eq!(BValue::parse(b"i0e"), Ok(vec![BValue::Int(0)]));
    }

    #[test]
    fn int_positive() {
        assert_eq!(BValue::parse(b"i4e"), Ok(vec![BValue::Int(4)]));
    }

    #[test]
    fn int_negative() {
        assert_eq!(BValue::parse(b"i-4e"), Ok(vec![BValue::Int(-4)]));
    }

    #[test]
    fn int_above_u32() {
        assert_eq!(
            BValue::parse(b"i4294967297e"),
            Ok(vec![BValue::Int(4294967297)])
        );
    }

    // TODO: bit int support needed
    //    fn int_above_i64() {
    //        assert_eq!(BValue::parse(b"i9223372036854775808e"), Ok(vec![BValue::Int(9223372036854775808)]));
    //    }

    #[test]
    fn list_of_strings() {
        assert_eq!(
            BValue::parse(b"l4:spam4:eggse"),
            Ok(vec![BValue::List(vec![
                BValue::ByteStr(vec![b's', b'p', b'a', b'm']),
                BValue::ByteStr(vec![b'e', b'g', b'g', b's'])
            ])])
        );
    }

    #[test]
    fn list_of_ints() {
        assert_eq!(
            BValue::parse(b"li1ei9ee"),
            Ok(vec![BValue::List(vec![BValue::Int(1), BValue::Int(9)])])
        );
    }

    #[test]
    fn list_of_nested_values() {
        assert_eq!(
            BValue::parse(b"lli1ei5ee3:abce"),
            Ok(vec![BValue::List(vec![
                BValue::List(vec![BValue::Int(1), BValue::Int(5)]),
                BValue::ByteStr(vec![b'a', b'b', b'c'])
            ])])
        );
    }

    #[test]
    fn dict_odd_number_of_elements() {
        assert_eq!(
            BValue::parse(b"di1ee"),
            Err(String::from("Dict [0]: Odd number of elements"))
        );
    }

    #[test]
    fn dict_key_not_string() {
        assert_eq!(
            BValue::parse(b"di1ei1ee"),
            Err(String::from("Dict [0]: Key not string"))
        );
    }

    #[test]
    fn dict() {
        assert_eq!(
            BValue::parse(b"d1:ki5ee"),
            Ok(vec![BValue::Dict(hashmap![vec![b'k'] => BValue::Int(5)]),])
        );
    }

    #[test]
    fn two_ints() {
        assert_eq!(
            BValue::parse(b"i2ei-3e"),
            Ok(vec![BValue::Int(2), BValue::Int(-3)])
        );
    }

    #[test]
    fn empty_string_and_int() {
        assert_eq!(
            BValue::parse(b"0:i4e"),
            Ok(vec![BValue::ByteStr(vec![]), BValue::Int(4)])
        );
    }

    #[test]
    fn incorrect_value_char_pointer_change() {
        assert_eq!(
            BValue::parse(b"i1ei2ei01e"),
            Err(String::from("Int [6]: Leading zero"))
        );
    }
}
