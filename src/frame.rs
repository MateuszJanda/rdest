use crate::frame::MsgId::HaveId;
use crate::Error;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
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

pub struct Handshake {
    info_hash: [u8; 20],
    peer_id: [u8; 20],
}

impl Handshake {
    const PROTOCOL_ID: &'static [u8; 19] = b"BitTorrent protocol";
    const ID_FROM_PROTOCOL: u8 = Handshake::PROTOCOL_ID[2];
    const PREFIX_LEN: usize = 1;
    const RESERVED_LEN: usize = 8;
    const INFO_HASH_LEN: usize = 20;
    const PEER_ID_LEN: usize = 20;
    const LEN: usize = Handshake::PROTOCOL_ID.len()
        + Handshake::RESERVED_LEN
        + Handshake::INFO_HASH_LEN
        + Handshake::PEER_ID_LEN;
    const FULL_LEN: usize = Handshake::PREFIX_LEN + Handshake::LEN;

    pub fn new(info_hash: &[u8; 20], peer_id: &[u8; 20]) -> Handshake {
        Handshake {
            info_hash: info_hash.clone(),
            peer_id: peer_id.clone(),
        }
    }

    fn from(crs: &Cursor<&[u8]>) -> Handshake {
        let start = Handshake::PREFIX_LEN + Handshake::PROTOCOL_ID.len() + Handshake::RESERVED_LEN;
        let mut info_hash = [0; Handshake::INFO_HASH_LEN];
        info_hash.clone_from_slice(&crs.get_ref()[start..start + Handshake::INFO_HASH_LEN]);

        let start = Handshake::PREFIX_LEN
            + Handshake::PROTOCOL_ID.len()
            + Handshake::RESERVED_LEN
            + Handshake::INFO_HASH_LEN;
        let mut peer_id = [0; Handshake::PEER_ID_LEN];
        peer_id.clone_from_slice(&crs.get_ref()[start..start + Handshake::PEER_ID_LEN]);

        Handshake { info_hash, peer_id }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.push(Handshake::PROTOCOL_ID.len() as u8);
        vec.extend_from_slice(Handshake::PROTOCOL_ID);
        vec.extend_from_slice(&[0; Handshake::RESERVED_LEN]);
        vec.extend_from_slice(&self.info_hash);
        vec.extend_from_slice(&self.peer_id);

        vec
    }
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

#[derive(FromPrimitive)]
#[repr(u8)]
enum MsgId {
    ChokeId = Choke::ID,
    UnchokeId = Unchoke::ID,
    InterestedId = Interested::ID,
    NotInterestedId = NotInterested::ID,
    HaveId = Have::ID,
    BitfieldId = Bitfield::ID,
    RequestId = Request::ID,
    PieceId = Piece::ID,
    CancelId = Cancel::ID,
    PortId = Port::ID,
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

        if msg_id == Handshake::ID_FROM_PROTOCOL
            && Self::get_handshake_length(crs)? == Handshake::LEN
            && Self::available_data(crs) >= Handshake::FULL_LEN
        {
            for idx in 0..Handshake::LEN {
                if crs.get_ref()[idx + 1] != Handshake::PROTOCOL_ID[idx] {
                    return Err(Error::InvalidHeader);
                }
            }
            return Ok(());
        }

