fn main() {
    println!("Hello, world!");

//    fun(b"asdf");

//    let s : String = String::new("text");

//    bencode::Value::parse("l4:spam4:eggse");
//    bencode::Value::parse(b"i4e");
    the_loop(b"asdf")
}


//fn fun(a : &[u8]) {
//    println!("len {}", a.len());
//    for (i, &v) in a.iter().enumerate() {
//        println!("{}", v as char);
//
//        if v == b'l' {
//            fun1(&a[i+1..]);
//        }
//    }
//}
//
//fn fun1(a : &[u8]) {
//
//    println!("=====");
//    println!("len {}", a.len());
//    for &v in a.iter() {
//        println!("> {}", v as char);
//    }
//}

fn the_loop(arg : &[u8]) {
    let mut it = arg.iter();
    while let Some(v) = it.next() {
        println!("{}", *v as char);
        it.nth(0);
    }
}


fn inc(arg : &[u8]) -> &[u8] {
    let r = &arg[1..];
//    println!("{}", r.len());
    r

}

mod bencode {
    pub enum Value {
        Int(i32),
        List(),
//        Val(Value)
        Val(Box<Value>)
    }


    impl Value {
        pub fn parse(arg: &[u8]) {
            for (i, &b) in arg.iter().enumerate() {
                if b == b'i' {
                    let len = Value::parse_int(&arg[i..]);
                    println!("{}", len);
                }
            }

        }

        fn parse_int(arg : &[u8]) -> i32{

            let mut len = Vec::new();

            for (i, &b) in arg.iter().enumerate() {
                if (b >= b'0' && b <= b'9') || b == b'-' {
                    len.push(b);
                } else if b == b'e' {
                    break;
                }
            }

//            println!(i);
            let s = String::from_utf8(len).unwrap();
            let l : i32 = s.parse().unwrap();
            l
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