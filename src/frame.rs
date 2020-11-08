use crate::manager::Status;
use crate::Error;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::io::Cursor;

const LEN_SIZE: usize = 4;
const ID_POS: usize = LEN_SIZE;
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
    const LEN: u32 = (Handshake::PROTOCOL_ID.len()
        + Handshake::RESERVED_SIZE
        + Handshake::INFO_HASH_SIZE
        + Handshake::PEER_ID_SIZE) as u32;
    const PROTOCOL_ID: &'static [u8; 19] = b"BitTorrent protocol";
    const ID_FROM_PROTOCOL: u8 = Handshake::PROTOCOL_ID[3];
    const LEN_SIZE: usize = 1;
    const RESERVED_SIZE: usize = 8;
    const INFO_HASH_SIZE: usize = 20;
    const PEER_ID_SIZE: usize = 20;
    const FULL_SIZE: usize = Handshake::LEN_SIZE + Handshake::LEN as usize;

    pub fn new(info_hash: &[u8; 20], peer_id: &[u8; 20]) -> Handshake {
        Handshake {
            info_hash: info_hash.clone(),
            peer_id: peer_id.clone(),
        }
    }

    fn from(crs: &Cursor<&[u8]>) -> Handshake {
        let start = Handshake::LEN_SIZE + Handshake::PROTOCOL_ID.len() + Handshake::RESERVED_SIZE;
        let mut info_hash = [0; Handshake::INFO_HASH_SIZE];
        info_hash.copy_from_slice(&crs.get_ref()[start..start + Handshake::INFO_HASH_SIZE]);

        let start = Handshake::LEN_SIZE
            + Handshake::PROTOCOL_ID.len()
            + Handshake::RESERVED_SIZE
            + Handshake::INFO_HASH_SIZE;
        let mut peer_id = [0; Handshake::PEER_ID_SIZE];
        peer_id.copy_from_slice(&crs.get_ref()[start..start + Handshake::PEER_ID_SIZE]);

        Handshake { info_hash, peer_id }
    }

    fn check(
        crs: &Cursor<&[u8]>,
        protocol_id_length: usize,
        available_data: usize,
    ) -> Result<usize, Error> {
        if protocol_id_length == Handshake::PROTOCOL_ID.len() {
            if available_data < Handshake::FULL_SIZE {
                return Err(Error::Incomplete);
            }

            for idx in 0..Handshake::PROTOCOL_ID.len() {
                if crs.get_ref()[idx + 1] != Handshake::PROTOCOL_ID[idx] {
                    return Err(Error::InvalidProtocolId);
                }
            }

            return Ok(Handshake::FULL_SIZE);
        }

        return Err(Error::InvalidProtocolId);
    }

    pub fn validate(&self, info_hash: &[u8; 20]) -> Result<(), Error> {
        for i in 0..self.info_hash.len() {
            if info_hash[i] != self.info_hash[i] {
                return Err(Error::InvalidInfoHash);
            }
        }

        Ok(())
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
    const LEN: u32 = 0;
    const LEN_SIZE: usize = LEN_SIZE;
    const FULL_SIZE: usize = KeepAlive::LEN_SIZE;

    pub fn new() -> KeepAlive {
        KeepAlive {}
    }
}

impl Serializer for KeepAlive {
    fn data(&self) -> Vec<u8> {
        KeepAlive::LEN.to_be_bytes().to_vec()
    }
}

#[derive(Debug)]
pub struct Choke {}

impl Choke {
    const LEN: u32 = 1;
    const ID: u8 = 0;
    const LEN_SIZE: usize = LEN_SIZE;
    const FULL_SIZE: usize = Choke::LEN_SIZE + Choke::LEN as usize;

    pub fn new() -> Choke {
        Choke {}
    }

    pub fn check(length: usize) -> Result<usize, Error> {
        if length == Choke::LEN as usize {
            return Ok(Choke::FULL_SIZE);
        }

        Err(Error::Incomplete)
    }
}

impl Serializer for Choke {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&Choke::LEN.to_be_bytes());
        vec.push(Choke::ID);

        vec
    }
}

#[derive(Debug)]
pub struct Unchoke {}

impl Unchoke {
    const LEN: u32 = 1;
    const ID: u8 = 1;
    const LEN_SIZE: usize = LEN_SIZE;
    const FULL_SIZE: usize = Unchoke::LEN_SIZE + Unchoke::LEN as usize;

    pub fn new() -> Unchoke {
        Unchoke {}
    }

    pub fn check(length: usize) -> Result<usize, Error> {
        if length == Unchoke::LEN as usize {
            return Ok(Unchoke::FULL_SIZE);
        }

        Err(Error::Incomplete)
    }
}

impl Serializer for Unchoke {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&Unchoke::LEN.to_be_bytes());
        vec.push(Unchoke::ID);

        vec
    }
}

