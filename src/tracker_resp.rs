// Copyright 2020 Mateusz Janda.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::bcodec::bvalue::BValue;
use crate::constants::HASH_SIZE;
use crate::{BDecoder, Error};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

/// Response from the tracker.
#[derive(PartialEq, Clone, Debug)]
pub struct TrackerResp {
    interval: u64,
    peers: Vec<PeerAddr>,
}

#[derive(PartialEq, Clone, Debug)]
pub struct PeerAddr {
    ip: String,
    peer_id: [u8; HASH_SIZE],
    port: u64,
}

impl TrackerResp {
    /// Parse tracker response from [bencoded](https://en.wikipedia.org/wiki/Bencode) string.
    pub fn from_bencode(data: &[u8]) -> Result<TrackerResp, Error> {
        let bvalues = BDecoder::from_array(data)?;

        if bvalues.is_empty() {
            return Err(Error::TrackerBEncodeMissing);
        }

        let mut err = Err(Error::TrackerDataMissing);
        for val in bvalues {
            match val {
                BValue::Dict(dict) => match Self::parse(&dict) {
                    Ok(torrent) => return Ok(torrent),
                    Err(e) => err = Err(e),
                },
                _ => (),
            }
        }

        err
    }

    fn parse(dict: &HashMap<Vec<u8>, BValue>) -> Result<TrackerResp, Error> {
        if let Some(reason) = Self::find_failure_reason(dict) {
            return Err(Error::TrackerRespFail(reason));
        }

        let response = TrackerResp {
            interval: Self::find_interval(dict)?,
            peers: Self::find_peers(dict)?,
        };

        Ok(response)
    }

    fn find_failure_reason(dict: &HashMap<Vec<u8>, BValue>) -> Option<String> {
        match dict.get(&b"failure reason".to_vec()) {
            Some(BValue::ByteStr(reason)) => String::from_utf8(reason.to_vec()).ok(),
            _ => None,
        }
    }

    fn find_interval(dict: &HashMap<Vec<u8>, BValue>) -> Result<u64, Error> {
        match dict.get(&b"interval".to_vec()) {
            Some(BValue::Int(interval)) => {
                u64::try_from(*interval).or(Err(Error::TrackerRespFail("interval".into())))
            }
            _ => Err(Error::TrackerIncorrectOrMissing("interval")),
        }
    }

    fn find_peers(dict: &HashMap<Vec<u8>, BValue>) -> Result<Vec<PeerAddr>, Error> {
        match dict.get(&b"peers".to_vec()) {
            Some(BValue::List(peers)) => Ok(Self::peer_list(peers)),
            _ => Err(Error::TrackerIncorrectOrMissing("peers")),
        }
    }

    fn peer_list(list: &Vec<BValue>) -> Vec<PeerAddr> {
        list.iter()
            .filter_map(|elem| match elem {
                BValue::Dict(dict) => Some(dict),
                _ => None,
            })
            .filter_map(|dict| {
                match (
                    dict.get(&b"ip".to_vec()),
                    dict.get(&b"peer id".to_vec()),
                    dict.get(&b"port".to_vec()),
                ) {
                    (
                        Some(BValue::ByteStr(ip)),
                        Some(BValue::ByteStr(peer_id)),
                        Some(BValue::Int(port)),
                    ) => Some((ip, peer_id, port)),
                    _ => None,
                }
            })
            .filter_map(|(ip, peer_id, port)| {
                match (
                    String::from_utf8(ip.to_vec()),
                    peer_id.as_slice().try_into(),
                    u64::try_from(*port),
                ) {
                    (Ok(ip), Ok(peer_id), Ok(port)) => Some(PeerAddr { ip, peer_id, port }),
                    _ => None,
                }
            })
            .collect()
    }

    /// Return addresses and peer ID's.
    pub fn peers(&self) -> Vec<(String, [u8; HASH_SIZE])> {
        self.peers
            .iter()
            .map(|p| (p.ip.clone() + ":" + p.port.to_string().as_str(), p.peer_id))
            .collect()
    }
}
