use crate::Error;
use std::io::Cursor;


pub enum Frame {
    Handshake,
    KeepAlive,
    Choke(Choke),
    Unchoke,
    Interested,
    NotInterested,
    Have,
    Bitfield,
    Request,
    Piece,
    Cancel,
    Port,
}

pub struct Handshake {}

impl Handshake {
    const THIRD_BIT: u8 = b'i';
    const PREFIX_LEN: usize = 1;
}


pub struct Choke {}

impl Choke {
    const ID: u8 = 0;
    const PREFIX_LEN: usize = 2;
}

struct Unchoke {}

impl Unchoke {
    const ID: u8 = 1;
    const PREFIX_LEN: usize = 2;
}

struct Interested {}

impl Interested {
    const ID: u8 = 2;
    const PREFIX_LEN: usize = 2;
}

struct NotInterested {}

impl NotInterested {
    const ID: u8 = 3;
    const PREFIX_LEN: usize = 2;
}

struct Have {}

impl Have {
    const ID: u8 = 4;
    const PREFIX_LEN: usize = 2;
    const LEN: usize = 5;
}

struct Bitfield {}

impl Bitfield {
    const ID: u8 = 5;
    const PREFIX_LEN: usize = 2;
}

struct Request {}

impl Request {
    const ID: u8 = 6;
    const PREFIX_LEN: usize = 2;
    const LEN: usize = 13;
}

struct Piece {}

impl Piece {
    const ID: u8 = 7;
    const PREFIX_LEN: usize = 2;
}

struct Cancel {}

impl Cancel {
    const ID: u8 = 8;
    const PREFIX_LEN: usize = 2;
    const LEN: usize = 13;
}

struct Port {}

impl Port {
    const ID: u8 = 9;
    const PREFIX_LEN: usize = 2;
    const LEN: usize = 3;
}



// enum MessageId {
//     Choke = 0,
//     Unchoke = 1,
//     Interested = 2,
//     NotInterested = 3,
//     Have = 4,
//     Bitfield = 5,
//     Request = 6,
//     Piece = 7,
//     Cancel = 8,
//     Port = 9,
// }
//
//
// impl TryFrom<u8> for MessageId {
//     type Error = ();
//
//     fn try_from(v: u8) -> Result<Self, Self::Error> {
//         match v {
//             x if x == MessageId::Choke as u8 => Ok(MessageId::Choke),
//             x if x == MessageId::Unchoke as u8 => Ok(MessageId::Unchoke),
//             x if x == MessageId::Interested as u8 => Ok(MessageId::Interested),
//             x if x == MessageId::NotInterested as u8 => Ok(MessageId::NotInterested),
//             x if x == MessageId::Have as u8 => Ok(MessageId::Have),
//             x if x == MessageId::Bitfield as u8 => Ok(MessageId::Bitfield),
//             x if x == MessageId::Request as u8 => Ok(MessageId::Request),
//             x if x == MessageId::Piece as u8 => Ok(MessageId::Piece),
//             x if x == MessageId::Cancel as u8 => Ok(MessageId::Cancel),
//             x if x == MessageId::Port as u8 => Ok(MessageId::Port),
//             _ => Err(()),
//         }
//     }
// }

const LENGTH_FIELD_LEN: usize = 2;
const ID_FIELD_LEN: usize = 1;

const HANDSHAKE_PSTR_LEN: usize = 19;

const KEEP_ALIVE_LEN: usize = 0;
const CHOKE_LEN: usize = 1;
const UNCHOKE_LEN: usize = 1;
const INTERESTED_LEN: usize = 1;
const NOT_INTERESTED_LEN: usize = 1;
const HAVE_LEN: usize = 5;
const REQUEST_LEN: usize = 13;
const CANCEL_LEN: usize = 13;
const PORT_LEN: usize = 3;

const MIN_PIECE_LEN: usize = 9;

const HANDSHAKE: &[u8; 19] = b"BitTorrent protocol";

impl Frame {
    pub fn check(src: &mut Cursor<&[u8]>) -> Result<(), Error> {
        let length = Self::get_message_length(src)?;
        if length == KEEP_ALIVE_LEN {
            return Ok(())
        }

        let msg_id = Self::get_message_id(src)?;

        if msg_id == b'i' && Self::get_handshake_length(src)? == HANDSHAKE_PSTR_LEN && Self::available_data(src) - 1 >= HANDSHAKE_PSTR_LEN {
            for idx in 0..HANDSHAKE.len() {
                if src.get_ref()[idx + 1] != HANDSHAKE[idx] {
                    return Err(Error::S("nope".into()))
                }
            }
            return Ok(())
        }

        match msg_id {
            Choke::ID => Ok(()),
            Unchoke::ID => Ok(()),
            Interested::ID => Ok(()),
            NotInterested::ID => Ok(()),
            Have::ID if Self::available_data(src) >= Have::PREFIX_LEN + Have::LEN => Ok(()),
            Bitfield::ID if Self::available_data(src) >= Bitfield::PREFIX_LEN + length => Ok(()),
            Request::ID if Self::available_data(src) >= Have::PREFIX_LEN + Request::LEN => Ok(()),
            Piece::ID if Self::available_data(src) >= Piece::PREFIX_LEN + length => Ok(()),
            Cancel::ID if Self::available_data(src) >= Cancel::PREFIX_LEN + Cancel::LEN => Ok(()),
            Port::ID if Self::available_data(src) >= Port::PREFIX_LEN + Port::LEN => Ok(()),
            _ => Err(Error::S("fuck".into()))
        }
    }