#[derive(Debug)]
pub struct Interested {}

impl Interested {
    const LEN: u32 = 1;
    const ID: u8 = 2;
    const LEN_SIZE: usize = LEN_SIZE;
    const FULL_SIZE: usize = Interested::LEN_SIZE + Interested::LEN as usize;

    pub fn new() -> Interested {
        Interested {}
    }

    pub fn check(length: usize) -> Result<usize, Error> {
        if length == Interested::LEN as usize {
            return Ok(Interested::FULL_SIZE);
        }

        Err(Error::Incomplete)
    }
}

impl Serializer for Interested {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&Interested::LEN.to_be_bytes());
        vec.push(Interested::ID);

        vec
    }
}

#[derive(Debug)]
pub struct NotInterested {}

impl NotInterested {
    const LEN: u32 = 1;
    const ID: u8 = 3;
    const LEN_SIZE: usize = LEN_SIZE;
    const FULL_SIZE: usize = NotInterested::LEN_SIZE + NotInterested::LEN as usize;

    pub fn new() -> NotInterested {
        NotInterested {}
    }

    pub fn check(length: usize) -> Result<usize, Error> {
        if length == NotInterested::LEN as usize {
            return Ok(NotInterested::FULL_SIZE);
        }

        Err(Error::Incomplete)
    }
}

impl Serializer for NotInterested {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&NotInterested::LEN.to_be_bytes());
        vec.push(NotInterested::ID);

        vec
    }
}

#[derive(Debug)]
pub struct Have {
    index: u32,
}

impl Have {
    const LEN: u32 = 5;
    const ID: u8 = 4;
    const LEN_SIZE: usize = LEN_SIZE;
    const ID_SIZE: usize = ID_SIZE;
    const INDEX_SIZE: usize = 4;
    const FULL_SIZE: usize = Have::LEN_SIZE + Have::LEN as usize;

    pub fn new(index: usize) -> Have {
        Have {
            index: index as u32,
        }
    }

    fn from(crs: &Cursor<&[u8]>) -> Have {
        let start = Have::LEN_SIZE + Have::ID_SIZE;
        let mut index = [0; Have::INDEX_SIZE];
        index.copy_from_slice(&crs.get_ref()[start..start + Have::INDEX_SIZE]);

        Have {
            index: u32::from_be_bytes(index),
        }
    }

    fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        if length == Have::LEN as usize && available_data >= Have::LEN_SIZE + length {
            return Ok(Have::FULL_SIZE);
        }

        Err(Error::Incomplete)
    }

    pub(crate) fn index(&self) -> usize {
        self.index as usize
    }

    pub fn validate(&self, pieces_count: usize) -> Result<(), Error> {
        if self.index as usize >= pieces_count {
            return Err(Error::InvalidIndex);
        }

        Ok(())
    }
}

impl Serializer for Have {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&Have::LEN.to_be_bytes());
        vec.push(Have::ID);
        vec.extend_from_slice(&self.index.to_be_bytes());

        vec
    }
}

#[derive(Debug)]
pub struct Bitfield {
    pieces: Vec<u8>,
}

impl Bitfield {
    const ID: u8 = 5;
    const LEN_SIZE: usize = LEN_SIZE;
    const ID_SIZE: usize = ID_SIZE;

    pub fn from_vec(pieces: &Vec<bool>) -> Bitfield {
        let mut v = vec![];

        for p in pieces.chunks(8) {
            let mut byte: u8 = 0;
            for (idx, vv) in p.iter().enumerate() {
                if *vv {
                    byte |= 0b1000_0000 >> idx;
                }
            }

            v.push(byte);
        }

        Bitfield { pieces: v }
    }

    fn from(crs: &Cursor<&[u8]>) -> Bitfield {
        let start = Bitfield::LEN_SIZE + Bitfield::ID_SIZE;
        let end = crs.position() as usize;
        let mut pieces = vec![];
        pieces.extend_from_slice(&crs.get_ref()[start..end]);

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
        if available_data >= Bitfield::LEN_SIZE + length {
            return Ok(Bitfield::LEN_SIZE + length);
        }

        Err(Error::Incomplete)
    }

    pub fn validate(&self, pieces_count: &usize) -> Result<(), Error> {
        if self.available_pieces().len() != *pieces_count {
            return Err(Error::InvalidSize);
        }

        Ok(())
    }
}

impl Serializer for Bitfield {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&((Bitfield::ID_SIZE + self.pieces.len()) as u32).to_be_bytes());
        vec.push(Bitfield::ID);
        vec.extend_from_slice(self.pieces.as_slice());

        vec
    }
}

#[derive(Debug)]
pub struct Request {
    index: u32,
    begin: u32,
    length: u32,
}

