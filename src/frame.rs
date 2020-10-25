use crate::Error;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::io::Cursor;

const PREFIX_SIZE: usize = 4;
const ID_POS: usize = PREFIX_SIZE;
const ID_SIZE: usize = 1;

#[derive(Debug)]
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

pub trait Serializer {
    fn data(&self) -> Vec<u8>;
}

#[derive(Debug)]
pub struct Handshake {
    info_hash: [u8; 20],
    peer_id: [u8; 20],
}

impl Handshake {
    const PROTOCOL_ID: &'static [u8; 19] = b"BitTorrent protocol";
    const ID_FROM_PROTOCOL: u8 = Handshake::PROTOCOL_ID[3];
    const PREFIX_SIZE: usize = 1;
    const RESERVED_SIZE: usize = 8;
    const INFO_HASH_SIZE: usize = 20;
    const PEER_ID_SIZE: usize = 20;
    const LEN: usize = Handshake::PROTOCOL_ID.len()
        + Handshake::RESERVED_SIZE
        + Handshake::INFO_HASH_SIZE
        + Handshake::PEER_ID_SIZE;
    const FULL_LEN: usize = Handshake::PREFIX_SIZE + Handshake::LEN;

    pub fn new(info_hash: &[u8; 20], peer_id: &[u8; 20]) -> Handshake {
        Handshake {
            info_hash: info_hash.clone(),
            peer_id: peer_id.clone(),
        }
    }

    fn from(crs: &Cursor<&[u8]>) -> Handshake {
        let start =
            Handshake::PREFIX_SIZE + Handshake::PROTOCOL_ID.len() + Handshake::RESERVED_SIZE;
        let mut info_hash = [0; Handshake::INFO_HASH_SIZE];
        info_hash.clone_from_slice(&crs.get_ref()[start..start + Handshake::INFO_HASH_SIZE]);

        let start = Handshake::PREFIX_SIZE
            + Handshake::PROTOCOL_ID.len()
            + Handshake::RESERVED_SIZE
            + Handshake::INFO_HASH_SIZE;
        let mut peer_id = [0; Handshake::PEER_ID_SIZE];
        peer_id.clone_from_slice(&crs.get_ref()[start..start + Handshake::PEER_ID_SIZE]);

        Handshake { info_hash, peer_id }
    }

    pub fn check(
        crs: &Cursor<&[u8]>,
        protocol_id_length: usize,
        available_data: usize,
    ) -> Result<usize, Error> {
        if protocol_id_length == Handshake::PROTOCOL_ID.len() {
            if available_data < Handshake::FULL_LEN {
                return Err(Error::Incomplete);
            }

            for idx in 0..Handshake::PROTOCOL_ID.len() {
                if crs.get_ref()[idx + 1] != Handshake::PROTOCOL_ID[idx] {
                    return Err(Error::Invalid);
                }
            }

            return Ok(Handshake::FULL_LEN);
        }

        return Err(Error::Invalid);
    }
}

impl Serializer for Handshake {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.push(Handshake::PROTOCOL_ID.len() as u8);
        vec.extend_from_slice(Handshake::PROTOCOL_ID);
        vec.extend_from_slice(&[0; Handshake::RESERVED_SIZE]);
        vec.extend_from_slice(&self.info_hash);
        vec.extend_from_slice(&self.peer_id);

        vec
    }
}

#[derive(Debug)]
pub struct KeepAlive {}

impl KeepAlive {
    const LEN: usize = 0;
    const PREFIX_SIZE: usize = PREFIX_SIZE;
    const FULL_LEN: usize = KeepAlive::PREFIX_SIZE;
}

#[derive(Debug)]
pub struct Choke {}

impl Choke {
    const ID: u8 = 0;
    const PREFIX_SIZE: usize = PREFIX_SIZE;
    const LEN: usize = 1;
    const FULL_LEN: usize = Choke::PREFIX_SIZE + Choke::LEN;

    pub fn check(length: usize) -> Result<usize, Error> {
        if length == Choke::LEN {
            return Ok(Choke::FULL_LEN);
        }

        Err(Error::Incomplete)
    }
}

#[derive(Debug)]
pub struct Unchoke {}

impl Unchoke {
    const ID: u8 = 1;
    const PREFIX_SIZE: usize = PREFIX_SIZE;
    const LEN: usize = 1;
    const FULL_LEN: usize = Unchoke::PREFIX_SIZE + Unchoke::LEN;

    pub fn check(length: usize) -> Result<usize, Error> {
        if length == Unchoke::LEN {
            return Ok(Unchoke::FULL_LEN);
        }

        Err(Error::Incomplete)
    }
}

#[derive(Debug)]
pub struct Interested {}

