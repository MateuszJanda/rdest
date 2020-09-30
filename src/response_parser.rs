use std::fs;
use crate::BValue;
use std::collections::HashMap;
use std::convert::TryFrom;

#[derive(PartialEq, Clone, Debug)]
pub struct ResponseParser {
    interval : u64,
    peers: Vec<Peer>
}

#[derive(PartialEq, Clone, Debug)]
pub struct Peer {
    ip : String,
    peer_id : String,
    port : u64
}

impl ResponseParser {
    pub fn from_file(path: String) -> Result<ResponseParser, String> {
        match &fs::read(path) {
            Ok(val) => Self::from_bencode(val),
            Err(_) => Err(format!("File not found")),
        }
    }

    pub fn from_bencode(data: &[u8]) -> Result<ResponseParser, String> {
        let bvalues = BValue::parse(data)?;
        // let raw_info = BValue::cut_raw_info(arg)?;

        if bvalues.is_empty() {
            return Err(format!("Empty torrent"));
        }

        let mut err = Err(format!("Missing data"));
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

    fn create_response(dict: &HashMap<Vec<u8>, BValue>) -> Result<ResponseParser, String> {
        if let Some(reason) = Self::find_failure_reason(dict) {
            return Err(reason)
        }

        let response = ResponseParser{
            interval : Self::find_interval(dict)?,
            peers: Self::find_peers(dict)?,
        };

        Ok(response)
    }

    fn find_failure_reason(dict: &HashMap<Vec<u8>, BValue>) -> Option<String> {
        match dict.get(&b"failure reason".to_vec()) {
            Some(BValue::ByteStr(reason)) => String::from_utf8(reason.to_vec()).ok(),
            _ => None
        }
    }

    fn find_interval(dict: &HashMap<Vec<u8>, BValue>) -> Result<u64, String> {
        match dict.get(&b"interval".to_vec()) {
            Some(BValue::Int(interval)) => u64::try_from(*interval).or(Err(format!("Can't convert 'interval' to u64"))),
            _ => Err(format!("Incorrect or missing 'interval' value"))
        }
    }

    fn find_peers(dict: &HashMap<Vec<u8>, BValue>) -> Result<Vec<Peer>, String> {
        match dict.get(&b"peers".to_vec()) {
            Some(BValue::List(peers)) => Ok(Self::peer_list(peers)),
            _ => Err(format!("Incorrect or missing 'peers' value"))
        }
    }

    fn peer_list(list: &Vec<BValue>) -> Vec<Peer> {
        list.iter()
            .filter_map(|elem| match elem {
                BValue::Dict(dict) => Some(dict),
                _ => None,
            })
            .filter_map(
                |dict| match (dict.get(&b"ip".to_vec()), dict.get(&b"peer id".to_vec()), dict.get(&b"port".to_vec())) {
                    (Some(BValue::ByteStr(ip)), Some(BValue::ByteStr(peer_id)), Some(BValue::Int(port))) => {
                        Some((ip, peer_id, port))
                    }
                    _ => None,
                },
            )
            .filter_map(|(ip, peer_id, port)| {
                match (String::from_utf8(ip.to_vec()), String::from_utf8(peer_id.to_vec()), u64::try_from(*port)) {
                    (Ok(ip), Ok(peer_id), Ok(port)) => Some(Peer { ip, peer_id, port }),
                    _ => None,
                }
            })
            .collect()
    }
}
