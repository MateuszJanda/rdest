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
pub struct Unchoke {}

impl Unchoke {
    const LEN: u32 = 1;
    pub const ID: u8 = 1;
    const LEN_SIZE: usize = MSG_LEN_SIZE;
    const FULL_SIZE: usize = Unchoke::LEN_SIZE + Unchoke::LEN as usize;

    pub fn new() -> Unchoke {
        Unchoke {}
    }

    pub fn check(length: usize) -> Result<usize, Error> {
        match length == Unchoke::LEN as usize {
            true => Ok(Unchoke::FULL_SIZE),
            false => Err(Error::Incomplete("Unchoke")),
        }
    }
}

impl Serializer for Unchoke {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&Unchoke::LEN.to_be_bytes());
        vec.push(Unchoke::ID);

        vec
    }
}
