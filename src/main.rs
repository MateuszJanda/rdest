fn main() {
    println!("Hello, world!");


}


mod bencode {
    #[derive(PartialEq, Debug)]
    pub enum Value {
        Int(i32),
        Str(String),
//        List(),
//        Val(Value)
//        Val(Box<Value>)
    }


    impl Value {
        pub fn parse(arg: &[u8]) -> Vec<Value> {
            let mut result = vec![];
            let mut it = arg.iter();
            while let Some(b) = it.next() {
                if *b == b'i' {
                    result.push(Value::parse_int(&mut it));
                }
            }

            result
        }

        fn parse_int(it : &mut std::slice::Iter<u8>) -> Value {
            let mut nums = vec![];
            while let Some(b) = it.next() {
                if (*b >= b'0' && *b <= b'9') || *b == b'-' {
                    nums.push(*b);
                } else if *b == b'e' {
                    let nums_str = String::from_utf8(nums).unwrap();
                    let int : i32 = nums_str.parse().unwrap();
                    return Value::Int(int)

                }

            }

            Value::Int(0)
//            println!(i);
        }

    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use bencode::*;

    #[test]
    fn empty_input() {
        assert_eq!(Value::parse(b""), vec![]);
    }

    #[test]
    fn positive_int() {
        assert_eq!(Value::parse(b"i4e"), vec![Value::Int(4)]);
    }

    #[test]
    fn negative_int() {
        assert_eq!(Value::parse(b"i-4e"), vec![Value::Int(-4)]);
    }

    #[test]
    fn two_ints() {
        assert_eq!(Value::parse(b"i2ei-3e"), vec![Value::Int(2), Value::Int(-3)]);
    }
}