use rdest::hashmap;
use rdest::{BValue, Error, File, Metainfo};

#[test]
fn find_announce_incorrect() {
    assert_eq!(
        Metainfo::find_announce(&hashmap![b"announce".to_vec() => BValue::Int(5)]),
        Err(Error::Meta("Incorrect or missing 'announce' value".into()))
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
        Err(Error::Meta("Incorrect or missing 'name' value".into()))
    );
}

#[test]
fn find_name_incorrect_info() {
    assert_eq!(
        Metainfo::find_name(&hashmap![b"info".to_vec() => BValue::Int(12)]),
        Err(Error::Meta("Incorrect or missing 'info' value".into()))
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
        Err(Error::Meta(
            "Incorrect or missing 'piece length' value".into()
        ))
    );
}

#[test]
fn find_piece_length_negative() {
    assert_eq!(
        Metainfo::find_piece_length(
            &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"piece length".to_vec() => BValue::Int(-12)])]
        ),
        Err(Error::Meta("Can't convert 'piece length' to u64".into()))
    );
}

#[test]
fn find_piece_length_incorrect_info() {
    assert_eq!(
        Metainfo::find_piece_length(&hashmap![b"info".to_vec() => BValue::Int(12)]),
        Err(Error::Meta("Incorrect or missing 'info' value".into()))
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
        Err(Error::Meta("Incorrect or missing 'pieces' value".into()))
    );
}

#[test]
fn find_pieces_not_divisible() {
    assert_eq!(
        Metainfo::find_pieces(
            &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"pieces".to_vec() => BValue::ByteStr(b"aaa".to_vec())])]
        ),
        Err(Error::Meta("'pieces' not divisible by 20".into()))
    );
}

#[test]
fn find_pieces_incorrect_info() {
    assert_eq!(
        Metainfo::find_pieces(&hashmap![b"info".to_vec() => BValue::Int(12)]),
        Err(Error::Meta("Incorrect or missing 'info' value".into()))
    );
}

#[test]
fn find_pieces_ok() {
    assert_eq!(
        Metainfo::find_pieces(
            &hashmap![b"info".to_vec() => BValue::Dict(hashmap![b"pieces".to_vec() => BValue::ByteStr(b"aaaaabbbbbcccccdddddAAAAABBBBBCCCCCDDDDD".to_vec())])]
        ),
        Ok(vec![
            *b"aaaaabbbbbcccccddddd",
            *b"AAAAABBBBBCCCCCDDDDD",
        ])
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
    assert_eq!(
        Metainfo::from_bencode(b""),
        Err(Error::Meta("Empty bencode".into()))
    );
}

#[test]
fn incorrect_bencode() {
    assert_eq!(
        Metainfo::from_bencode(b"12"),
        Err(Error::Decode("ByteStr [0]: Not enough characters".into()))
    );
}

#[test]
fn missing_announce() {
    assert_eq!(
        Metainfo::from_bencode(b"d8:announcei1e4:infod4:name4:NAME6:lengthi111ee"),
        Err(Error::Meta("Incorrect or missing 'announce' value".into()))
    );
}

#[test]
fn torrent_incorrect() {
    assert_eq!(
        Metainfo::from_bencode(b"i12e"),
        Err(Error::Meta("Missing data".into()))
    );
}

#[test]
fn torrent_missing_length_and_files() {
    assert_eq!(
        Metainfo::from_bencode(b"d8:announce3:URL4:infod4:name4:NAME12:piece lengthi999e6:pieces20:AAAAABBBBBCCCCCDDDDDee"),
        Err(Error::Meta("Missing 'length' or 'files'".into()))
    );
}

#[test]
fn torrent_with_both_length_and_files() {
    assert_eq!(
        Metainfo::from_bencode(b"d8:announce3:URL4:infod4:name4:NAME12:piece lengthi999e6:pieces20:AAAAABBBBBCCCCCDDDDD6:lengthi1e5:filesleee"),
        Err(Error::Meta("Conflicting 'length' and 'files' values present. Only one is allowed".into()))
    );
}

#[test]
fn torrent_with_one_file() {
    let m = Metainfo::from_bencode(b"d8:announce3:URL4:infod4:name4:NAME12:piece lengthi999e6:pieces20:AAAAABBBBBCCCCCDDDDD6:lengthi111eee").unwrap();
    assert_eq!(m.tracker_url(), "URL".to_string());
    assert_eq!(m.total_length(), 111);
    assert_eq!(m.piece_length(), 999);
    assert_eq!(m.pieces(), vec![*b"AAAAABBBBBCCCCCDDDDD"]);
    assert_eq!(m.info_hash(), [0xaf, 0xee, 0xde, 0xee, 0x6c, 0x1a, 0xb8, 0x35, 0x6b, 0x8e, 0x2a, 0xf, 0x7d, 0xa7, 0x4d, 0x8c, 0x33, 0xe3, 0x68, 0x6a], "Hash mismatch");
}

#[test]
fn torrent_with_multi_files() {
    let m = Metainfo::from_bencode(b"d8:announce3:URL4:infod4:name4:NAME12:piece lengthi999e6:pieces20:AAAAABBBBBCCCCCDDDDD5:filesld6:lengthi777e4:path4:PATHeeee").unwrap();
    assert_eq!(m.tracker_url(), "URL".to_string());
    assert_eq!(m.total_length(), 777);
    assert_eq!(m.piece_length(), 999);
    assert_eq!(m.pieces(), vec![*b"AAAAABBBBBCCCCCDDDDD"]);
    assert_eq!(m.info_hash(), [0x69, 0xc7, 0xa3, 0x43, 0xb4, 0x22, 0x68, 0xaa, 0x00, 0x94, 0xcf, 0x3e, 0x95, 0xa6, 0xfd, 0x48, 0xc4, 0x1f, 0x08, 0xa7], "Hash mismatch");
}