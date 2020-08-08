fn main() {
    println!("Hello, world!");
}

mod bencode {
    pub enum Value {
        Int(i32),
        List()
    }

    impl Value {
        pub fn parse(arg: &[u8]) {
            for &b in arg.iter() {
                if b == b'i' {

                }
            }

        }
    }
}

//struct Bencode {
//
//}
//
//impl Bencode {
//    fn parse(arg : &str) {
//
//    }
//}


#[cfg(test)]
mod tests {
    use super::*;
    use bencode::*;

    #[test]
    fn one_result() {
        assert_eq!(Value::parse(""), ());
    }
}