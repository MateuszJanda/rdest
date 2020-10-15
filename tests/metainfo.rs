use rdest::hashmap;
use rdest::{BValue, Error, File, Metainfo};

#[test]
fn find_announce_incorrect() {
    assert!(
        Metainfo::find_announce(&hashmap![b"announce".to_vec() => BValue::Int(5)]).is_err(),
        "Incorrect or missing 'announce' value df"
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
            b"aaaaabbbbbcccccddddd".to_vec(),
            b"AAAAABBBBBCCCCCDDDDD".to_vec()
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
fn length_only() {
    let torrent = Metainfo {
        announce: "URL".to_string(),
        name: "NAME".to_string(),
        piece_length: 999,
        pieces: vec![b"AAAAABBBBBCCCCCDDDDD".to_vec()],
        length: Some(111),
        files: None,
        hash: *b"AAAAABBBBBCCCCCDDDDD",
    };
    assert_eq!(torrent.is_valid(), true);
}

#[test]
fn missing_length_and_files() {
    let torrent = Metainfo {
        announce: "URL".to_string(),
        name: "NAME".to_string(),
        piece_length: 999,
        pieces: vec![b"AAAAABBBBBCCCCCDDDDD".to_vec()],
        length: None,
        files: None,
        hash: *b"AAAAABBBBBCCCCCDDDDD",
    };
    assert_eq!(torrent.is_valid(), false);
}

#[test]
fn files_only() {
    let torrent = Metainfo {
        announce: "URL".to_string(),
        name: "NAME".to_string(),
        piece_length: 999,
        pieces: vec![b"AAAAABBBBBCCCCCDDDDD".to_vec()],
        length: None,
        files: Some(vec![]),
        hash: *b"AAAAABBBBBCCCCCDDDDD",
    };
    assert_eq!(torrent.is_valid(), true);
}

#[test]
fn both_length_and_files() {
    let torrent = Metainfo {
        announce: "URL".to_string(),
        name: "NAME".to_string(),
        piece_length: 999,
        pieces: vec![b"AAAAABBBBBCCCCCDDDDD".to_vec()],
        length: Some(111),
        files: Some(vec![]),
        hash: *b"AAAAABBBBBCCCCCDDDDD",
    };
    assert_eq!(torrent.is_valid(), false);
}

#[test]
fn empty_input_incorrect() {
    assert_eq!(
        Metainfo::from_bencode(b""),
        Err(Error::Meta("Empty torrent".into()))
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
        Metainfo::from_bencode(b"d8:announcei1ee"),
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
        Err(Error::Meta("Conflicting values 'length' and 'files'. Only one is allowed".into()))
    );
}

#[test]
fn torrent_correct() {
    assert_eq!(
        Metainfo::from_bencode(b"d8:announce3:URL4:infod4:name4:NAME12:piece lengthi999e6:pieces20:AAAAABBBBBCCCCCDDDDD6:lengthi111eee"),
        Ok(Metainfo {
            announce: "URL".to_string(),
            name : "NAME".to_string(),
            piece_length : 999,
            pieces : vec![b"AAAAABBBBBCCCCCDDDDD".to_vec()],
            length : Some(111),
            files: None,
            hash: *b"AAAAABBBBBCCCCCDDDDD",
        }));
}