impl Request {
    const LEN: u32 = 13;
    const ID: u8 = 6;
    const LEN_SIZE: usize = LEN_SIZE;
    const ID_SIZE: usize = ID_SIZE;
    const INDEX_SIZE: usize = 4;
    const BEGIN_SIZE: usize = 4;
    const LENGTH_SIZE: usize = 4;
    const FULL_SIZE: usize = Request::LEN_SIZE + Request::LEN as usize;

    pub fn new(index: usize, begin: usize, length: usize) -> Request {
        Request {
            index: index as u32,
            begin: begin as u32,
            length: length as u32,
        }
    }

    fn from(crs: &Cursor<&[u8]>) -> Request {
        let start = Request::LEN_SIZE + Request::ID_SIZE;
        let mut index = [0; Request::INDEX_SIZE];
        index.copy_from_slice(&crs.get_ref()[start..start + Request::INDEX_SIZE]);

        let start = start + Request::INDEX_SIZE;
        let mut begin = [0; Request::BEGIN_SIZE];
        begin.copy_from_slice(&crs.get_ref()[start..start + Request::BEGIN_SIZE]);

        let start = start + Request::BEGIN_SIZE;
        let mut length = [0; Request::LENGTH_SIZE];
        length.copy_from_slice(&crs.get_ref()[start..start + Request::LENGTH_SIZE]);

        Request {
            index: u32::from_be_bytes(index),
            begin: u32::from_be_bytes(begin),
            length: u32::from_be_bytes(length),
        }
    }

    fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        if length == Request::LEN as usize && available_data >= Request::LEN_SIZE + length {
            return Ok(Request::FULL_SIZE);
        }

        Err(Error::Incomplete)
    }
}

impl Serializer for Request {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&Request::LEN.to_be_bytes());
        vec.push(Request::ID);
        vec.extend_from_slice(&self.index.to_be_bytes());
        vec.extend_from_slice(&self.begin.to_be_bytes());
        vec.extend_from_slice(&self.length.to_be_bytes());

        vec
    }
}

#[derive(Debug)]
pub struct Piece {
    pub index: u32,
    pub begin: u32,
    pub block: Vec<u8>,
}

impl Piece {
    const ID: u8 = 7;
    const LEN_SIZE: usize = LEN_SIZE;
    const ID_SIZE: usize = ID_SIZE;
    const INDEX_SIZE: usize = 4;
    const BEGIN_SIZE: usize = 4;
    const MIN_LEN: usize = 9;

    pub fn new(index: u32, begin: u32, block: Vec<u8>) -> Piece {
        Piece {
            index,
            begin,
            block,
        }
    }

    fn from(crs: &Cursor<&[u8]>) -> Piece {
        let start = Piece::LEN_SIZE + Piece::ID_SIZE;
        let mut index = [0; Piece::INDEX_SIZE];
        index.copy_from_slice(&crs.get_ref()[start..start + Piece::INDEX_SIZE]);

        let start = start + Piece::INDEX_SIZE;
        let mut begin = [0; Piece::BEGIN_SIZE];
        begin.copy_from_slice(&crs.get_ref()[start..start + Piece::BEGIN_SIZE]);

        let start = start + Piece::BEGIN_SIZE;
        let end = crs.position() as usize;
        let mut block = vec![];
        block.extend_from_slice(&crs.get_ref()[start..end]);

        Piece {
            index: u32::from_be_bytes(index),
            begin: u32::from_be_bytes(begin),
            block,
        }
    }

    fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        if length >= Piece::MIN_LEN && available_data >= Piece::LEN_SIZE + length {
            return Ok(Piece::LEN_SIZE + length);
        }

        Err(Error::Incomplete)
    }

    pub fn validate(&self, pieces_count: usize, piece_size: usize) -> Result<(), Error> {
        if self.index as usize >= pieces_count {
            return Err(Error::InvalidIndex);
        }

        if self.begin as usize + self.block.len() > piece_size {
            return Err(Error::InvalidSize);
        }

        Ok(())
    }
}

impl Serializer for Piece {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&((Piece::ID_SIZE + self.block.len()) as u32).to_be_bytes());
        vec.push(Piece::ID);
        vec.extend_from_slice(&self.index.to_be_bytes());
        vec.extend_from_slice(&self.begin.to_be_bytes());
        vec.extend_from_slice(self.block.as_slice());

        vec
    }
}

#[derive(Debug)]
pub struct Cancel {
    index: u32,
    begin: u32,
    length: u32,
}

impl Cancel {
    const LEN: u32 = 13;
    const ID: u8 = 8;
    const LEN_SIZE: usize = LEN_SIZE;
    const ID_SIZE: usize = ID_SIZE;
    const INDEX_SIZE: usize = 4;
    const BEGIN_SIZE: usize = 4;
    const LENGTH_SIZE: usize = 4;
    const FULL_SIZE: usize = Cancel::LEN_SIZE + Cancel::LEN as usize;

