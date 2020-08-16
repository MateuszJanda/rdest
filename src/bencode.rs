use std::collections::HashMap;

type Key = Vec<u8>;

#[derive(PartialEq, Clone, Debug)]
pub enum BValue {
    Int(i32),
    ByteStr(Vec<u8>),
    List(Vec<BValue>),
    Dict(HashMap<Key, BValue>)
}

impl BValue {
    pub fn parse(arg: &[u8]) -> Result<Vec<BValue>, &'static str> {
        let mut it = arg.iter();
        Self::parse_values(&mut it, None)
    }

    fn parse_values(mut it: &mut std::slice::Iter<u8>, delimiter : Option<u8>) -> Result<Vec<BValue>, &'static str> {
        let mut result = vec![];
        let (is_delim, delim) = delimiter.map_or((false, b' '), |v| (true, v));

        while let Some(b) = it.next() {
            if *b >= b'0' && *b <= b'9' {
                let s = match Self::parse_byte_str(&mut it, b) {
                    Ok(v) => v,
                    Err(desc) => return Err(desc)
                };
                result.push(s);
            } else if *b == b'i' {
                let num = match Self::parse_int(&mut it) {
                    Ok(v) => v,
                    Err(desc) => return Err(desc)
                };
                result.push(num);
            } else if *b == b'l' {
                let list = match Self::parse_list(&mut it) {
                    Ok(v) => v,
                    Err(desc) => return Err(desc)
                };
                result.push(list);
            } else if *b == b'd' {
                let list = match Self::parse_dict(&mut it) {
                    Ok(v) => v,
                    Err(desc) => return Err(desc)
                };
                result.push(list);
            } else if is_delim && *b == delim {
                return Ok(result)
            } else {
                return Err("Incorrect character when parsing bencode data")
            }
        }

        Ok(result)
    }

    fn parse_byte_str(it : &mut std::slice::Iter<u8>, first_num : &u8) -> Result<BValue, &'static str> {
        let mut len_bytes = vec![*first_num];
        while let Some(b) = it.next() {
            if *b >= b'0' && *b <= b'9' {
                len_bytes.push(*b);
            } else if *b == b':' {
                let len_str = match String::from_utf8(len_bytes) {
                    Ok(v) => v,
                    Err(_) => return Err("Unable convert string len (bytes) to string")
                };
                let len : usize = match len_str.parse() {
                    Ok(v) => v,
                    Err(_) => return Err("Unable convert string len (string) to int")
                };

                if len == 0 {
                    return Ok(BValue::ByteStr(vec![]));
                }

                let mut str_value = vec![];
                while let Some(ch) = it.next() {
                    str_value.push(*ch);
                    if str_value.len() == len {
                        return Ok(BValue::ByteStr(str_value));
                    }
                }

                return Err("Not enough characters when parsing string");
            } else {
                return Err("Incorrect character when parsing string")
            }
        }

        Err("String parsing end unexpectedly")
    }

    fn parse_int(it : &mut std::slice::Iter<u8>) -> Result<BValue, &'static str> {
        let mut it_start = it.clone();
        let num_bytes = Self::extract_int(it)?;

        if let None = it_start.nth(num_bytes.len()) {
            return Err("Missing terminate character 'e' when parsing int");
        }

//        let num_bytes: Result<Vec<_>, _>= it
//            .take_while(|&&b| b != b'e')
//            .map(|&b| {
//                if (b >= b'0' && b <= b'9') || b == b'-' {
//                    Ok(b)
//                } else {
//                    Err("Incorrect character when parsing int")
//                }
//            })
////            .map(|b| )
//            .collect();
//        num_bytes?;
//        let mut num_bytes = vec![];
//        while let Some(b) = it.next() {
//            if (*b >= b'0' && *b <= b'9') || *b == b'-' {
//                num_bytes.push(*b);
//            } else if *b == b'e' {
                let num_str = match String::from_utf8(num_bytes) {
                    Ok(v) => v,
                    Err(_) => return Err("Unable convert int (bytes) to string")
                };
                let num : i32 = match num_str.parse() {
                    Ok(v) => v,
                    Err(_) => return Err("Unable convert int (string) to int")
                };

                if num_str.len() >= 2 && num_str.starts_with("0") || num_str.starts_with("-0") {
                    return Err("Leading zero when converting to int")
                }

                return Ok(BValue::Int(num))
