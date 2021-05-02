// Copyright 2020 Mateusz Janda.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use rdest::{DeepFinder, RawFinder};

#[test]
fn find_raw_int_value() {
    assert_eq!(
        DeepFinder::find_first("1:k", b"d1:ki-5ee"),
        Some(b"i-5e".to_vec())
    );
}

#[test]
fn find_raw_str_value() {
    assert_eq!(
        DeepFinder::find_first("1:k", b"d1:k4:spame"),
        Some(b"4:spam".to_vec())
    );
}

#[test]
fn find_raw_list_value() {
    assert_eq!(
        DeepFinder::find_first("1:k", b"d1:kli10ei20ee"),
        Some(b"li10ei20ee".to_vec())
    );
}

#[test]
fn find_raw_dict_value() {
    assert_eq!(
        DeepFinder::find_first("1:k", b"i4ed1:kdi5ei0eee"),
        Some(b"di5ei0ee".to_vec())
    );
}

#[test]
fn find_raw_first_find() {
    assert_eq!(
        DeepFinder::find_first("1:k", b"d1:ki1eed1:ki2ee"),
        Some(b"i1e".to_vec())
    );
}

#[test]
fn find_deep_not_found() {
    assert_eq!(DeepFinder::find_first("1:k", b"di0ei1ee"), None);
}

#[test]
fn find_deep_incorrect_bencode() {
    assert_eq!(DeepFinder::find_first("1:k", b"d1:kX4:spame"), None);
}

#[test]
fn find_deep_of_last_key() {
    assert_eq!(
        DeepFinder::find_first("i2e", b"di0ei1ei2ei3ee"),
        Some(b"i3e".to_vec())
    );
}

#[test]
fn find_deep_in_sub_dict() {
    assert_eq!(
        DeepFinder::find_first("i1e", b"i4ed1:kdi1ei9eee"),
        Some(b"i9e".to_vec())
    );
}

#[test]
fn find_deep_in_dict_key() {
    assert_eq!(
        DeepFinder::find_first("i1e", b"ddi1ei9ee1:ke"),
        Some(b"i9e".to_vec())
    );
}

#[test]
fn find_deep_key_as_dict() {
    assert_eq!(
        DeepFinder::find_first("di1ei9ee", b"ddi1ei9ee1:ke"),
        Some(b"1:k".to_vec())
    );
}
