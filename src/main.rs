fn main() {
    println!("Hello, world!");

    fun(b"asdf01234");

}

fn fun(a : &[u8]) {
    for (i, &v) in a.iter().enumerate() {
        println!("{}", v as char);

        if v == b'f' {
            fun1(&a[i..]);
        }
    }
}

fn fun1(a : &[u8]) {
    println!("=====");
    for &v in a.iter() {
        println!("{}", v as char);
    }
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