//            } else {
//                return Err("Incorrect character when parsing int")
//            }
//        }
//
//        Err("Missing terminate character 'e' when parsing int")

    }

    fn extract_int(it : &mut std::slice::Iter<u8>) -> Result<Vec<u8>, &'static str> {
        it.take_while(|&&b| b != b'e')
            .map(|&b| {
                if (b >= b'0' && b <= b'9') || b == b'-' {
                    Ok(b)
                } else {
                    Err("Incorrect character when parsing int")
                }
            })
            .collect()
    }

    fn parse_list(it : &mut std::slice::Iter<u8>) -> Result<BValue, &'static str> {
        return match Self::parse_values(it, Some(b'e')) {
            Ok(v) => Ok(BValue::List(v)),
            Err(e) => Err(e)
        }
    }

    fn parse_dict(it : &mut std::slice::Iter<u8>) -> Result<BValue, &'static str> {
        let list = match Self::parse_values(it, Some(b'e')) {
            Ok(v) => v,
            Err(e) => return Err(e)
        };

        if list.len() % 2 != 0 {
            return Err("Dict: odd number of elements")
        }

        let mut dict : HashMap<Key, BValue> = HashMap::new();
        for i in (0..list.len()).step_by(2) {
            let key = match &list[i] {
                BValue::ByteStr(val) => val,
                _ => return Err("ddd")
            };
            dict.insert(key.to_vec(), list[i+1].clone());
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
    fn byte_str() {
        assert_eq!(BValue::parse(b"4:spam"), Ok(vec![
            BValue::ByteStr(vec![b's', b'p', b'a', b'm'])
        ]));
    }

    #[test]
    fn byte_str_unexpected_nd() {
        assert_eq!(BValue::parse(b"4"), Err("String parsing end unexpectedly"));
    }

    #[test]
    fn byte_str_missing_value() {
        assert_eq!(BValue::parse(b"4:"), Err("Not enough characters when parsing string"));
    }

    #[test]
    fn byte_str_not_nough_characters() {
        assert_eq!(BValue::parse(b"4:spa"), Err("Not enough characters when parsing string"));
    }

    #[test]
    fn byte_str_invalid_len_character() {
        assert_eq!(BValue::parse(b"4+3:spa"), Err("Incorrect character when parsing string"));
    }

    #[test]
    fn byte_str_zero_length() {
        assert_eq!(BValue::parse(b"0:"), Ok(vec![
            BValue::ByteStr(vec![])
        ]));
    }

    #[test]
    fn int_missing_e() {
        assert_eq!(BValue::parse(b"i"), Err("Missing terminate character 'e' when parsing int"));
    }

    #[test]
    fn int_missing_value() {
        assert_eq!(BValue::parse(b"ie"), Err("Unable convert int (string) to int"));
    }

    #[test]
    fn int_incorrect_format1() {
        assert_eq!(BValue::parse(b"i-e"), Err("Unable convert int (string) to int"));
    }

    #[test]
    fn int_incorrect_format2() {
        assert_eq!(BValue::parse(b"i--4e"), Err("Unable convert int (string) to int"));
    }

    #[test]
    fn int_incorrect_format3() {
        assert_eq!(BValue::parse(b"i-4-e"), Err("Unable convert int (string) to int"));
    }

    #[test]
    fn int_incorrect_character() {
        assert_eq!(BValue::parse(b"i+4e"), Err("Incorrect character when parsing int"));
    }

    #[test]
    fn int_leading_zero() {
        assert_eq!(BValue::parse(b"i01e"), Err("Leading zero when converting to int"));
    }

    #[test]
    fn int_leading_zero_for_negative() {
        assert_eq!(BValue::parse(b"i-01e"), Err("Leading zero when converting to int"));
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
        assert_eq!(BValue::parse(b"l4:spam4:eggse"),
                   Ok(vec![BValue::List(vec![
                       BValue::ByteStr(vec![b's', b'p', b'a', b'm']),
                       BValue::ByteStr(vec![b'e', b'g', b'g', b's'])
                   ])]));
    }

    #[test]
    fn list_of_ints() {
        assert_eq!(BValue::parse(b"li1ei5ee"),
                   Ok(vec![BValue::List(vec![
                       BValue::Int(1),
                       BValue::Int(5)
                   ])]));
    }

    #[test]
    fn list_of_nested_values() {
        assert_eq!(BValue::parse(b"lli1ei5ee3:abce"),
                   Ok(vec![BValue::List(vec![
                       BValue::List(vec![
                           BValue::Int(1),
                           BValue::Int(5)
                       ]),
                       BValue::ByteStr(vec![b'a', b'b', b'c'])
                   ])]));
    }

    #[test]
    fn dict_odd_number_of_elements() {
        assert_eq!(BValue::parse(b"di1ee"), Err("Dict: odd number of elements"));
    }

    #[test]
    fn dict() {
        assert_eq!(BValue::parse(b"d1:ki5ee"),
                   Ok(vec![
                       BValue::Dict(hashmap![vec![b'k'] => BValue::Int(5)]),
                   ]));
    }

    #[test]
    fn two_ints() {
        assert_eq!(BValue::parse(b"i2ei-3e"), Ok(vec![BValue::Int(2), BValue::Int(-3)]));
    }

    #[test]
    fn empty_string_and_int() {
        assert_eq!(BValue::parse(b"0:i4e"), Ok(vec![
            BValue::ByteStr(vec![]),
            BValue::Int(4)]
        ));
    }
}