impl Interested {
    const ID: u8 = 2;
    const PREFIX_SIZE: usize = PREFIX_SIZE;
    const LEN: usize = 1;
    const FULL_LEN: usize = Interested::PREFIX_SIZE + Interested::LEN;

    pub fn check(length: usize) -> Result<usize, Error> {
        if length == Interested::LEN {
            return Ok(Interested::FULL_LEN);
        }

        Err(Error::Incomplete)
    }
}

#[derive(Debug)]
pub struct NotInterested {}

impl NotInterested {
    const ID: u8 = 3;
    const PREFIX_SIZE: usize = PREFIX_SIZE;
    const LEN: usize = 1;
    const FULL_LEN: usize = NotInterested::PREFIX_SIZE + NotInterested::LEN;

    pub fn check(length: usize) -> Result<usize, Error> {
        if length == NotInterested::LEN {
            return Ok(NotInterested::FULL_LEN);
        }

        Err(Error::Incomplete)
    }
}

#[derive(Debug)]
pub struct Have {}

impl Have {
    const ID: u8 = 4;
    const PREFIX_SIZE: usize = PREFIX_SIZE;
    const LEN: usize = 5;
    const FULL_LEN: usize = Have::PREFIX_SIZE + Have::LEN;

    fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        if length == Have::LEN && available_data >= Have::PREFIX_SIZE + length {
            return Ok(Have::FULL_LEN);
        }

        Err(Error::Incomplete)
    }
}

#[derive(Debug)]
pub struct Bitfield {
    pieces: Vec<u8>,
}

impl Bitfield {
    const ID: u8 = 5;
    const PREFIX_SIZE: usize = PREFIX_SIZE;

    fn from(crs: &Cursor<&[u8]>) -> Bitfield {
        let end = crs.position() as usize;
        let mut pieces = vec![];
        pieces.extend_from_slice(&crs.get_ref()[..end]);

        Bitfield { pieces }
    }

    pub fn available_pieces(&self) -> Vec<bool> {
        let mut pieces = vec![];
        for b in self.pieces.iter() {
            let mut byte = *b;
            for _ in 0..8 {
                if byte & 0b1000_0000 != 0 {
                    pieces.push(true);
                } else {
                    pieces.push(false);
                }

                byte = byte << 1;
            }
        }

        pieces
    }

    fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        if available_data >= Bitfield::PREFIX_SIZE + length {
            return Ok(Bitfield::PREFIX_SIZE + length);
        }

        Err(Error::Incomplete)
    }
}

#[derive(Debug)]
pub struct Request {
    index: u32,
    begin: u32,
    length: u32,
}

impl Request {
    const LEN: usize = 13;
    const ID: u8 = 6;
    const PREFIX_SIZE: usize = PREFIX_SIZE;
    const ID_SIZE: usize = ID_SIZE;
    const INDEX_LEN: usize = 4;
    const BEGIN_LEN: usize = 4;
    const LENGTH_LEN: usize = 4;
    const FULL_LEN: usize = Request::PREFIX_SIZE + Request::LEN;

    pub fn new(index: usize, begin: usize, length: usize) -> Request {
        Request {
            index: index as u32,
            begin: begin as u32,
            length: length as u32,
        }
    }

    fn from(crs: &Cursor<&[u8]>) -> Request {
        let start = Request::PREFIX_SIZE + Request::ID_SIZE;
        let mut index = [0; Request::INDEX_LEN];
        index.copy_from_slice(&crs.get_ref()[start..start + Request::INDEX_LEN]);

        let start = start + Request::INDEX_LEN;
        let mut begin = [0; Request::BEGIN_LEN];
        begin.clone_from_slice(&crs.get_ref()[start..start + Request::BEGIN_LEN]);

        let start = start + Request::BEGIN_LEN;
        let mut length = [0; Request::LENGTH_LEN];
        length.clone_from_slice(&crs.get_ref()[start..start + Request::LENGTH_LEN]);

        Request {
            index: u32::from_be_bytes(index),
            begin: u32::from_be_bytes(begin),
            length: u32::from_be_bytes(length),
        }
    }

    pub fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&Request::FULL_LEN.to_be_bytes());
        vec.push(Request::ID);
        vec.extend_from_slice(&self.index.to_be_bytes());
        vec.extend_from_slice(&self.begin.to_be_bytes());
        vec.extend_from_slice(&self.length.to_be_bytes());

        vec
    }

    fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        if length == Request::LEN && available_data >= Request::PREFIX_SIZE + length {
            return Ok(Request::FULL_LEN);
        }

        Err(Error::Incomplete)
    }
}

#[derive(Debug)]
pub struct Piece {}

impl Piece {
    const ID: u8 = 7;
    const PREFIX_SIZE: usize = PREFIX_SIZE;
    const MIN_LEN: usize = 9;

    fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        if length >= Piece::MIN_LEN && available_data >= Piece::PREFIX_SIZE + length {
            return Ok(Piece::PREFIX_SIZE + length);
        }

