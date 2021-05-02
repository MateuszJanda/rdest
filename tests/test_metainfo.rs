// Copyright 2020 Mateusz Janda.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use rdest::hashmap;
use rdest::{BValue, Error, File, Metainfo};

#[test]
fn find_announce_incorrect() {
    assert_eq!(
        Metainfo::find_announce(&hashmap![b"announce".to_vec() => BValue::Int(5)]),
        Err(Error::MetaIncorrectOrMissing("announce"))
    );
}

#[test]
fn find_announce_ok() {
    assert_eq!(
        Metainfo::find_announce(
            &hashmap![b"announce".to_vec() => BValue::ByteStr(b"ANN".to_vec())]
        ),
        Ok(format!("ANN"))
    );
}

#[test]
fn find_name_incorrect() {
    assert_eq!(
        Metainfo::find_name(
            &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"name".to_vec() => BValue::Int(12)])]
        ),
        Err(Error::MetaIncorrectOrMissing("name"))
    );
}

#[test]
fn find_name_incorrect_info() {
    assert_eq!(
        Metainfo::find_name(&hashmap![b"info".to_vec() => BValue::Int(12)]),
        Err(Error::MetaIncorrectOrMissing("info"))
    );
}

#[test]
fn find_name_ok() {
    assert_eq!(
        Metainfo::find_name(
            &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"name".to_vec() => BValue::ByteStr(b"INFO".to_vec())])]
        ),
        Ok(format!("INFO"))
    );
}

#[test]
fn find_piece_length_incorrect() {
    assert_eq!(
        Metainfo::find_piece_length(
            &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"piece length".to_vec() => BValue::ByteStr(b"BAD".to_vec())])]
        ),
        Err(Error::MetaIncorrectOrMissing("piece length"))
    );
}

#[test]
fn find_piece_length_negative() {
    assert_eq!(
        Metainfo::find_piece_length(
            &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"piece length".to_vec() => BValue::Int(-12)])]
        ),
        Err(Error::MetaInvalidU64("piece length"))
    );
}

#[test]
fn find_piece_length_incorrect_info() {
    assert_eq!(
        Metainfo::find_piece_length(&hashmap![b"info".to_vec() => BValue::Int(12)]),
        Err(Error::MetaIncorrectOrMissing("info"))
    );
}

#[test]
fn find_piece_length_ok() {
    assert_eq!(
        Metainfo::find_piece_length(
            &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"piece length".to_vec() => BValue::Int(12)])]
        ),
        Ok(12)
    );
}

#[test]
fn find_pieces_incorrect() {
    assert_eq!(
        Metainfo::find_pieces(
            &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"pieces".to_vec() => BValue::Int(12)])]
        ),
        Err(Error::MetaIncorrectOrMissing("pieces"))
    );
}

#[test]
fn find_pieces_not_divisible() {
    assert_eq!(
        Metainfo::find_pieces(
            &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"pieces".to_vec() => BValue::ByteStr(b"aaa".to_vec())])]
        ),
        Err(Error::MetaNotDivisible("pieces".into()))
    );
}

#[test]
fn find_pieces_incorrect_info() {
    assert_eq!(
        Metainfo::find_pieces(&hashmap![b"info".to_vec() => BValue::Int(12)]),
        Err(Error::MetaIncorrectOrMissing("info"))
    );
}

#[test]
fn find_pieces_ok() {
    assert_eq!(
        Metainfo::find_pieces(
            &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"pieces".to_vec() => BValue::ByteStr(b"aaaaabbbbbcccccdddddAAAAABBBBBCCCCCDDDDD".to_vec())])]
        ),
        Ok(vec![*b"aaaaabbbbbcccccddddd", *b"AAAAABBBBBCCCCCDDDDD",])
    );
}

#[test]
fn find_length_incorrect() {
    assert_eq!(
        Metainfo::find_length(
            &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"length".to_vec() => BValue::ByteStr(b"BAD".to_vec())])]
        ),
        None
    );
}

#[test]
fn find_length_negative() {
    assert_eq!(
        Metainfo::find_length(
            &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"length".to_vec() => BValue::Int(-12)])]
        ),
        None
    );
}

#[test]
fn find_length_incorrect_info() {
    assert_eq!(
        Metainfo::find_length(&hashmap![b"info".to_vec() => BValue::Int(12)]),
        None
    );
}

#[test]
fn find_length_ok() {
    assert_eq!(
        Metainfo::find_length(
            &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"length".to_vec() => BValue::Int(12)])]
        ),
        Some(12)
    );
}

#[test]
fn find_files_incorrect() {
    assert_eq!(
        Metainfo::find_files(
            &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"files".to_vec() => BValue::ByteStr(b"BAD".to_vec())])]
        ),
        None
    );
}

#[test]
fn find_files_empty_list() {
    assert_eq!(
        Metainfo::find_files(
            &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"files".to_vec() => BValue::List(vec![])])]
        ),
        Some(vec![])
    );
}

#[test]
fn find_files_invalid_dict() {
    assert_eq!(
        Metainfo::find_files(&hashmap![b"info".to_vec() =>
            BValue::Dict(hashmap![b"files".to_vec() =>
                BValue::List(vec![
                    BValue::Dict(hashmap![b"a".to_vec() => BValue::Int(12),
                                          b"b".to_vec() => BValue::ByteStr(b"PATH".to_vec())])
                ])
            ])
        ]),
        Some(vec![])
    );
}

