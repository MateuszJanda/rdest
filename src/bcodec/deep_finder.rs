use crate::bcodec::bvalue::Delimiter;
use crate::bcodec::raw_finder::RawFinder;
use crate::{BDecoder, Error};
use std::iter::Enumerate;
use std::slice::Iter;

/// Deep finder class looking for dictionary key in [bencoded](https://en.wikipedia.org/wiki/Bencode)
/// string.
///
/// Perform deep search, by looking for keys in dictionary values (that can be dictionaries itself).
pub struct DeepFinder {}

impl DeepFinder {
    fn raw_values_vector(
        it: &mut Enumerate<Iter<u8>>,
        key: Option<&[u8]>,
        with_end: bool,
        extract: bool,
    ) -> Result<Vec<u8>, Error> {
        let mut values = vec![];

        while let Some((pos, b)) = it.next() {
            match b.into() {
                Delimiter::Num => values.append(&mut Self::raw_byte_str(it, pos, b, extract)?),
                Delimiter::Int => values.append(&mut Self::raw_int(it, pos, extract)?),
                Delimiter::List => values.append(&mut Self::raw_list(it, extract)?),
                Delimiter::Dict => match key {
                    Some(key) => {
                        let val = Self::traverse_dict(it, key)?;
                        if val.len() > 0 {
                            return Ok(val);
                        }
                    }
                    None => values.append(&mut Self::raw_dict(it, extract)?),
                },
                Delimiter::End => {
                    return match with_end {
                        true => Ok(values),
                        false => Err(Error::DecodeUnexpectedChar("raw_values_vector", pos)),
                    }
                }
                Delimiter::Unknown => {
                    return Err(Error::DecodeIncorrectChar("raw_values_vector", pos))
                }
            }
        }
        Ok(values)
    }

    fn raw_int(it: &mut Enumerate<Iter<u8>>, pos: usize, extract: bool) -> Result<Vec<u8>, Error> {
        let val = BDecoder::parse_int(it, pos)?.1;
        match extract {
            true => Ok(val),
            false => Ok(vec![]),
        }
    }

    fn raw_list(it: &mut Enumerate<Iter<u8>>, extract: bool) -> Result<Vec<u8>, Error> {
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

    fn raw_dict(it: &mut Enumerate<Iter<u8>>, extract: bool) -> Result<Vec<u8>, Error> {
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

    fn traverse_dict(it: &mut Enumerate<Iter<u8>>, key: &[u8]) -> Result<Vec<u8>, Error> {
        const EXTRACT_KEY: bool = true;
        let mut extract_value = false;
        let mut key_turn = true;
        while let Some((pos, b)) = it.next() {
            if key_turn {
                match b.into() {
                    Delimiter::Num => {
                        extract_value = &*Self::raw_byte_str(it, pos, b, EXTRACT_KEY)? == key
                    }
                    Delimiter::Int => extract_value = &*Self::raw_int(it, pos, EXTRACT_KEY)? == key,
                    Delimiter::List => extract_value = &*Self::raw_list(it, EXTRACT_KEY)? == key,
                    Delimiter::Dict => {
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
                    Delimiter::End => break,
                    Delimiter::Unknown => {
                        return Err(Error::DecodeIncorrectChar("traverse_dict", pos))
                    }
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
    ) -> Result<Vec<u8>, Error> {
        let mut values = vec![];
        let extract = true;
        match b.into() {
            Delimiter::Num => values.append(&mut BDecoder::parse_byte_str(it, pos, b)?.1),
            Delimiter::Int => values.append(&mut Self::raw_int(it, pos, extract)?),
            Delimiter::List => values.append(&mut Self::raw_list(it, extract)?),
            Delimiter::Dict => values.append(&mut Self::raw_dict(it, extract)?),
            Delimiter::End => {
                return Err(Error::DecodeUnexpectedChar("extract_dict_raw_value", pos))
            }
            Delimiter::Unknown => {
                return Err(Error::DecodeIncorrectChar("extract_dict_raw_value", pos))
            }
        }

        Ok(values)
    }

    fn raw_byte_str(
        it: &mut Enumerate<Iter<u8>>,
        pos: usize,
        first_num: &u8,
        extract: bool,
    ) -> Result<Vec<u8>, Error> {
        let val = BDecoder::parse_byte_str(it, pos, first_num)?.1;
        match extract {
            true => Ok(val),
            false => Ok(vec![]),
        }
    }
}

impl RawFinder for DeepFinder {
    /// Find first value by specific dictionary key in
    /// [bencoded](https://en.wikipedia.org/wiki/Bencode) string. Look also in dictionary values
    /// which may be dictionaries itself. Value is returned in raw foramt.
    ///
    /// # Example
    /// ```
    /// use rdest::{DeepFinder, RawFinder};
    ///
    /// let value = DeepFinder::find_first("1:k", b"d1:k4:spame").unwrap();
    /// assert_eq!(value, b"4:spam".to_vec());
    /// ```
    fn find_first(key: &str, arg: &[u8]) -> Option<Vec<u8>> {
        let mut it = arg.iter().enumerate();
        match Self::raw_values_vector(&mut it, Some(key.as_bytes()), false, false) {
            Ok(val) if val.len() > 0 => Some(val),
            _ => None,
        }
    }
}
