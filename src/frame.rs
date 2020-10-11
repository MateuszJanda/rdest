use crate::Error;
use std::io::Cursor;


pub enum Frame {
    Handshake(Handshake),
    KeepAlive(KeepAlive),
    Choke(Choke),
    Unchoke(Unchoke),
    Interested(Interested),
    NotInterested(NotInterested),
    Have(Have),
    Bitfield(Bitfield),
    Request(Request),
    Piece(Piece),
    Cancel(Cancel),
    Port(Port),
}

pub struct Handshake {}

impl Handshake {
    const ID_BYTE: u8 = b'i';
    const PREFIX_LEN: usize = 1;
    const LEN: usize = 19;
    const FULL_LEN: usize = Handshake::PREFIX_LEN + Handshake::LEN;
}

pub struct KeepAlive {}

impl KeepAlive {
    const LEN: usize = 0;
    const PREFIX_LEN: usize = 2;
    const FULL_LEN: usize = KeepAlive::PREFIX_LEN;
}

pub struct Choke {}

impl Choke {
    const ID: u8 = 0;
    const PREFIX_LEN: usize = 2;
    const LEN: usize = 1;
    const FULL_LEN: usize = Choke::PREFIX_LEN + Choke::LEN;
}

pub struct Unchoke {}

impl Unchoke {
    const ID: u8 = 1;
    const PREFIX_LEN: usize = 2;
    const LEN: usize = 1;
    const FULL_LEN: usize = Unchoke::PREFIX_LEN +  Unchoke::LEN;
}

pub struct Interested {}

impl Interested {
    const ID: u8 = 2;
    const PREFIX_LEN: usize = 2;
    const LEN: usize = 1;
    const FULL_LEN: usize = Interested::PREFIX_LEN + Interested::LEN;
}

pub struct NotInterested {}

impl NotInterested {
    const ID: u8 = 3;
    const PREFIX_LEN: usize = 2;
    const LEN: usize = 1;
    const FULL_LEN: usize = NotInterested::PREFIX_LEN + NotInterested::LEN;
}

pub struct Have {}

impl Have {
    const ID: u8 = 4;
    const PREFIX_LEN: usize = 2;
    const LEN: usize = 5;
    const FULL_LEN: usize = Have::PREFIX_LEN + Have::LEN;
}

pub struct Bitfield {}

impl Bitfield {
    const ID: u8 = 5;
    const PREFIX_LEN: usize = 2;
}

pub struct Request {}

impl Request {
    const ID: u8 = 6;
    const PREFIX_LEN: usize = 2;
    const LEN: usize = 13;
    const FULL_LEN: usize = Request::PREFIX_LEN + Request::LEN;
}

pub struct Piece {}

impl Piece {
    const ID: u8 = 7;
    const PREFIX_LEN: usize = 2;
    const MIN_LEN: usize = 9;
}

pub struct Cancel {}

impl Cancel {
    const ID: u8 = 8;
    const PREFIX_LEN: usize = 2;
    const LEN: usize = 13;
    const FULL_LEN: usize = Cancel::PREFIX_LEN + Cancel::LEN;
}

pub struct Port {}

