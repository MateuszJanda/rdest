use crate::raw_finder::RawFinder;
use crate::bdecoder::BValue;
use std::iter::Enumerate;
use std::slice::Iter;

pub struct DeepFinder {
}

impl DeepFinder {
    fn raw_values_vector(
        it: &mut Enumerate<Iter<u8>>,
        key: Option<&[u8]>,
        with_end: bool,
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
                b'e' if with_end => return Ok(values),
                _ => return Err(format!("Raw Loop [{}]: Incorrect character", pos)),
            }
        }
        Ok(values)
    }

    fn raw_int(
        it: &mut Enumerate<Iter<u8>>,
        pos: usize,
        extract: bool,
    ) -> Result<Vec<u8>, String> {
        let val = BValue::parse_int(it, pos)?.1;
        match extract {
            true => Ok(val),
            false => Ok(vec![]),
        }
    }

    fn raw_list(
        it: &mut Enumerate<Iter<u8>>,
        extract: bool,
    ) -> Result<Vec<u8>, String> {
        match extract {
            true => {
                let mut list = vec![b'l'];
                list.append(&mut Self::raw_values_vector(it, None, true, extract)?);
                list.push(b'e');
                Ok(list)
            }
            false => Ok(vec![]),
        }
    }

    fn raw_dict(
        it: &mut Enumerate<Iter<u8>>,
        extract: bool,
    ) -> Result<Vec<u8>, String> {
        match extract {
            true => {
                let mut list = vec![b'd'];
                list.append(&mut Self::raw_values_vector(it, None, true, extract)?);
                list.push(b'e');
                Ok(list)
            }
            false => Ok(vec![]),
        }
    }

    fn traverse_dict(
        it: &mut Enumerate<Iter<u8>>,
        key: &[u8],
    ) -> Result<Vec<u8>, String> {
        const EXTRACT_KEY: bool = true;
        let mut extract_value = false;
        let mut key_turn = true;
        while let Some((pos, b)) = it.next() {
            if key_turn {
                match b {
                    b'0'..=b'9' => {
                        extract_value = &*Self::raw_byte_str(it, pos, b, EXTRACT_KEY)? == key
                    }
                    b'i' => extract_value = &*Self::raw_int(it, pos, EXTRACT_KEY)? == key,
                    b'l' => extract_value = &*Self::raw_list(it, EXTRACT_KEY)? == key,
                    b'd' => {
                        let mut dict_it = it.clone();
                        if &*Self::raw_dict(it, EXTRACT_KEY)? == key {
                            extract_value = true;
                        } else {
                            let val = Self::traverse_dict(&mut dict_it, key)?;
                            if val.len() > 0 {
                                return Ok(val);
                            }
                        }
                    }
                    b'e' => break,
                    _ => return Err(format!("Traverse [{}] : Incorrect character", pos)),
                };
            } else if !key_turn {
                let mut dict_it = it.clone();
                let val = Self::extract_dict_raw_value(it, b, pos);
                if extract_value {
                    return val;
                } else if *b == b'd' {
                    let val = Self::traverse_dict(&mut dict_it, key)?;
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
        it: &mut Enumerate<Iter<u8>>,
        b: &u8,
        pos: usize,
    ) -> Result<Vec<u8>, String> {
        let mut values = vec![];
        let extract = true;
        match b {
            b'0'..=b'9' => values.append(&mut BValue::parse_byte_str(it, pos, b)?.1),
            b'i' => values.append(&mut Self::raw_int(it, pos, extract)?),
            b'l' => values.append(&mut Self::raw_list(it, extract)?),
            b'd' => values.append(&mut Self::raw_dict(it, extract)?),
            _ => return Err(format!("Extract dict val [{}]: Incorrect character", pos)),
        }

        Ok(values)
    }

    fn raw_byte_str(
        it: &mut Enumerate<Iter<u8>>,
        pos: usize,
        first_num: &u8,
        extract: bool,
    ) -> Result<Vec<u8>, String> {
        let val = BValue::parse_byte_str(it, pos, first_num)?.1;
        match extract {
            true => Ok(val),
            false => Ok(vec![]),
        }
    }
}

impl RawFinder for DeepFinder {
    fn find_first(key: &str, arg: &[u8]) -> Option<Vec<u8>> {
        let mut it = arg.iter().enumerate();
        match Self::raw_values_vector(&mut it, Some(key.as_bytes()), false, false) {
            Ok(val) if val.len() > 0 => Some(val),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn find_raw_int_value() {
        assert_eq!(
            DeepFinder::find_first("1:k", b"d1:ki-5ee"),
            Some(b"i-5e".to_vec())
        );
    }

    #[test]
    fn find_raw_str_value() {
        assert_eq!(
            DeepFinder::find_first("1:k", b"d1:k4:spame"),
            Some(b"4:spam".to_vec())
        );
    }

    #[test]
    fn find_raw_list_value() {
        assert_eq!(
            DeepFinder::find_first("1:k", b"d1:kli10ei20ee"),
            Some(b"li10ei20ee".to_vec())
        );
    }

    #[test]
    fn find_raw_dict_value() {
        assert_eq!(
            DeepFinder::find_first("1:k", b"i4ed1:kdi5ei0eee"),
            Some(b"di5ei0ee".to_vec())
        );
    }

    #[test]
    fn find_raw_first_find() {
        assert_eq!(
            DeepFinder::find_first("1:k", b"d1:ki1eed1:ki2ee"),
            Some(b"i1e".to_vec())
        );
    }

    #[test]
    fn find_deep_not_found() {
        assert_eq!(DeepFinder::find_first("1:k", b"di0ei1ee"), None);
    }

    #[test]
    fn find_deep_incorrect_bencode() {
        assert_eq!(DeepFinder::find_first("1:k", b"d1:kX4:spame"), None);
    }

    #[test]
    fn find_deep_of_last_key() {
        assert_eq!(
            DeepFinder::find_first("i2e", b"di0ei1ei2ei3ee"),
            Some(b"i3e".to_vec())
        );
    }

    #[test]
    fn find_deep_in_sub_dict() {
        assert_eq!(
            DeepFinder::find_first("i1e", b"i4ed1:kdi1ei9eee"),
            Some(b"i9e".to_vec())
        );
    }

    #[test]
    fn find_deep_in_dict_key() {
        assert_eq!(
            DeepFinder::find_first("i1e", b"ddi1ei9ee1:ke"),
            Some(b"i9e".to_vec())
        );
    }

    #[test]
    fn find_deep_key_as_dict() {
        assert_eq!(
            DeepFinder::find_first("di1ei9ee", b"ddi1ei9ee1:ke"),
            Some(b"1:k".to_vec())
        );
    }
}