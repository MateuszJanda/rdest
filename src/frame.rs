use crate::constant::{MSG_ID_POS, MSG_ID_SIZE, MSG_LEN_SIZE, PIECE_BLOCK_SIZE};
use crate::messages::bitfield::Bitfield;
use crate::messages::choke::Choke;
use crate::messages::handshake::Handshake;
use crate::messages::have::Have;
use crate::messages::interested::Interested;
use crate::messages::keep_alive::KeepAlive;
use crate::messages::not_interested::NotInterested;
use crate::messages::unchoke::Unchoke;
use crate::serializer::Serializer;
use crate::Error;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::io::Cursor;

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
}

#[derive(Debug)]
pub struct Request {
    index: u32,
    block_begin: u32,
    block_length: u32,
}

impl Request {
    const LEN: u32 = 13;
    const ID: u8 = 6;
    const LEN_SIZE: usize = MSG_LEN_SIZE;
    const ID_SIZE: usize = MSG_ID_SIZE;
    const INDEX_SIZE: usize = 4;
    const BEGIN_SIZE: usize = 4;
    const LENGTH_SIZE: usize = 4;
    const FULL_SIZE: usize = Request::LEN_SIZE + Request::LEN as usize;

    pub fn new(index: usize, block_begin: usize, block_length: usize) -> Request {
        Request {
            index: index as u32,
            block_begin: block_begin as u32,
            block_length: block_length as u32,
        }
    }

    fn from(crs: &Cursor<&[u8]>) -> Request {
        let start = Request::LEN_SIZE + Request::ID_SIZE;
        let mut index = [0; Request::INDEX_SIZE];
        index.copy_from_slice(&crs.get_ref()[start..start + Request::INDEX_SIZE]);

        let start = start + Request::INDEX_SIZE;
        let mut block_begin = [0; Request::BEGIN_SIZE];
        block_begin.copy_from_slice(&crs.get_ref()[start..start + Request::BEGIN_SIZE]);

        let start = start + Request::BEGIN_SIZE;
        let mut block_length = [0; Request::LENGTH_SIZE];
        block_length.copy_from_slice(&crs.get_ref()[start..start + Request::LENGTH_SIZE]);

        Request {
            index: u32::from_be_bytes(index),
            block_begin: u32::from_be_bytes(block_begin),
            block_length: u32::from_be_bytes(block_length),
        }
    }

    fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        if length == Request::LEN as usize && available_data >= Request::LEN_SIZE + length {
            return Ok(Request::FULL_SIZE);
        }

        Err(Error::Incomplete("Request".into()))
    }

    pub fn index(&self) -> usize {
        self.index as usize
    }

    pub fn block_begin(&self) -> usize {
        self.block_begin as usize
    }

    pub fn block_length(&self) -> usize {
        self.block_length as usize
    }

    pub fn validate(&self, piece_length: Option<usize>, pieces_num: usize) -> Result<(), Error> {
        if self.index >= pieces_num as u32 {
            return Err(Error::InvalidIndex("Request".into()));
        }

        if self.block_length >= PIECE_BLOCK_SIZE as u32 {
            return Err(Error::InvalidLength("Request".into()));
        }

        if let Some(piece_length) = piece_length {
            if self.block_begin + self.block_length > piece_length as u32 {
                return Err(Error::InvalidLength("Request".into()));
            }
        }

        Ok(())
    }
}

impl Serializer for Request {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&Request::LEN.to_be_bytes());
        vec.push(Request::ID);
        vec.extend_from_slice(&self.index.to_be_bytes());
        vec.extend_from_slice(&self.block_begin.to_be_bytes());
        vec.extend_from_slice(&self.block_length.to_be_bytes());

        vec
    }
}

#[derive(Debug)]
pub struct Piece {
    index: u32,
    block_begin: u32,
    block: Vec<u8>,
}

impl Piece {
    const ID: u8 = 7;
    const LEN_SIZE: usize = MSG_LEN_SIZE;
    const ID_SIZE: usize = MSG_ID_SIZE;
    const INDEX_SIZE: usize = 4;
    const BEGIN_SIZE: usize = 4;
    const MIN_LEN: usize = 9;

    pub fn new(index: usize, block_begin: usize, block: Vec<u8>) -> Piece {
        Piece {
            index: index as u32,
            block_begin: block_begin as u32,
            block,
        }
    }

    fn from(crs: &Cursor<&[u8]>) -> Piece {
        let start = Piece::LEN_SIZE + Piece::ID_SIZE;
        let mut index = [0; Piece::INDEX_SIZE];
        index.copy_from_slice(&crs.get_ref()[start..start + Piece::INDEX_SIZE]);

        let start = start + Piece::INDEX_SIZE;
        let mut block_begin = [0; Piece::BEGIN_SIZE];
        block_begin.copy_from_slice(&crs.get_ref()[start..start + Piece::BEGIN_SIZE]);

        let start = start + Piece::BEGIN_SIZE;
        let end = crs.position() as usize;
        let mut block = vec![];
        block.extend_from_slice(&crs.get_ref()[start..end]);

        Piece {
            index: u32::from_be_bytes(index),
            block_begin: u32::from_be_bytes(block_begin),
            block,
        }
    }

    fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        if length >= Piece::MIN_LEN && available_data >= Piece::LEN_SIZE + length {
            return Ok(Piece::LEN_SIZE + length);
        }

        Err(Error::Incomplete("Piece".into()))
    }

    pub fn block_begin(&self) -> usize {
        self.block_begin as usize
    }

    pub fn block_length(&self) -> usize {
        self.block.len()
    }

    pub fn block(&self) -> &Vec<u8> {
        &self.block
    }

    pub fn validate(
        &self,
        index: usize,
        block_begin: usize,
        block_length: usize,
    ) -> Result<(), Error> {
        if self.index as usize != index {
            return Err(Error::InvalidIndex("Piece".into()));
        }

        if self.block_begin as usize != block_begin {
            return Err(Error::InvalidIndex("Piece".into()));
        }

        if self.block.len() as usize != block_length {
            return Err(Error::InvalidLength("Piece".into()));
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
        vec.extend_from_slice(&self.block_begin.to_be_bytes());
        vec.extend_from_slice(self.block.as_slice());

        vec
    }
}

#[derive(Debug)]
pub struct Cancel {
    index: u32,
    block_begin: u32,
    block_length: u32,
}

impl Cancel {
    const LEN: u32 = 13;
    const ID: u8 = 8;
    const LEN_SIZE: usize = MSG_LEN_SIZE;
    const ID_SIZE: usize = MSG_ID_SIZE;
    const INDEX_SIZE: usize = 4;
    const BEGIN_SIZE: usize = 4;
    const LENGTH_SIZE: usize = 4;
    const FULL_SIZE: usize = Cancel::LEN_SIZE + Cancel::LEN as usize;

    fn from(crs: &Cursor<&[u8]>) -> Cancel {
        let start = Cancel::LEN_SIZE + Cancel::ID_SIZE;
        let mut index = [0; Cancel::INDEX_SIZE];
        index.copy_from_slice(&crs.get_ref()[start..start + Cancel::INDEX_SIZE]);

        let start = start + Cancel::INDEX_SIZE;
        let mut block_begin = [0; Cancel::BEGIN_SIZE];
        block_begin.clone_from_slice(&crs.get_ref()[start..start + Cancel::BEGIN_SIZE]);

        let start = start + Cancel::BEGIN_SIZE;
        let mut block_length = [0; Cancel::LENGTH_SIZE];
        block_length.clone_from_slice(&crs.get_ref()[start..start + Cancel::LENGTH_SIZE]);

        Cancel {
            index: u32::from_be_bytes(index),
            block_begin: u32::from_be_bytes(block_begin),
            block_length: u32::from_be_bytes(block_length),
        }
    }

    fn check(available_data: usize, length: usize) -> Result<usize, Error> {
        if length == Cancel::LEN as usize && available_data >= Cancel::LEN_SIZE + length {
            return Ok(Cancel::FULL_SIZE);
        }

        Err(Error::Incomplete("Cancel".into()))
    }
}

impl Serializer for Cancel {
    fn data(&self) -> Vec<u8> {
        let mut vec = vec![];
        vec.extend_from_slice(&Cancel::LEN.to_be_bytes());
        vec.push(Cancel::ID);
        vec.extend_from_slice(&self.index.to_be_bytes());
        vec.extend_from_slice(&self.block_begin.to_be_bytes());
        vec.extend_from_slice(&self.block_length.to_be_bytes());

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
            None => {
                // To skip unknown message
                crs.set_position((MSG_LEN_SIZE + length) as u64);
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

        Err(Error::Incomplete("Protocol ID getter".into()))
    }

    fn get_message_length(crs: &Cursor<&[u8]>) -> Result<usize, Error> {
        let start = crs.position() as usize;
        let end = crs.get_ref().len();

        if end - start >= MSG_LEN_SIZE as usize {
            let mut b = [0; MSG_LEN_SIZE];
            b.copy_from_slice(&crs.get_ref()[0..MSG_LEN_SIZE]);
            return Ok(u32::from_be_bytes(b) as usize);
        }

        Err(Error::Incomplete("Message length getter".into()))
    }

    fn get_message_id(crs: &Cursor<&[u8]>) -> Result<u8, Error> {
        let start = crs.position() as usize;
        let end = crs.get_ref().len();

        if end - start >= (MSG_LEN_SIZE + MSG_ID_SIZE) as usize {
            return Ok(crs.get_ref()[MSG_ID_POS]);
        }

        Err(Error::Incomplete("Message ID getter".into()))
    }

    fn available_data(crs: &Cursor<&[u8]>) -> usize {
        let start = crs.position() as usize;
        let end = crs.get_ref().len();

        return end - start;
    }
}
