// Copyright 2020 Mateusz Janda.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::constants::HASH_SIZE;

/// Create new HashMap with emplaced elements.
///
/// # Example
/// ```
/// use rdest::hashmap;
///
/// let h = hashmap![&"a" => 1, &"b" => 2];
///
/// assert_eq!(h[&"a"], 1);
/// assert_ne!(h[&"b"], 3);
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