    fn get_handshake_length(src: &Cursor<&[u8]>) -> Result<usize, Error> {
        let start = src.position() as usize;
        let end = src.get_ref().len();

        if end - start >= 1 {
            return Ok(src.get_ref()[0] as usize);
        }

        Err(Error::Incomplete)
    }

    fn get_message_length(src: &Cursor<&[u8]>) -> Result<usize, Error> {
        let start = src.position() as usize;
        let end = src.get_ref().len();

        if end - start >= LENGTH_FIELD_LEN as usize {
            // let b : [u8; 2] = src.get_ref()[0..2];
            let b = [src.get_ref()[0], src.get_ref()[1]];
            return Ok(u16::from_be_bytes(b) as usize);
        }

        Err(Error::Incomplete)
    }


    fn get_message_id(src: &Cursor<&[u8]>) -> Result<u8, Error> {
        let start = src.position() as usize;
        let end = src.get_ref().len();

        if end - start >= (LENGTH_FIELD_LEN + 1) as usize {
            return Ok(src.get_ref()[3]);
        }

        Err(Error::Incomplete)
    }

    fn available_data(src: &Cursor<&[u8]>) -> usize {
        let start = src.position() as usize;
        let end = src.get_ref().len();

        return end - start
    }

    pub fn parse(src: &mut Cursor<&[u8]>) -> Result<Frame, Error>
    {
        let length = Self::get_message_length(src)?;
        if length == KEEP_ALIVE_LEN {
            src.set_position(LENGTH_FIELD_LEN as u64);
            return Ok(Frame::KeepAlive)
        }

        let msg_id = Self::get_message_id(src)?;

        if msg_id == b'i' && Self::get_handshake_length(src)? == HANDSHAKE_PSTR_LEN && Self::available_data(src) - 1 >= HANDSHAKE_PSTR_LEN {
            for idx in 0..HANDSHAKE.len() {
                if src.get_ref()[idx + 1] != HANDSHAKE[idx] {
                    return Err(Error::S("nope".into()))
                }
            }
            return Ok(Frame::Handshake)
        }

        let available_data = Self::available_data(src) - LENGTH_FIELD_LEN;
        match msg_id {
            Choke::ID if length == CHOKE_LEN => {
                src.set_position((LENGTH_FIELD_LEN + length) as u64);
                Ok(Frame::Choke(Choke{}))
            },
            // Ok(MessageId::Unchoke) if length == UNCHOKE_LEN => {
            //     src.set_position((LENGTH_FIELD_LEN + length) as u64);
            //     Ok(Frame::Unchoke)
            // },
            // Ok(MessageId::Interested) if length == INTERESTED_LEN => {
            //     src.set_position((LENGTH_FIELD_LEN + length) as u64);
            //     Ok(Frame::Interested)
            // },
            // Ok(MessageId::NotInterested) if length == NOT_INTERESTED_LEN => {
            //     src.set_position((LENGTH_FIELD_LEN + length) as u64);
            //     Ok(Frame::NotInterested)
            // },
            // Ok(MessageId::Have) if length == HAVE_LEN => {
            //     src.set_position((LENGTH_FIELD_LEN + length) as u64);
            //     Ok(Frame::Have)
            // },
            // Ok(MessageId::Bitfield) if available_data >= length => {
            //     src.set_position((LENGTH_FIELD_LEN + length) as u64);
            //     Ok(Frame::Bitfield)
            // },
            // Ok(MessageId::Request) if length == REQUEST_LEN => {
            //     src.set_position((LENGTH_FIELD_LEN + length) as u64);
            //     Ok(Frame::Request)
            // },
            // Ok(MessageId::Piece) if available_data >= length => {
            //     src.set_position((LENGTH_FIELD_LEN + length) as u64);
            //     Ok(Frame::Piece)
            // },
            // Ok(MessageId::Cancel) if length == CANCEL_LEN => {
            //     src.set_position((LENGTH_FIELD_LEN + length) as u64);
            //     Ok(Frame::Cancel)
            // },
            // Ok(MessageId::Port) if length == PORT_LEN => {
            //     src.set_position((LENGTH_FIELD_LEN + length) as u64);
            //     Ok(Frame::Port)
            // },
            _ => Err(Error::S("fuck".into()))
        }
    }
}