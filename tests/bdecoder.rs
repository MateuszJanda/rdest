use rdest::hashmap;
use rdest::{BDecoder, BValue, Error};

#[test]
fn empty_input() {
    assert_eq!(BDecoder::from_array(b""), Ok(vec![]));
}

#[test]
fn incorrect_character() {
    assert_eq!(
        BDecoder::from_array(b"x"),
        Err(Error::Decode("Loop [0]: Incorrect character".into()))
    );
}

#[test]
fn byte_str() {
    assert_eq!(
        BDecoder::from_array(b"9:spamIsLoL"),
        Ok(vec![BValue::ByteStr(b"spamIsLoL".to_vec())])
    );
}

#[test]
fn byte_str_unexpected_end() {
    assert_eq!(
        BDecoder::from_array(b"4"),
        Err(Error::Decode("ByteStr [0]: Not enough characters".into()))
    );
}

#[test]
fn byte_str_missing_value() {
    assert_eq!(
        BDecoder::from_array(b"4:"),
        Err(Error::Decode("ByteStr [0]: Not enough characters".into()))
    );
}

#[test]
fn byte_str_not_enough_characters() {
    assert_eq!(
        BDecoder::from_array(b"4:spa"),
        Err(Error::Decode("ByteStr [0]: Not enough characters".into()))
    );
}

#[test]
fn byte_str_invalid_len_character() {
    assert_eq!(
        BDecoder::from_array(b"4+3:spa"),
        Err(Error::Decode("ByteStr [0]: Incorrect character".into()))
    );
}

#[test]
fn byte_str_zero_length() {
    assert_eq!(
        BDecoder::from_array(b"0:"),
        Ok(vec![BValue::ByteStr(vec![])])
    );
}

#[test]
fn int_missing_e() {
    assert_eq!(
        BDecoder::from_array(b"i"),
        Err(Error::Decode(
            "Int [0]: Missing terminate character 'e'".into()
        ))
    );
}

#[test]
fn int_missing_value() {
    assert_eq!(
        BDecoder::from_array(b"ie"),
        Err(Error::Decode("Int [0]: Unable convert to int".into()))
    );
}

#[test]
fn int_incorrect_format1() {
    assert_eq!(
        BDecoder::from_array(b"i-e"),
        Err(Error::Decode("Int [0]: Unable convert to int".into()))
    );
}

#[test]
fn int_incorrect_format2() {
    assert_eq!(
        BDecoder::from_array(b"i--4e"),
        Err(Error::Decode("Int [0]: Unable convert to int".into()))
    );
}

#[test]
fn int_incorrect_format3() {
    assert_eq!(
        BDecoder::from_array(b"i-4-e"),
        Err(Error::Decode("Int [0]: Unable convert to int".into()))
    );
}

#[test]
fn int_incorrect_character() {
    assert_eq!(
        BDecoder::from_array(b"i+4e"),
        Err(Error::Decode("Int [0]: Incorrect character".into()))
    );
}

#[test]
fn int_leading_zero() {
    assert_eq!(
        BDecoder::from_array(b"i01e"),
        Err(Error::Decode("Int [0]: Leading zero".into()))
    );
}

#[test]
fn int_leading_zero_for_negative() {
    assert_eq!(
        BDecoder::from_array(b"i-01e"),
        Err(Error::Decode("Int [0]: Leading zero".into()))
    );
}

#[test]
fn int_zero() {
    assert_eq!(BDecoder::from_array(b"i0e"), Ok(vec![BValue::Int(0)]));
}

#[test]
fn int_positive() {
    assert_eq!(BDecoder::from_array(b"i4e"), Ok(vec![BValue::Int(4)]));
}

#[test]
fn int_negative() {
    assert_eq!(BDecoder::from_array(b"i-4e"), Ok(vec![BValue::Int(-4)]));
}

#[test]
fn int_above_u32() {
    assert_eq!(
        BDecoder::from_array(b"i4294967297e"),
        Ok(vec![BValue::Int(4294967297)])
    );
}

// TODO: bit int support needed
//    fn int_above_i64() {
//        assert_eq!(BDecoder::from_array(b"i9223372036854775808e"), Ok(vec![BValue::Int(9223372036854775808)]));
//    }

#[test]
fn list_of_strings() {
    assert_eq!(
        BDecoder::from_array(b"l4:spam4:eggse"),
        Ok(vec![BValue::List(vec![
            BValue::ByteStr(b"spam".to_vec()),
            BValue::ByteStr(b"eggs".to_vec())
        ])])
    );
}

#[test]
fn list_of_ints() {
    assert_eq!(
        BDecoder::from_array(b"li1ei9ee"),
        Ok(vec![BValue::List(vec![BValue::Int(1), BValue::Int(9)])])
    );
}

#[test]
fn list_of_nested_values() {
    assert_eq!(
        BDecoder::from_array(b"lli1ei5ee3:abce"),
        Ok(vec![BValue::List(vec![
            BValue::List(vec![BValue::Int(1), BValue::Int(5)]),
            BValue::ByteStr(b"abc".to_vec())
        ])])
    );
}

#[test]
fn dict_odd_number_of_elements() {
    assert_eq!(
        BDecoder::from_array(b"di1ee"),
        Err(Error::Decode("Dict [0]: Odd number of elements".into()))
    );
}

#[test]
fn dict_key_not_string() {
    assert_eq!(
        BDecoder::from_array(b"di1ei1ee"),
        Err(Error::Decode("Dict [0]: Key not string".into()))
    );
}

#[test]
fn dict() {
    assert_eq!(
        BDecoder::from_array(b"d1:ki5ee"),
        Ok(vec![BValue::Dict(hashmap![vec![b'k'] => BValue::Int(5)]),])
    );
}

#[test]
fn two_ints() {
    assert_eq!(
        BDecoder::from_array(b"i2ei-3e"),
        Ok(vec![BValue::Int(2), BValue::Int(-3)])
    );
}

#[test]
fn empty_string_and_int() {
    assert_eq!(
        BDecoder::from_array(b"0:i4e"),
        Ok(vec![BValue::ByteStr(vec![]), BValue::Int(4)])
    );
}

#[test]
fn incorrect_value_char_pointer_change() {
    assert_eq!(
        BDecoder::from_array(b"i1ei2ei01e"),
        Err(Error::Decode("Int [6]: Leading zero".into()))
    );
}