        Err(Error::Incomplete)
    }
}

#[derive(Debug)]
pub struct Cancel {}

impl Cancel {
    const ID: u8 = 8;
    const PREFIX_SIZE: usize = PREFIX_SIZE;
    const LEN: usize = 13;
    const FULL_LEN: usize = Cancel::PREFIX_SIZE + Cancel::LEN;

    fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        if length == Have::LEN && available_data >= Have::PREFIX_SIZE + length {
            return Ok(Have::FULL_LEN);
        }

        Err(Error::Incomplete)
    }
}

#[derive(Debug)]
pub struct Port {}

impl Port {
    const ID: u8 = 9;
    const PREFIX_SIZE: usize = PREFIX_SIZE;
    const LEN: usize = 3;
    const FULL_LEN: usize = Port::PREFIX_SIZE + Port::LEN;

    fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        if length == Port::LEN && available_data >= Port::PREFIX_SIZE + length {
            return Ok(Port::FULL_LEN);
        }

        Err(Error::Incomplete)
    }
}

#[derive(FromPrimitive)]
#[repr(u8)]
enum MsgId {
    HandshakeId = Handshake::ID_FROM_PROTOCOL,
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

impl Frame {
    pub fn parse(crs: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
        let length = Self::get_message_length(crs)?;
        if length == KeepAlive::LEN {
            crs.set_position(KeepAlive::FULL_LEN as u64);
            return Ok(Frame::KeepAlive(KeepAlive {}));
        }

        let msg_id = Self::get_message_id(crs)?;
        let protocol_id_length = Self::get_protocol_id_length(crs)?;
        let available_data = Self::available_data(crs);

        match FromPrimitive::from_u8(msg_id) {
            Some(MsgId::HandshakeId) => {
                crs.set_position(Handshake::check(crs, protocol_id_length, available_data)? as u64);
                Ok(Frame::Handshake(Handshake::from(crs)))
            }
            Some(MsgId::ChokeId) => {
                crs.set_position(Choke::check(length)? as u64);
                Ok(Frame::Choke(Choke {}))
            }
            Some(MsgId::UnchokeId) => {
                crs.set_position(Unchoke::check(length)? as u64);
                Ok(Frame::Unchoke(Unchoke {}))
            }
            Some(MsgId::InterestedId) => {
                crs.set_position(Interested::check(length)? as u64);
                Ok(Frame::Interested(Interested {}))
            }
            Some(MsgId::NotInterestedId) => {
                crs.set_position(NotInterested::check(length)? as u64);
                Ok(Frame::NotInterested(NotInterested {}))
            }
            Some(MsgId::HaveId) => {
                crs.set_position(Have::check(available_data, length)? as u64);
                Ok(Frame::Have(Have {}))
            }
            Some(MsgId::BitfieldId) => {
                crs.set_position(Bitfield::check(available_data, length)? as u64);
                Ok(Frame::Bitfield(Bitfield::from(crs)))
            }
            Some(MsgId::RequestId) => {
                crs.set_position(Request::check(available_data, length)? as u64);
                Ok(Frame::Request(Request::from(crs)))
            }
            Some(MsgId::PieceId) => {
                crs.set_position(Piece::check(available_data, length)? as u64);
                Ok(Frame::Piece(Piece {}))
            }
            Some(MsgId::CancelId) => {
                crs.set_position(Cancel::check(available_data, length)? as u64);
                Ok(Frame::Cancel(Cancel {}))
            }
            Some(MsgId::PortId) => {
                crs.set_position(Port::check(available_data, length)? as u64);
                Ok(Frame::Port(Port {}))
            }
            None => {
                // To skip unknown message
                crs.set_position((PREFIX_SIZE + length) as u64);
                Err(Error::UnknownId(msg_id))
            }
        }
    }

    fn get_protocol_id_length(crs: &Cursor<&[u8]>) -> Result<usize, Error> {
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

        if end - start >= PREFIX_SIZE as usize {
            let mut b = [0; PREFIX_SIZE];
            b.copy_from_slice(&crs.get_ref()[0..PREFIX_SIZE]);
            return Ok(u32::from_be_bytes(b) as usize);
        }

        Err(Error::Incomplete)
    }

    fn get_message_id(crs: &Cursor<&[u8]>) -> Result<u8, Error> {
        let start = crs.position() as usize;
        let end = crs.get_ref().len();

        if end - start >= (PREFIX_SIZE + ID_SIZE) as usize {
            return Ok(crs.get_ref()[ID_POS]);
        }

        Err(Error::Incomplete)
    }

    fn available_data(crs: &Cursor<&[u8]>) -> usize {
        let start = crs.position() as usize;
        let end = crs.get_ref().len();

        return end - start;
    }
}
