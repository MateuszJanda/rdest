use std::collections::HashMap;

type Key = Vec<u8>;
type ParseError = String;

#[derive(PartialEq, Clone, Debug)]
pub enum BValue {
    Int(i32),
    ByteStr(Vec<u8>),
    List(Vec<BValue>),
    Dict(HashMap<Key, BValue>),
}

impl BValue {
    pub fn parse(arg: &[u8]) -> Result<Vec<BValue>, ParseError> {
        let mut it = arg.iter();
        Self::parse_values(&mut it, None)
    }

    fn parse_values(
        mut it: &mut std::slice::Iter<u8>,
        delimiter: Option<u8>,
    ) -> Result<Vec<BValue>, ParseError> {
        let mut result = vec![];
        let (is_delim, delim) = delimiter.map_or((false, b' '), |v| (true, v));

        while let Some((pos, b)) = it.enumerate().next() {
            if *b >= b'0' && *b <= b'9' {
                let s = Self::parse_byte_str(&mut it, pos, b)?;
                result.push(s);
            } else if *b == b'i' {
                let num = Self::parse_int(&mut it, pos)?;
                result.push(num);
            } else if *b == b'l' {
                let list = Self::parse_list(&mut it)?;
                result.push(list);
            } else if *b == b'd' {
                let list = Self::parse_dict(&mut it, pos)?;
                result.push(list);
            } else if is_delim && *b == delim {
                return Ok(result);
            } else {
                return Err(format!("Loop [{}]: Incorrect character", pos));
            }
        }

        Ok(result)
    }

    fn parse_byte_str(
        it: &mut std::slice::Iter<u8>,
        pos: usize,
        first_num: &u8,
    ) -> Result<BValue, ParseError> {
        let mut len_bytes = vec![*first_num];
        let mut rest_len_bytes: Vec<u8> = it.take_while(|&&b| b != b':').map(|&b| b).collect();
        len_bytes.append(&mut rest_len_bytes);

        if !len_bytes.iter().all(|&b| b >= b'0' && b <= b'9') {
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

        let str_value: Vec<u8> = it.take(len).map(|&b| b).collect();
        if str_value.len() != len {
            return Err(format!("ByteStr [{}]: Not enough characters", pos));
        }

        return Ok(BValue::ByteStr(str_value));
    }

    fn parse_int(it: &mut std::slice::Iter<u8>, pos: usize) -> Result<BValue, ParseError> {
        let mut it_start = it.clone();
        let num_bytes = Self::extract_int(it, pos)?;

        if let None = it_start.nth(num_bytes.len()) {
            return Err(format!("Int [{}]: Missing terminate character 'e'", pos));
        }
        let num_str = match String::from_utf8(num_bytes) {
            Ok(v) => v,
            Err(_) => return Err(format!("Int [{}]: Unable convert to string", pos)),
        };
        let num: i32 = match num_str.parse() {
            Ok(v) => v,
            Err(_) => return Err(format!("Int [{}]: Unable convert int", pos)),
        };

        if num_str.len() >= 2 && num_str.starts_with("0") || num_str.starts_with("-0") {
            return Err(format!("Int [{}]: Leading zero", pos));
        }

        return Ok(BValue::Int(num));
    }

    fn extract_int(it: &mut std::slice::Iter<u8>, pos: usize) -> Result<Vec<u8>, ParseError> {
        it.take_while(|&&b| b != b'e')
            .map(|&b| {
                if (b >= b'0' && b <= b'9') || b == b'-' {
                    Ok(b)
                } else {
                    Err(format!("Int [{}]: Incorrect character", pos))
                }
            })
            .collect()
    }

    fn parse_list(it: &mut std::slice::Iter<u8>) -> Result<BValue, ParseError> {
        return match Self::parse_values(it, Some(b'e')) {
            Ok(v) => Ok(BValue::List(v)),
            Err(e) => Err(e),
        };
    }

    fn parse_dict(it: &mut std::slice::Iter<u8>, pos: usize) -> Result<BValue, ParseError> {
        let list = Self::parse_values(it, Some(b'e'))?;
        if list.len() % 2 != 0 {
            return Err(format!("Dict [{}]: Odd number of elements", pos));
        }

        let mut dict: HashMap<Key, BValue> = HashMap::new();
        for i in (0..list.len()).step_by(2) {
            let key = match &list[i] {
                BValue::ByteStr(val) => val,
                _ => return Err(format!("Dict [{}]: Key not string", pos)),
            };
            dict.insert(key.to_vec(), list[i + 1].clone());
        }

        Ok(BValue::Dict(dict))
    }
}

macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = ::std::collections::HashMap::new();
         $( map.insert($key, $val); )*
         map
    }}
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
            BValue::parse(b"4:spam"),
            Ok(vec![BValue::ByteStr(vec![b's', b'p', b'a', b'm'])])
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
    fn byte_str_not_nough_characters() {
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
            Err(String::from("Int [0]: Unable convert int"))
        );
    }

    #[test]
    fn int_incorrect_format1() {
        assert_eq!(
            BValue::parse(b"i-e"),
            Err(String::from("Int [0]: Unable convert int"))
        );
    }

    #[test]
    fn int_incorrect_format2() {
        assert_eq!(
            BValue::parse(b"i--4e"),
            Err(String::from("Int [0]: Unable convert int"))
        );
    }

    #[test]
    fn int_incorrect_format3() {
        assert_eq!(
            BValue::parse(b"i-4-e"),
            Err(String::from("Int [0]: Unable convert int"))
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
            BValue::parse(b"li1ei5ee"),
            Ok(vec![BValue::List(vec![BValue::Int(1), BValue::Int(5)])])
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
}
