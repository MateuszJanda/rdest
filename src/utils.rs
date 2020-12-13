use crate::constant::HASH_SIZE;

/// Create new HashMap with emplaced elements.
///
/// # Example
/// ```
/// let h = hashmap![&"a" => 1, &"b" => 2];
///
/// assert_eq!(h[&"a"], 1);
/// assert_eq!(h[&"b"], 3);
/// ```
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
