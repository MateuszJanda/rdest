// Copyright 2020 Mateusz Janda.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/// Dictionary value (raw format) finder in [bencoded](https://en.wikipedia.org/wiki/Bencode) string.
pub trait RawFinder {
    /// Find first value for defined key in [bencoded](https://en.wikipedia.org/wiki/Bencode) string
    /// with dictionaries. Value is returned in raw format.
    fn find_first(key: &str, arg: &[u8]) -> Option<Vec<u8>>;
}
