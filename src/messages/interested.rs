// Copyright 2020 Mateusz Janda.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::constants::MSG_LEN_SIZE;
use crate::serializer::Serializer;
use crate::Error;

#[derive(Debug)]
pub struct Interested {}

impl Interested {
    const LEN: u32 = 1;
    pub const ID: u8 = 2;
    const LEN_SIZE: usize = MSG_LEN_SIZE;
    const FULL_SIZE: usize = Interested::LEN_SIZE + Interested::LEN as usize;

    pub fn new() -> Interested {
        Interested {}
    }

    pub fn check(length: usize) -> Result<usize, Error> {
        match length == Interested::LEN as usize {
            true => Ok(Interested::FULL_SIZE),
            false => Err(Error::Incomplete("Interested")),
        }
    }
}

impl Serializer for Interested {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&Interested::LEN.to_be_bytes());
        vec.push(Interested::ID);

        vec
    }
}
