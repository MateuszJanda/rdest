use crate::bencode::Value;

fn main() {
    println!("Hello, world!");
    Value::parse(b"i4e").unwrap();
}


mod bencode {
    #[derive(PartialEq, Debug)]
    pub enum Value {
        Int(i32),
//        Str(String),
//        List(),
//        Val(Value)
//        Val(Box<Value>)
    }


    impl Value {
        pub fn parse(arg: &[u8]) -> Result<Vec<Value>, &'static str> {
            let mut result = vec![];
            let mut it = arg.iter();
            while let Some(b) = it.next() {
                if *b == b'i' {
                    let num = match Value::parse_int(&mut it) {
                        Ok(v) => v,
                        Err(desc) => return Err(desc)
                    };
                    result.push(num);
                }
            }

            Ok(result)
        }

        fn parse_int(it : &mut std::slice::Iter<u8>) -> Result<Value, &'static str> {
            let mut nums = vec![];
            while let Some(b) = it.next() {
                if (*b >= b'0' && *b <= b'9') || *b == b'-' {
                    nums.push(*b);
                } else if *b == b'e' {
                    let num_str = match String::from_utf8(nums) {
                        Ok(v) => v,
                        Err(_e) => return Err("Unable convert bytes to string")
                    };
                    let num : i32 = match num_str.parse() {
                        Ok(v) => v,
                        Err(_e) => return Err("Unable convert string to int")
                    };
                    return Ok(Value::Int(num))
                }
            }

            Err("Missing terminate character 'e' for int parsing")
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use bencode::*;

    #[test]
    fn empty_input() {
        assert_eq!(Value::parse(b""), Ok(vec![]));
    }

    #[test]
    fn parse_int_missing_e() {
        assert_eq!(Value::parse(b"i"), Err("Missing terminate character 'e' for int parsing"));
    }

    #[test]
    fn parse_int_missing_value() {
        assert_eq!(Value::parse(b"ie"), Err("Unable convert string to int"));
    }

    #[test]
    fn parse_int_incorrect_value1() {
        assert_eq!(Value::parse(b"i-e"), Err("Unable convert string to int"));
    }

    #[test]
    fn parse_int_incorrect_value2() {
        assert_eq!(Value::parse(b"i--4e"), Err("Unable convert string to int"));
    }

    #[test]
    fn parse_int_incorrect_value3() {
        assert_eq!(Value::parse(b"i-4-e"), Err("Unable convert string to int"));
    }

//    #[test]
//    fn parse_int_incorrect_value4() {
//        assert_eq!(Value::parse(b"i+4e"), Err("Unable convert string to int"));
//    }

    #[test]
    fn positive_int() {
        assert_eq!(Value::parse(b"i4e"), Ok(vec![Value::Int(4)]));
    }

    #[test]
    fn negative_int() {
        assert_eq!(Value::parse(b"i-4e"), Ok(vec![Value::Int(-4)]));
    }

    #[test]
    fn two_ints() {
        assert_eq!(Value::parse(b"i2ei-3e"), Ok(vec![Value::Int(2), Value::Int(-3)]));
    }
}