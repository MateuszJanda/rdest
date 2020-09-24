#[cfg(test)]
use super::hashmap;
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
    pub fn parse(arg: &[u8]) -> Result<Vec<BValue>, String> {
        let mut it = arg.iter().enumerate();
        Self::values_vector(&mut it, None)
    }

    pub fn find_raw_value(key: &str, arg: &[u8]) -> Option<Vec<u8>> {
        let mut it = arg.iter().enumerate();
        match Self::raw_values_vector(&mut it, Some(key.as_bytes()), None, false) {
            Ok(val) if val.len() > 0 => Some(val),
            _ => None,
        }
    }

    fn raw_values_vector(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
        key: Option<&[u8]>,
        delimiter: Option<u8>,
        extract: bool,
    ) -> Result<Vec<u8>, String> {
        let mut values = vec![];

        while let Some((pos, b)) = it.next() {
            match b {
                b'0'..=b'9' => values.append(&mut Self::raw_byte_str(it, pos, b, extract)?),
                b'i' => values.append(&mut Self::raw_int(it, pos, extract)?),
                b'l' => values.append(&mut Self::raw_list(it, extract)?),
                b'd' if key.is_some() => {
                    let val = Self::traverse_dict(it, key.unwrap())?;
                    if val.len() > 0 {
                        return Ok(val);
                    }
                }
                b'd' if key.is_none() => values.append(&mut Self::raw_dict(it, extract)?),
                d if delimiter.is_some() && delimiter.unwrap() == *d => return Ok(values),
                _ => return Err(format!("Raw Loop [{}]: Incorrect character", pos)),
            }
        }
        Ok(values)
    }

    fn raw_list(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
        extract: bool,
    ) -> Result<Vec<u8>, String> {
        match extract {
            true => {
                let mut list = vec![b'l'];
                list.append(&mut Self::raw_values_vector(it, None, Some(b'e'), extract)?);
                list.push(b'e');
                Ok(list)
            }
            false => Ok(vec![]),
        }
    }

    fn raw_dict(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
        extract: bool,
    ) -> Result<Vec<u8>, String> {
        match extract {
            true => {
                let mut list = vec![b'd'];
                list.append(&mut Self::raw_values_vector(it, None, Some(b'e'), extract)?);
                list.push(b'e');
                Ok(list)
            }
            false => Ok(vec![]),
        }
    }

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

    fn raw_byte_str(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
        pos: usize,
        first_num: &u8,
        extract: bool,
    ) -> Result<Vec<u8>, String> {
        let val = Self::parse_byte_str(it, pos, first_num)?.1;
        match extract {
            true => Ok(val),
            false => Ok(vec![]),
        }
    }

    fn parse_byte_str(
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

    fn value_int(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
        pos: usize,
    ) -> Result<BValue, String> {
        Ok(BValue::Int(Self::parse_int(it, pos)?.0))
    }

    fn raw_int(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
        pos: usize,
        extract: bool,
    ) -> Result<Vec<u8>, String> {
        let val = Self::parse_int(it, pos)?.1;
        println!("vvv {:?}", val);
        match extract {
            true => Ok(val),
            false => Ok(vec![]),
        }
    }

    fn parse_int(
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

    fn value_list(it: &mut std::iter::Enumerate<std::slice::Iter<u8>>) -> Result<BValue, String> {
        return match Self::parse_list(it) {
            Ok(v) => Ok(BValue::List(v)),
            Err(e) => Err(e),
        };
    }

    fn parse_list(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
    ) -> Result<Vec<BValue>, String> {
        return Self::values_vector(it, Some(b'e'));
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

    fn traverse_dict(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
        key: &[u8],
    ) -> Result<Vec<u8>, String> {
        println!("trawerse");
        const EXTRACT_KEY: bool = true;
        let mut extract_value = false;
        let mut key_turn = true;
        while let Some((pos, b)) = it.next() {
            if key_turn {
                println!("kkk {:?}", *b as char);
                match b {
                    b'0'..=b'9' => {
                        if &*Self::raw_byte_str(it, pos, b, EXTRACT_KEY)? == key {
                            println!("klucz to string");
                            extract_value = true;
                        }
                    },
                    b'i' => {
                        if &*Self::raw_int(it, pos, EXTRACT_KEY)? == key {
                            extract_value = true;
                        }
                    },
                    b'l' => {
                        if &*Self::raw_list(it, EXTRACT_KEY)? == key {
                            extract_value = true;
                        }
                    },
                    b'd' => {
                        let mut bak = it.clone();
                        if &*Self::raw_dict(it, EXTRACT_KEY)? == key {
                            extract_value = true;
                        } else {
                            println!("przeszukuje klucz");
                            let val = Self::traverse_dict(&mut bak, key)?;
                            if val.len() > 0 {
                                println!("znalazlem");
                                return Ok(val);
                            }
                        }
                    },
                    b'e' => {
                        println!("break?");
                        break
                    },
                    _ => return {
                        println!("error?");
                        Err(format!("TODO"))
                    },
                };
            } else if !key_turn {
                let mut bak = it.clone();
                let val = Self::extract_dict_raw_value(it, b, pos);
                if extract_value {
                    return val
                } else if *b == b'd' {
                    println!("przeszukuje wartość");
                    println!("www {:?}", *b as char);
                    let val = Self::traverse_dict(&mut bak, key)?;
                    if val.len() > 0 {
                        return Ok(val);
                    }
                }


            }

            key_turn = !key_turn;
        }

        Ok(vec![])
    }

    fn extract_dict_raw_value(
        it: &mut std::iter::Enumerate<std::slice::Iter<u8>>,
        b: &u8,
        pos: usize,
    ) -> Result<Vec<u8>, String> {
        let mut values = vec![];
        let extract = true;
        match b {
            b'0'..=b'9' => values.append(&mut Self::parse_byte_str(it, pos, b)?.1),
            b'i' => values.append(&mut Self::raw_int(it, pos, extract)?),
            b'l' => values.append(&mut Self::raw_list(it, extract)?),
            b'd' => values.append(&mut Self::raw_dict(it, extract)?),
            _ => return Err(format!("Raw dict val [{}]: Incorrect character", pos)),
        }

        Ok(values)
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
            Ok(vec![BValue::ByteStr(b"spamIsLoL".to_vec())])
        );
    }

    #[test]
    fn byte_str_unexpected_end() {
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
                BValue::ByteStr(b"spam".to_vec()),
                BValue::ByteStr(b"eggs".to_vec())
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
                BValue::ByteStr(b"abc".to_vec())
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

    #[test]
    fn find_raw_int_value() {
        assert_eq!(
            BValue::find_raw_value("1:k", b"d1:ki-5ee"),
            Some(b"i-5e".to_vec())
        );
    }

    #[test]
    fn find_raw_str_value() {
        assert_eq!(
            BValue::find_raw_value("1:k", b"d1:k4:spame"),
            Some(b"4:spam".to_vec())
        );
    }

    #[test]
    fn find_raw_list_value() {
        assert_eq!(
            BValue::find_raw_value("1:k", b"d1:kli10ei20ee"),
            Some(b"li10ei20ee".to_vec())
        );
    }

    #[test]
    fn find_raw_dict_value() {
        assert_eq!(
            BValue::find_raw_value("1:k", b"i4ed1:kdi5ei0eee"),
            Some(b"di5ei0ee".to_vec())
        );
    }

    #[test]
    fn find_raw_first_find() {
        assert_eq!(
            BValue::find_raw_value("1:k", b"d1:ki1eed1:ki2ee"),
            Some(b"i1e".to_vec())
        );
    }

    #[test]
    fn find_raw_value_not_found() {
        assert_eq!(
            BValue::find_raw_value("1:k", b"di0ei1ee"),
            None
        );
    }

    #[test]
    fn find_raw_value_of_last_key() {
        assert_eq!(
            BValue::find_raw_value("i2e", b"di0ei1ei2ei3ee"),
            Some(b"i3e".to_vec())
        );
    }

    #[test]
    fn find_raw_value_in_sub_dict() {
        assert_eq!(
            BValue::find_raw_value("i1e", b"i4ed1:kdi1ei9eee"),
            Some(b"i9e".to_vec())
        );
    }

    #[test]
    fn find_raw_value_in_dict_key() {
        assert_eq!(
            BValue::find_raw_value("i1e", b"ddi1ei9ee1:ke"),
            Some(b"i9e".to_vec())
        );
    }

    #[test]
    fn find_raw_value_key_as_dict() {
        assert_eq!(
            BValue::find_raw_value("di1ei9ee", b"ddi1ei9ee1:ke"),
            Some(b"1:k".to_vec())
        );
    }
}