impl Port {
    const ID: u8 = 9;
    const PREFIX_LEN: usize = 2;
    const LEN: usize = 3;
    const FULL_LEN: usize = Port::PREFIX_LEN + Port::LEN;
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

const PREFIX_LEN: usize = 2;
const ID_LEN: usize = 1;

// const ID_FIELD_LEN: usize = 1;
//
// const HANDSHAKE_PSTR_LEN: usize = 19;
//
// const KEEP_ALIVE_LEN: usize = 0;
// const CHOKE_LEN: usize = 1;
// const UNCHOKE_LEN: usize = 1;
// const INTERESTED_LEN: usize = 1;
// const NOT_INTERESTED_LEN: usize = 1;
// const HAVE_LEN: usize = 5;
// const REQUEST_LEN: usize = 13;
// const CANCEL_LEN: usize = 13;
// const PORT_LEN: usize = 3;

// const MIN_PIECE_LEN: usize = 9;

const HANDSHAKE: &[u8; 19] = b"BitTorrent protocol";

impl Frame {
    pub fn check(src: &mut Cursor<&[u8]>) -> Result<(), Error> {
        let length = Self::get_message_length(src)?;
        if length == KeepAlive::LEN {
            return Ok(())
        }

        let msg_id = Self::get_message_id(src)?;

        if msg_id == Handshake::ID_BYTE && Self::get_handshake_length(src)? == Handshake::LEN && Self::available_data(src) >= Handshake::FULL_LEN {
            for idx in 0..Handshake::LEN {
                if src.get_ref()[idx + 1] != HANDSHAKE[idx] {
                    return Err(Error::S("nope".into()))
                }
            }
            return Ok(())
        }

        let available_data = Self::available_data(src);
        match msg_id {
            Choke::ID => Ok(()),
            Unchoke::ID => Ok(()),
            Interested::ID => Ok(()),
            NotInterested::ID => Ok(()),
            Have::ID if available_data >= Have::FULL_LEN => Ok(()),
            Bitfield::ID if available_data >= Bitfield::PREFIX_LEN + length => Ok(()),
            Request::ID if available_data >= Have::FULL_LEN => Ok(()),
            Piece::ID if available_data >= Piece::PREFIX_LEN + length => Ok(()),
            Cancel::ID if available_data >= Cancel::FULL_LEN => Ok(()),
            Port::ID if available_data >= Port::FULL_LEN => Ok(()),
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

        if end - start >= PREFIX_LEN as usize {
            let b = [src.get_ref()[0], src.get_ref()[1]];
            return Ok(u16::from_be_bytes(b) as usize);
        }

        Err(Error::Incomplete)
    }


    fn get_message_id(src: &Cursor<&[u8]>) -> Result<u8, Error> {
        let start = src.position() as usize;
        let end = src.get_ref().len();

        if end - start >= (PREFIX_LEN + ID_LEN) as usize {
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
        if length == KeepAlive::LEN {
            src.set_position(KeepAlive::FULL_LEN as u64);
            return Ok(Frame::KeepAlive(KeepAlive{}))
        }

        let msg_id = Self::get_message_id(src)?;

        if msg_id == b'i' && Self::get_handshake_length(src)? == Handshake::LEN && Self::available_data(src) >= Handshake::FULL_LEN {
            for idx in 0..Handshake::LEN {
                if src.get_ref()[idx + 1] != HANDSHAKE[idx] {
                    return Err(Error::S("nope".into()))
                }
            }
            src.set_position(Handshake::FULL_LEN as u64);
            return Ok(Frame::Handshake(Handshake{}))
        }

        let available_data = Self::available_data(src);
        match msg_id {
            Choke::ID if length == Choke::LEN => {
                src.set_position(Choke::FULL_LEN as u64);
                Ok(Frame::Choke(Choke{}))
            },
            Unchoke::ID if length == Unchoke::LEN => {
                src.set_position(Unchoke::FULL_LEN as u64);
                Ok(Frame::Unchoke(Unchoke{}))
            },
            Interested::ID if length == Interested::LEN => {
                src.set_position(Interested::FULL_LEN as u64);
                Ok(Frame::Interested(Interested{}))
            },
            NotInterested::ID if length == NotInterested::LEN => {
                src.set_position(NotInterested::FULL_LEN as u64);
                Ok(Frame::NotInterested(NotInterested{}))
            },
            Have::ID if length == Have::LEN && available_data >= Have::PREFIX_LEN + length => {
                src.set_position(Have::FULL_LEN as u64);
                Ok(Frame::Have(Have{}))
            },
            Bitfield::ID if available_data >= Bitfield::PREFIX_LEN + length => {
                src.set_position((Bitfield::PREFIX_LEN + length) as u64);
                Ok(Frame::Bitfield(Bitfield{}))
            },
            Request::ID if length == Request::LEN && available_data >= Request::PREFIX_LEN + length => {
                src.set_position(Request::FULL_LEN as u64);
                Ok(Frame::Request(Request{}))
            },
            Piece::ID if length >= Piece::MIN_LEN && available_data >= Piece::PREFIX_LEN + length => {
                src.set_position((Piece::PREFIX_LEN + length) as u64);
                Ok(Frame::Piece(Piece{}))
            },
            Cancel::ID if length == Cancel::LEN && available_data >= Cancel::PREFIX_LEN + length => {
                src.set_position(Cancel::FULL_LEN as u64);
                Ok(Frame::Cancel(Cancel{}))
            },
            Port::ID if length == Port::LEN && available_data >= Port::PREFIX_LEN + length => {
                src.set_position(Port::FULL_LEN as u64);
                Ok(Frame::Port(Port{}))
            },
            _ => Err(Error::S("fuck".into()))
        }
    }
}