    pub fn new(index: usize, begin: usize, length: usize) -> Cancel {
        Cancel {
            index: index as u32,
            begin: begin as u32,
            length: length as u32,
        }
    }

    fn from(crs: &Cursor<&[u8]>) -> Cancel {
        let start = Cancel::LEN_SIZE + Cancel::ID_SIZE;
        let mut index = [0; Cancel::INDEX_SIZE];
        index.copy_from_slice(&crs.get_ref()[start..start + Cancel::INDEX_SIZE]);

        let start = start + Cancel::INDEX_SIZE;
        let mut begin = [0; Cancel::BEGIN_SIZE];
        begin.clone_from_slice(&crs.get_ref()[start..start + Cancel::BEGIN_SIZE]);

        let start = start + Cancel::BEGIN_SIZE;
        let mut length = [0; Cancel::LENGTH_SIZE];
        length.clone_from_slice(&crs.get_ref()[start..start + Cancel::LENGTH_SIZE]);

        Cancel {
            index: u32::from_be_bytes(index),
            begin: u32::from_be_bytes(begin),
            length: u32::from_be_bytes(length),
        }
    }

    fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        if length == Cancel::LEN as usize && available_data >= Cancel::LEN_SIZE + length {
            return Ok(Cancel::FULL_SIZE);
        }

        Err(Error::Incomplete)
    }
}

impl Serializer for Cancel {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&Cancel::LEN.to_be_bytes());
        vec.push(Cancel::ID);
        vec.extend_from_slice(&self.index.to_be_bytes());
        vec.extend_from_slice(&self.begin.to_be_bytes());
        vec.extend_from_slice(&self.length.to_be_bytes());

        vec
    }
}

#[derive(Debug)]
pub struct Port {
    port: u32,
}

impl Port {
    const LEN: u32 = 3;
    const ID: u8 = 9;
    const LEN_SIZE: usize = LEN_SIZE;
    const ID_SIZE: usize = ID_SIZE;
    const PORT_SIZE: usize = 4;
    const FULL_SIZE: usize = Port::LEN_SIZE + Port::LEN as usize;

    pub fn new(port: u32) -> Port {
        Port { port }
    }

    fn from(crs: &Cursor<&[u8]>) -> Port {
        let start = Port::LEN_SIZE + Port::ID_SIZE;
        let mut listen_port = [0; Port::PORT_SIZE];
        listen_port.copy_from_slice(&crs.get_ref()[start..start + Port::PORT_SIZE]);

        Port {
            port: u32::from_be_bytes(listen_port),
        }
    }

    fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        if length == Port::LEN as usize && available_data >= Port::LEN_SIZE + length {
            return Ok(Port::FULL_SIZE);
        }

        Err(Error::Incomplete)
    }
}

impl Serializer for Port {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&Port::LEN.to_be_bytes());
        vec.push(Port::ID);
        vec.extend_from_slice(&self.port.to_be_bytes());

        vec
    }
}

#[derive(PartialEq, FromPrimitive)]
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
        if length == KeepAlive::LEN as usize {
            crs.set_position(KeepAlive::FULL_SIZE as u64);
            return Ok(Frame::KeepAlive(KeepAlive {}));
        }

        let msg_id = Self::get_message_id(crs)?;

        // TODO: check buffer auto expand and change size to 2**17 or 2**18
        if FromPrimitive::from_u8(msg_id) != Some(MsgId::HandshakeId) && length > 65536 {
            println!("len {}", length);
            return Err(Error::MsgToLarge);
        }

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
                Ok(Frame::Have(Have::from(crs)))
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
                Ok(Frame::Piece(Piece::from(crs)))
            }
            Some(MsgId::CancelId) => {
                crs.set_position(Cancel::check(available_data, length)? as u64);
                Ok(Frame::Cancel(Cancel::from(crs)))
            }
            Some(MsgId::PortId) => {
                crs.set_position(Port::check(available_data, length)? as u64);
                Ok(Frame::Port(Port::from(crs)))
            }
            None => {
                // To skip unknown message
                crs.set_position((LEN_SIZE + length) as u64);
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

        if end - start >= LEN_SIZE as usize {
            let mut b = [0; LEN_SIZE];
            b.copy_from_slice(&crs.get_ref()[0..LEN_SIZE]);
            return Ok(u32::from_be_bytes(b) as usize);
        }

        Err(Error::Incomplete)
    }

    fn get_message_id(crs: &Cursor<&[u8]>) -> Result<u8, Error> {
        let start = crs.position() as usize;
        let end = crs.get_ref().len();

        if end - start >= (LEN_SIZE + ID_SIZE) as usize {
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
