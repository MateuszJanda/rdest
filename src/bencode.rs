
#[derive(PartialEq, Debug)]
pub enum BValue {
    Int(i32),
    Str(String),
//        List(),
//        Val(BValue)
//        Val(Box<BValue>)
}

impl BValue {
    pub fn parse(arg: &[u8]) -> Result<Vec<BValue>, &'static str> {
        let mut result = vec![];
        let mut it = arg.iter();
        while let Some(b) = it.next() {
            if *b >= b'0' && *b <= b'9' {
                let s = match parse_str(&mut it, b) {
                    Ok(v) => v,
                    Err(desc) => return Err(desc)
                };
                result.push(s);
            } else if *b == b'i' {
                let num = match parse_int(&mut it) {
                    Ok(v) => v,
                    Err(desc) => return Err(desc)
                };
                result.push(num);
            } else {
                return Err("Incorrect character when parsing bencode data")
            }
        }

        Ok(result)
    }
}

fn parse_str(it : &mut std::slice::Iter<u8>, first_num : &u8) -> Result<BValue, &'static str> {
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
                return Ok(BValue::Str(String::from("")));
            }

            let mut str_value = vec![];
            while let Some(ch) = it.next() {
                str_value.push(*ch);
                if str_value.len() == len {
                    break
                }
            }

            if str_value.len() != len {
                return Err("Not enough characters when parsing string");
            }

            return match String::from_utf8(str_value) {
                Ok(v) => Ok(BValue::Str(v)),
                Err(_) => Err("Unable convert string (bytes) to string")
            };
        } else {
            return Err("Incorrect character when parsing string")
        }
    }

    Err("String parsing end unexpectedly")
}

fn parse_int(it : &mut std::slice::Iter<u8>) -> Result<BValue, &'static str> {
    let mut num_bytes = vec![];
    while let Some(b) = it.next() {
        if (*b >= b'0' && *b <= b'9') || *b == b'-' {
            num_bytes.push(*b);
        } else if *b == b'e' {
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
        } else {
            return Err("Incorrect character when parsing int")
        }
    }

    Err("Missing terminate character 'e' when parsing int")
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        assert_eq!(BValue::parse(b""), Ok(vec![]));
    }

    #[test]
    fn parse_str() {
        assert_eq!(BValue::parse(b"4:spam"), Ok(vec![BValue::Str(String::from("spam"))]));
    }

    #[test]
    fn parse_str_unexpected_nd() {
        assert_eq!(BValue::parse(b"4"), Err("String parsing end unexpectedly"));
    }

    #[test]
    fn parse_str_missing_value() {
        assert_eq!(BValue::parse(b"4:"), Err("Not enough characters when parsing string"));
    }

    #[test]
    fn parse_str_not_nough_characters() {
        assert_eq!(BValue::parse(b"4:spa"), Err("Not enough characters when parsing string"));
    }

    #[test]
    fn parse_str_invalid_len_character() {
        assert_eq!(BValue::parse(b"4+3:spa"), Err("Incorrect character when parsing string"));
    }

    #[test]
    fn parse_str_zero_length() {
        assert_eq!(BValue::parse(b"0:"), Ok(vec![BValue::Str(String::from(""))]));
    }

    #[test]
    fn parse_int_missing_e() {
        assert_eq!(BValue::parse(b"i"), Err("Missing terminate character 'e' when parsing int"));
    }

    #[test]
    fn parse_int_missing_value() {
        assert_eq!(BValue::parse(b"ie"), Err("Unable convert int (string) to int"));
    }

    #[test]
    fn parse_int_incorrect_format1() {
        assert_eq!(BValue::parse(b"i-e"), Err("Unable convert int (string) to int"));
    }

    #[test]
    fn parse_int_incorrect_format2() {
        assert_eq!(BValue::parse(b"i--4e"), Err("Unable convert int (string) to int"));
    }

    #[test]
    fn parse_int_incorrect_format3() {
        assert_eq!(BValue::parse(b"i-4-e"), Err("Unable convert int (string) to int"));
    }

    #[test]
    fn parse_int_incorrect_character() {
        assert_eq!(BValue::parse(b"i+4e"), Err("Incorrect character when parsing int"));
    }

    #[test]
    fn parse_int_leading_zero() {
        assert_eq!(BValue::parse(b"i01e"), Err("Leading zero when converting to int"));
    }

    #[test]
    fn parse_int_leading_zero_for_negative() {
        assert_eq!(BValue::parse(b"i-01e"), Err("Leading zero when converting to int"));
    }

    #[test]
    fn parse_int_zero() {
        assert_eq!(BValue::parse(b"i0e"), Ok(vec![BValue::Int(0)]));
    }

    #[test]
    fn positive_int() {
        assert_eq!(BValue::parse(b"i4e"), Ok(vec![BValue::Int(4)]));
    }

    #[test]
    fn positive_list_of_strings() {
        assert_eq!(BValue::parse(b"l4:spam4:eggse"),
                   Ok(vec![BValue::List(vec![
                       BValue::Str(String::from("spam")),
                       BValue::Str(String::from("eggs"))
                   ])]));
    }

    #[test]
    fn positive_list_of_ints() {
        assert_eq!(BValue::parse(b"li1ei5ee"),
                   Ok(vec![BValue::List(vec![
                       BValue::Int(1)   ,
                       BValue::Int(5)
                   ])]));
    }

    #[test]
    fn negative_int() {
        assert_eq!(BValue::parse(b"i-4e"), Ok(vec![BValue::Int(-4)]));
    }

    #[test]
    fn two_ints() {
        assert_eq!(BValue::parse(b"i2ei-3e"), Ok(vec![BValue::Int(2), BValue::Int(-3)]));
    }

    #[test]
    fn empty_string_and_int() {
        assert_eq!(BValue::parse(b"0:i4e"), Ok(vec![BValue::Str(String::from("")), BValue::Int(4)]));
    }
}