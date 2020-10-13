use crate::{BValue, Error};
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
            Err(_) => Err(Error::Str(format!("File not found"))),
        }
    }

    pub fn from_bencode(data: &[u8]) -> Result<TrackerResp, Error> {
        let bvalues = BValue::parse(data)?;
        // let raw_info = BValue::cut_raw_info(arg)?;

        if bvalues.is_empty() {
            return Err(Error::Str(format!("Empty torrent")));
        }

        let mut err = Err(Error::Str(format!("Missing data")));
        for val in bvalues {
            match val {
                BValue::Dict(dict) => match Self::create_response(&dict) {
                    Ok(torrent) => return Ok(torrent),
                    Err(e) => err = Err(e),
                },
                _ => (),
            }
        }

        err
    }

    fn create_response(dict: &HashMap<Vec<u8>, BValue>) -> Result<TrackerResp, Error> {
        if let Some(reason) = Self::find_failure_reason(dict) {
            return Err(Error::from(reason));
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
                u64::try_from(*interval).or(Err(Error::Str(format!("Can't convert 'interval' to u64"))))
            }
            _ => Err(Error::Str(format!("Incorrect or missing 'interval' value"))),
        }
    }

    fn find_peers(dict: &HashMap<Vec<u8>, BValue>) -> Result<Vec<Peer>, Error> {
        match dict.get(&b"peers".to_vec()) {
            Some(BValue::List(peers)) => Ok(Self::peer_list(peers)),
            _ => Err(Error::Str(format!("Incorrect or missing 'peers' value"))),
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
}
