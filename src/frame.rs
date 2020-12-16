use crate::constants::{MSG_ID_POS, MSG_ID_SIZE, MSG_LEN_SIZE};
use crate::messages::bitfield::Bitfield;
use crate::messages::cancel::Cancel;
use crate::messages::choke::Choke;
use crate::messages::handshake::Handshake;
use crate::messages::have::Have;
use crate::messages::interested::Interested;
use crate::messages::keep_alive::KeepAlive;
use crate::messages::not_interested::NotInterested;
use crate::messages::piece::Piece;
use crate::messages::request::Request;
use crate::messages::unchoke::Unchoke;
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
