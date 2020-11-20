use crate::constant::HASH_SIZE;

#[allow(unused_macros)]
#[macro_export]
macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = ::std::collections::HashMap::new();
         $( map.insert($key, $val); )*
         map
    }}
}

pub fn hash_to_string(hash: &[u8; HASH_SIZE]) -> String {
    hash.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<String>()
}
