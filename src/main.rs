fn main() {
    println!("Hello, world!");
}

mod bencode {
    pub enum Value {}

    impl Value {
        pub fn parse(arg: &str) {
            
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