        let available_data = Self::available_data(crs);
        match FromPrimitive::from_u8(msg_id) {
            Some(MsgId::ChokeId) => Ok(()),
            Some(MsgId::UnchokeId) => Ok(()),
            Some(MsgId::InterestedId) => Ok(()),
            Some(MsgId::NotInterestedId) => Ok(()),
            Some(MsgId::HaveId) if available_data >= Have::FULL_LEN => Ok(()),
            Some(MsgId::HaveId) => Err(Error::Incomplete),
            Some(MsgId::BitfieldId) if available_data >= Bitfield::PREFIX_LEN + length => Ok(()),
            Some(MsgId::BitfieldId) => Err(Error::Incomplete),
            Some(MsgId::RequestId) if available_data >= Have::FULL_LEN => Ok(()),
            Some(MsgId::RequestId) => Err(Error::Incomplete),
            Some(MsgId::PieceId) if available_data >= Piece::PREFIX_LEN + length => Ok(()),
            Some(MsgId::PieceId) => Err(Error::Incomplete),
            Some(MsgId::CancelId) if available_data >= Cancel::FULL_LEN => Ok(()),
            Some(MsgId::CancelId) => Err(Error::Incomplete),
            Some(MsgId::PortId) if available_data >= Port::FULL_LEN => Ok(()),
            Some(MsgId::PortId) => Err(Error::Incomplete),
            None => Err(Error::UnknownId),
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

        if msg_id == Handshake::ID_FROM_PROTOCOL
            && Self::get_handshake_length(crs)? == Handshake::LEN
            && Self::available_data(crs) >= Handshake::FULL_LEN
        {
            for idx in 0..Handshake::LEN {
                if crs.get_ref()[idx + 1] != Handshake::PROTOCOL_ID[idx] {
                    return Err(Error::InvalidHeader);
                }
            }
            crs.set_position(Handshake::FULL_LEN as u64);
            return Ok(Frame::Handshake(Handshake::from(crs)));
        }

        let available_data = Self::available_data(crs);
        match FromPrimitive::from_u8(msg_id) {
            Some(MsgId::ChokeId) if length == Choke::LEN => {
                crs.set_position(Choke::FULL_LEN as u64);
                Ok(Frame::Choke(Choke {}))
            }
            Some(MsgId::UnchokeId) if length == Unchoke::LEN => {
                crs.set_position(Unchoke::FULL_LEN as u64);
                Ok(Frame::Unchoke(Unchoke {}))
            }
            Some(MsgId::InterestedId) if length == Interested::LEN => {
                crs.set_position(Interested::FULL_LEN as u64);
                Ok(Frame::Interested(Interested {}))
            }
            Some(MsgId::NotInterestedId) if length == NotInterested::LEN => {
                crs.set_position(NotInterested::FULL_LEN as u64);
                Ok(Frame::NotInterested(NotInterested {}))
            }
            Some(MsgId::HaveId)
                if length == Have::LEN && available_data >= Have::PREFIX_LEN + length =>
            {
                crs.set_position(Have::FULL_LEN as u64);
                Ok(Frame::Have(Have {}))
            }
            Some(MsgId::BitfieldId) if available_data >= Bitfield::PREFIX_LEN + length => {
                crs.set_position((Bitfield::PREFIX_LEN + length) as u64);
                Ok(Frame::Bitfield(Bitfield {}))
            }
            Some(MsgId::RequestId)
                if length == Request::LEN && available_data >= Request::PREFIX_LEN + length =>
            {
                crs.set_position(Request::FULL_LEN as u64);
                Ok(Frame::Request(Request {}))
            }
            Some(MsgId::PieceId)
                if length >= Piece::MIN_LEN && available_data >= Piece::PREFIX_LEN + length =>
            {
                crs.set_position((Piece::PREFIX_LEN + length) as u64);
                Ok(Frame::Piece(Piece {}))
            }
            Some(MsgId::CancelId)
                if length == Cancel::LEN && available_data >= Cancel::PREFIX_LEN + length =>
            {
                crs.set_position(Cancel::FULL_LEN as u64);
                Ok(Frame::Cancel(Cancel {}))
            }
            Some(MsgId::PortId)
                if length == Port::LEN && available_data >= Port::PREFIX_LEN + length =>
            {
                crs.set_position(Port::FULL_LEN as u64);
                Ok(Frame::Port(Port {}))
            }
            _ => {
                // Skip unknown message
                crs.set_position((PREFIX_LEN + length) as u64);
                Err(Error::UnknownId)
            }
        }
    }
}
