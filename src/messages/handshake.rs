// Copyright 2020 Mateusz Janda.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::constants::{HASH_SIZE, PEER_ID_SIZE};
use crate::serializer::Serializer;
use crate::Error;
use std::io::Cursor;

#[derive(Debug)]
pub struct Handshake {
    info_hash: [u8; HASH_SIZE],
    peer_id: [u8; PEER_ID_SIZE],
}

impl Handshake {
    const LEN: u32 = (Handshake::PROTOCOL_ID.len()
        + Handshake::RESERVED_SIZE
        + Handshake::INFO_HASH_SIZE
        + Handshake::PEER_ID_SIZE) as u32;
    const PROTOCOL_ID: &'static [u8; 19] = b"BitTorrent protocol";
    pub const ID_FROM_PROTOCOL: u8 = Handshake::PROTOCOL_ID[3];
    const LEN_SIZE: usize = 1;
    const RESERVED_SIZE: usize = 8;
    const INFO_HASH_SIZE: usize = HASH_SIZE;
    const PEER_ID_SIZE: usize = PEER_ID_SIZE;
    const FULL_SIZE: usize = Handshake::LEN_SIZE + Handshake::LEN as usize;

    pub fn new(info_hash: &[u8; HASH_SIZE], peer_id: &[u8; PEER_ID_SIZE]) -> Handshake {
        Handshake {
            info_hash: info_hash.clone(),
            peer_id: peer_id.clone(),
        }
    }

    pub fn from(crs: &Cursor<&[u8]>) -> Handshake {
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

    pub fn check(
        crs: &Cursor<&[u8]>,
        protocol_id_length: usize,
        available_data: usize,
    ) -> Result<usize, Error> {
        if protocol_id_length == Handshake::PROTOCOL_ID.len() {
            if available_data < Handshake::FULL_SIZE {
                return Err(Error::Incomplete("Handshake"));
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

    pub fn peer_id(&self) -> &[u8; PEER_ID_SIZE] {
        &self.peer_id
    }

    pub fn validate(
        &self,
        info_hash: &[u8; HASH_SIZE],
        peer_id: &Option<[u8; PEER_ID_SIZE]>,
    ) -> Result<(), Error> {
        if self
            .info_hash
            .iter()
            .enumerate()
            .any(|(idx, b)| *b != info_hash[idx])
        {
            return Err(Error::InvalidInfoHash);
        }

        match peer_id {
            Some(peer_id) => {
                if self
                    .peer_id
                    .iter()
                    .enumerate()
                    .any(|(idx, b)| *b != peer_id[idx])
                {
                    return Err(Error::InvalidPeerId);
                }
                Ok(())
            }
            None => Ok(()),
        }
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
