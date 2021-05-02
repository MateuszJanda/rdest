// Copyright 2020 Mateusz Janda.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::constants::MSG_LEN_SIZE;
use crate::serializer::Serializer;

#[derive(Debug)]
pub struct KeepAlive {}

impl KeepAlive {
    pub const LEN: u32 = 0;
    const LEN_SIZE: usize = MSG_LEN_SIZE;
    pub const FULL_SIZE: usize = KeepAlive::LEN_SIZE;

    pub fn new() -> KeepAlive {
        KeepAlive {}
    }
}

impl Serializer for KeepAlive {
    fn data(&self) -> Vec<u8> {
        KeepAlive::LEN.to_be_bytes().to_vec()
    }
}
