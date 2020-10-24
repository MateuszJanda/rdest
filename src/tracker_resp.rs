use crate::{BDecoder, BValue, Error};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs;

#[derive(PartialEq, Clone, Debug)]
pub struct TrackerResp {
    pub interval: u64,
    pub peers: Vec<Peer>,
}

#[derive(PartialEq, Clone, Debug)]
pub struct Peer {
    ip: String,
    peer_id: String,
    port: u64,
}

impl TrackerResp {
    pub fn from_file(path: String) -> Result<TrackerResp, Error> {
        match &fs::read(path) {
            Ok(val) => Self::from_bencode(val),
            Err(_) => Err(Error::Tracker("File not found".into())),
        }
    }

    pub fn from_bencode(data: &[u8]) -> Result<TrackerResp, Error> {
        let bvalues = BDecoder::from_array(data)?;

        if bvalues.is_empty() {
            return Err(Error::Tracker("Empty bencode".into()));
        }

        let mut err = Err(Error::Tracker("Missing data".into()));
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
            return Err(Error::Tracker(reason));
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
            Some(BValue::Int(interval)) => u64::try_from(*interval).or(Err(Error::Tracker(
                "Can't convert 'interval' to u64".into(),
            ))),
            _ => Err(Error::Tracker(
                "Incorrect or missing 'interval' value".into(),
            )),
        }
    }

    fn find_peers(dict: &HashMap<Vec<u8>, BValue>) -> Result<Vec<Peer>, Error> {
        match dict.get(&b"peers".to_vec()) {
            Some(BValue::List(peers)) => Ok(Self::peer_list(peers)),
            _ => Err(Error::Tracker("Incorrect or missing 'peers' value".into())),
        }
    }

    fn peer_list(list: &Vec<BValue>) -> Vec<Peer> {
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
                    String::from_utf8(peer_id.to_vec()),
                    u64::try_from(*port),
                ) {
                    (Ok(ip), Ok(peer_id), Ok(port)) => Some(Peer { ip, peer_id, port }),
                    _ => None,
                }
            })
            .collect()
    }

    pub fn peers(&self) -> Vec<String> {
        self.peers
            .iter()
            .map(|peer| peer.ip.clone() + ":" + peer.port.to_string().as_str())
            .collect()
    }
}
