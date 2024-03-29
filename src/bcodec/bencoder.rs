// Copyright 2020 Mateusz Janda.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::bcodec::bvalue::BValue;
use std::collections::HashMap;

#[derive(PartialEq, Clone, Debug)]
pub struct BEncoder {
    data: Vec<u8>,
}

impl BEncoder {
    pub fn new() -> BEncoder {
        BEncoder { data: vec![] }
    }

    pub fn encode(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn add_int(&mut self, value: i64) -> &mut Self {
        self.data.extend_from_slice("i".as_bytes());
        self.data.extend_from_slice(value.to_string().as_bytes());
        self.data.extend_from_slice("e".as_bytes());
        self
    }

    pub fn add_byte_str(&mut self, value: &[u8]) -> &mut Self {
        self.data
            .extend_from_slice(value.len().to_string().as_bytes());
        self.data.extend_from_slice(":".as_bytes());
        self.data.extend_from_slice(value);
        self
    }

    pub fn add_list(&mut self, values: &Vec<BValue>) -> &mut Self {
        let mut out = BEncoder::new();
        self.data.extend_from_slice("l".as_bytes());
        for value in values {
            match value {
                BValue::Int(i) => out.add_int(*i),
                BValue::ByteStr(b) => out.add_byte_str(b.as_slice()),
                BValue::List(l) => out.add_list(l),
                BValue::Dict(d) => out.add_dict(d),
            };
        }
        self.data.extend_from_slice(out.encode().as_slice());
        self.data.extend_from_slice("e".as_bytes());
        self
    }

    pub fn add_dict(&mut self, values: &HashMap<Vec<u8>, BValue>) -> &mut Self {
        let mut out = BEncoder::new();
        self.data.extend_from_slice("d".as_bytes());

        let mut sorted_values: Vec<_> = values.iter().collect();
        sorted_values.sort_by(|a, b| a.0.cmp(b.0));
        for (key, value) in sorted_values {
            match value {
                BValue::Int(i) => out.add_byte_str(key.as_slice()).add_int(*i),
                BValue::ByteStr(b) => out.add_byte_str(key.as_slice()).add_byte_str(b.as_slice()),
                BValue::List(l) => out.add_byte_str(key.as_slice()).add_list(l),
                BValue::Dict(d) => out.add_byte_str(key.as_slice()).add_dict(d),
            };
        }
        self.data.extend_from_slice(out.encode().as_slice());
        self.data.extend_from_slice("e".as_bytes());
        self
    }
}