#[test]
fn find_files_invalid_dict_length() {
    assert_eq!(
        Metainfo::find_files(&hashmap![b"info".to_vec() =>
            BValue::Dict(hashmap![b"files".to_vec() =>
                BValue::List(vec![
                    BValue::Dict(hashmap![b"length".to_vec() => BValue::Int(-12),
                                          b"path".to_vec() => BValue::ByteStr(b"PATH".to_vec())])
                ])
            ])
        ]),
        Some(vec![])
    );
}

#[test]
fn find_files_invalid_dict_path() {
    assert_eq!(
        Metainfo::find_files(&hashmap![b"info".to_vec() =>
            BValue::Dict(hashmap![b"files".to_vec() =>
                BValue::List(vec![
                    BValue::Dict(hashmap![b"length".to_vec() => BValue::Int(1),
                                          b"path".to_vec() => BValue::Int(2)])
                ])
            ])
        ]),
        Some(vec![])
    );
}

#[test]
fn find_files_valid_and_invalid_dict() {
    assert_eq!(
        Metainfo::find_files(&hashmap![b"info".to_vec() =>
            BValue::Dict(hashmap![b"files".to_vec() =>
                BValue::List(vec![
                    BValue::Dict(hashmap![b"length".to_vec() => BValue::Int(1),
                                          b"path".to_vec() => BValue::Int(2)]),
                    BValue::Dict(hashmap![b"length".to_vec() => BValue::Int(12),
                                          b"path".to_vec() => BValue::ByteStr(b"PATH".to_vec())]),
                ])
            ])
        ]),
        Some(vec![File {
            length: 12,
            path: format!("PATH")
        }])
    );
}

#[test]
fn empty_input_incorrect() {
    assert_eq!(Metainfo::from_bencode(b""), Err(Error::MetaBEncodeMissing));
}

#[test]
fn incorrect_bencode() {
    assert_eq!(
        Metainfo::from_bencode(b"12"),
        Err(Error::DecodeNotEnoughChars("parse_byte_str", 0))
    );
}

#[test]
fn missing_announce() {
    assert_eq!(
        Metainfo::from_bencode(b"d8:announcei1e4:infod4:name4:NAME6:lengthi111ee"),
        Err(Error::MetaIncorrectOrMissing("announce"))
    );
}

#[test]
fn torrent_incorrect() {
    assert_eq!(Metainfo::from_bencode(b"i12e"), Err(Error::MetaDataMissing));
}

#[test]
fn torrent_missing_length_and_files() {
    assert_eq!(
        Metainfo::from_bencode(b"d8:announce3:URL4:infod4:name4:NAME12:piece lengthi999e6:pieces20:AAAAABBBBBCCCCCDDDDDee"),
        Err(Error::MetaLenOrFilesMissing)
    );
}

#[test]
fn torrent_with_both_length_and_files() {
    assert_eq!(
        Metainfo::from_bencode(b"d8:announce3:URL4:infod4:name4:NAME12:piece lengthi999e6:pieces20:AAAAABBBBBCCCCCDDDDD6:lengthi1e5:filesleee"),
        Err(Error::MetaLenAndFilesConflict)
    );
}

#[test]
fn torrent_with_one_file() {
    let m = Metainfo::from_bencode(b"d8:announce3:URL4:infod4:name4:NAME12:piece lengthi111e6:pieces20:AAAAABBBBBCCCCCDDDDD6:lengthi222eee").unwrap();
    assert_eq!(m.tracker_url(), &"URL".to_string());
    assert_eq!(m.total_length(), 222);
    assert_eq!(m.piece_length(0), 111);
    assert_eq!(m.piece(0), &*b"AAAAABBBBBCCCCCDDDDD");
    assert_eq!(
        m.info_hash(),
        &[
            0xdd, 0x95, 0xec, 0x87, 0x7c, 0x96, 0x6, 0x49, 0xef, 0x7d, 0x2f, 0xd5, 0xcc, 0x95,
            0x56, 0x59, 0x17, 0xaf, 0x35, 0x7c
        ],
        "Hash mismatch"
    );
}

#[test]
fn torrent_with_multi_files() {
    let m = Metainfo::from_bencode(b"d8:announce3:URL4:infod4:name4:NAME12:piece lengthi333e6:pieces20:AAAAABBBBBCCCCCDDDDD5:filesld6:lengthi777e4:path4:PATHeeee").unwrap();
    assert_eq!(m.tracker_url(), &"URL".to_string());
    assert_eq!(m.total_length(), 777);
    assert_eq!(m.piece_length(0), 111);
    assert_eq!(m.piece(0), b"AAAAABBBBBCCCCCDDDDD");
    assert_eq!(
        m.info_hash(),
        &[
            0xeb, 0xa1, 0xf6, 0xa6, 0xd8, 0x7b, 0x44, 0x56, 0xf4, 0xff, 0x6e, 0xfd, 0x3f, 0xe3,
            0xe0, 0xef, 0x41, 0xe9, 0xd, 0xb3
        ],
        "Hash mismatch"
    );
}
