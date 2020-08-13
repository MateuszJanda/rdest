#[derive(PartialEq, Debug)]
pub enum BValue {
    Int(i32),
//        Str(String),
//        List(),
//        Val(BValue)
//        Val(Box<BValue>)
}

impl BValue {
    pub fn parse(arg: &[u8]) -> Result<Vec<BValue>, &'static str> {
        let mut result = vec![];
        let mut it = arg.iter();
        while let Some(b) = it.next() {
            if *b == b'i' {
                let num = match parse_int(&mut it) {
                    Ok(v) => v,
                    Err(desc) => return Err(desc)
                };
                result.push(num);
            }
        }

        Ok(result)
    }
}

fn parse_int(it : &mut std::slice::Iter<u8>) -> Result<BValue, &'static str> {
    let mut num_bytes = vec![];
    while let Some(b) = it.next() {
        if (*b >= b'0' && *b <= b'9') || *b == b'-' {
            num_bytes.push(*b);
        } else if *b == b'e' {
            let num_str = match String::from_utf8(num_bytes) {
                Ok(v) => v,
                Err(_e) => return Err("Unable convert bytes to string")
            };
            let num : i32 = match num_str.parse() {
                Ok(v) => v,
                Err(_e) => return Err("Unable convert string to int")
            };

            if num_str.len() >= 2 && num_str.starts_with("0") || num_str.starts_with("-0") {
                return Err("Leading zero when converting string to int")
            }

            return Ok(BValue::Int(num))
        } else {
            return Err("Incorrect character when converting string to int")
        }
    }

    Err("Missing terminate character 'e' for int parsing")
}


#[cfg(test)]
mod tests {
    use super::*;
    use bencode::*;

    # [test]
    fn empty_input() {
    assert_eq ! (BValue::parse(b""), Ok(vec ! []));
    }

    # [test]
    fn parse_int_missing_e() {
    assert_eq ! (BValue::parse(b"i"), Err("Missing terminate character 'e' for int parsing"));
    }

    # [test]
    fn parse_int_missing_value() {
    assert_eq ! (BValue::parse(b"ie"), Err("Unable convert string to int"));
    }

    # [test]
    fn parse_int_incorrect_format1() {
    assert_eq ! (BValue::parse(b"i-e"), Err("Unable convert string to int"));
    }

    # [test]
    fn parse_int_incorrect_format2() {
    assert_eq ! (BValue::parse(b"i--4e"), Err("Unable convert string to int"));
    }

    # [test]
    fn parse_int_incorrect_format3() {
    assert_eq ! (BValue::parse(b"i-4-e"), Err("Unable convert string to int"));
    }

    # [test]
    fn parse_int_incorrect_character() {
    assert_eq ! (BValue::parse(b"i+4e"), Err("Incorrect character when converting string to int"));
    }

    # [test]
    fn parse_int_leading_zero() {
    assert_eq ! (BValue::parse(b"i01e"), Err("Leading zero when converting string to int"));
    }

    # [test]
    fn parse_int_leading_zero_for_negative() {
    assert_eq ! (BValue::parse(b"i-01e"), Err("Leading zero when converting string to int"));
    }

    # [test]
    fn parse_int_zero() {
    assert_eq ! (BValue::parse(b"i0e"), Ok(vec ! [BValue::Int(0)]));
    }

    # [test]
    fn positive_int() {
    assert_eq ! (BValue::parse(b"i4e"), Ok(vec ! [BValue::Int(4)]));
    }

    # [test]
    fn negative_int() {
    assert_eq ! (BValue::parse(b"i-4e"), Ok(vec ! [BValue::Int( -4)]));
    }

    # [test]
    fn two_ints() {
    assert_eq ! (BValue::parse(b"i2ei-3e"), Ok(vec ! [BValue::Int(2), BValue::Int( - 3)]));
    }
}