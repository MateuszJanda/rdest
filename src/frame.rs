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
    const PROTOCOL_ID: &'static [u8; 19] = b"BitTorrent protocol";
    const LEN: usize = Handshake::PROTOCOL_ID.len();
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
    const FULL_LEN: usize = Unchoke::PREFIX_LEN + Unchoke::LEN;
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

const PREFIX_LEN: usize = 2;
const ID_LEN: usize = 1;

impl Frame {
    pub fn check(crs: &mut Cursor<&[u8]>) -> Result<(), Error> {
        let length = Self::get_message_length(crs)?;
        if length == KeepAlive::LEN {
            return Ok(());
        }

        let msg_id = Self::get_message_id(crs)?;

        if msg_id == Handshake::ID_BYTE
            && Self::get_handshake_length(crs)? == Handshake::LEN
            && Self::available_data(crs) >= Handshake::FULL_LEN
        {
            for idx in 0..Handshake::LEN {
                if crs.get_ref()[idx + 1] != Handshake::PROTOCOL_ID[idx] {
                    return Err(Error::Str("nope".into()));
                }
            }
            return Ok(());
        }

        let available_data = Self::available_data(crs);
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
            _ => Err(Error::Str("fuck".into())),
        }
    }

    fn get_handshake_length(crs: &Cursor<&[u8]>) -> Result<usize, Error> {
        let start = crs.position() as usize;
        let end = crs.get_ref().len();

        if end - start >= 1 {
            return Ok(crs.get_ref()[0] as usize);
        }

        Err(Error::Incomplete)
    }

    fn get_message_length(crs: &Cursor<&[u8]>) -> Result<usize, Error> {
        let start = crs.position() as usize;
        let end = crs.get_ref().len();

        if end - start >= PREFIX_LEN as usize {
            let b = [crs.get_ref()[0], crs.get_ref()[1]];
            return Ok(u16::from_be_bytes(b) as usize);
        }

        Err(Error::Incomplete)
    }

    fn get_message_id(crs: &Cursor<&[u8]>) -> Result<u8, Error> {
        let start = crs.position() as usize;
        let end = crs.get_ref().len();

        if end - start >= (PREFIX_LEN + ID_LEN) as usize {
            return Ok(crs.get_ref()[3]);
        }

        Err(Error::Incomplete)
    }

    fn available_data(crs: &Cursor<&[u8]>) -> usize {
        let start = crs.position() as usize;
        let end = crs.get_ref().len();

        return end - start;
    }

    pub fn parse(crs: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
        let length = Self::get_message_length(crs)?;
        if length == KeepAlive::LEN {
            crs.set_position(KeepAlive::FULL_LEN as u64);
            return Ok(Frame::KeepAlive(KeepAlive {}));
        }

        let msg_id = Self::get_message_id(crs)?;

        if msg_id == Handshake::ID_BYTE
            && Self::get_handshake_length(crs)? == Handshake::LEN
            && Self::available_data(crs) >= Handshake::FULL_LEN
        {
            for idx in 0..Handshake::LEN {
                if crs.get_ref()[idx + 1] != Handshake::PROTOCOL_ID[idx] {
                    return Err(Error::Str("nope".into()));
                }
            }
            crs.set_position(Handshake::FULL_LEN as u64);
            return Ok(Frame::Handshake(Handshake {}));
        }

        let available_data = Self::available_data(crs);
        match msg_id {
            Choke::ID if length == Choke::LEN => {
                crs.set_position(Choke::FULL_LEN as u64);
                Ok(Frame::Choke(Choke {}))
            }
            Unchoke::ID if length == Unchoke::LEN => {
                crs.set_position(Unchoke::FULL_LEN as u64);
                Ok(Frame::Unchoke(Unchoke {}))
            }
            Interested::ID if length == Interested::LEN => {
                crs.set_position(Interested::FULL_LEN as u64);
                Ok(Frame::Interested(Interested {}))
            }
            NotInterested::ID if length == NotInterested::LEN => {
                crs.set_position(NotInterested::FULL_LEN as u64);
                Ok(Frame::NotInterested(NotInterested {}))
            }
            Have::ID if length == Have::LEN && available_data >= Have::PREFIX_LEN + length => {
                crs.set_position(Have::FULL_LEN as u64);
                Ok(Frame::Have(Have {}))
            }
            Bitfield::ID if available_data >= Bitfield::PREFIX_LEN + length => {
                crs.set_position((Bitfield::PREFIX_LEN + length) as u64);
                Ok(Frame::Bitfield(Bitfield {}))
            }
            Request::ID
                if length == Request::LEN && available_data >= Request::PREFIX_LEN + length =>
            {
                crs.set_position(Request::FULL_LEN as u64);
                Ok(Frame::Request(Request {}))
            }
            Piece::ID
                if length >= Piece::MIN_LEN && available_data >= Piece::PREFIX_LEN + length =>
            {
                crs.set_position((Piece::PREFIX_LEN + length) as u64);
                Ok(Frame::Piece(Piece {}))
            }
            Cancel::ID
                if length == Cancel::LEN && available_data >= Cancel::PREFIX_LEN + length =>
            {
                crs.set_position(Cancel::FULL_LEN as u64);
                Ok(Frame::Cancel(Cancel {}))
            }
            Port::ID if length == Port::LEN && available_data >= Port::PREFIX_LEN + length => {
                crs.set_position(Port::FULL_LEN as u64);
                Ok(Frame::Port(Port {}))
            }
            _ => Err(Error::Str("fuck".into())),
        }
    }
}
