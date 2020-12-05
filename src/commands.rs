use crate::constant::HASH_SIZE;
use crate::messages::bitfield::Bitfield;
use crate::TrackerResp;
use std::collections::HashMap;
use tokio::sync::oneshot;

#[derive(Debug, Clone)]
pub enum TrackerCmd {
    TrackerResp(TrackerResp),
    Fail(String),
}

#[derive(Debug, Clone)]
pub enum ExtractorCmd {
    Done,
    Fail(String),
}

pub enum ViewCmd {
    Log(String),
    Kill,
}

#[derive(Debug, Clone)]
pub enum BroadCmd {
    SendHave {
        index: usize,
    },
    SendOwnState {
        am_choked_map: HashMap<String, bool>,
    },
}
#[derive(Debug)]
pub enum JobCmd {
    Init {
        addr: String,
        resp_ch: oneshot::Sender<InitCmd>,
    },
    RecvChoke {
        addr: String,
    },
    RecvUnchoke {
        addr: String,
        resp_ch: oneshot::Sender<UnchokeCmd>,
    },
    RecvInterested {
        addr: String,
    },
    RecvNotInterested {
        addr: String,
        resp_ch: oneshot::Sender<NotInterestedCmd>,
    },
    RecvHave {
        addr: String,
        index: usize,
        resp_ch: oneshot::Sender<HaveCmd>,
    },
    RecvBitfield {
        addr: String,
        bitfield: Bitfield,
        resp_ch: oneshot::Sender<BitfieldCmd>,
    },
    RecvRequest {
        addr: String,
        index: usize,
        block_begin: usize,
        block_length: usize,
        resp_ch: oneshot::Sender<RequestCmd>,
    },
    PieceDone {
        addr: String,
        resp_ch: oneshot::Sender<PieceDoneCmd>,
    },
    SyncStats {
        addr: String,
        downloaded_rate: Option<u32>,
        uploaded_rate: Option<u32>,
        rejected_piece: u32,
    },
    KillReq {
        addr: String,
        index: Option<usize>,
        reason: String,
    },
}

#[derive(Debug)]
pub enum InitCmd {
    SendBitfield { bitfield: Bitfield },
}

#[derive(Debug)]
pub enum UnchokeCmd {
    SendInterestedAndRequest {
        index: usize,
        piece_length: usize,
        piece_hash: [u8; HASH_SIZE],
    },
    SendRequest {
        index: usize,
        piece_length: usize,
        piece_hash: [u8; HASH_SIZE],
    },
    SendNotInterested,
    Ignore,
}

#[derive(Debug)]
pub enum NotInterestedCmd {
    PrepareKill,
    Ignore,
}

#[derive(Debug)]
pub enum HaveCmd {
    SendInterestedAndRequest {
        index: usize,
        piece_length: usize,
        piece_hash: [u8; HASH_SIZE],
    },
    SendInterested,
    Ignore,
}

#[derive(Debug)]
pub enum BitfieldCmd {
    SendState {
        with_am_unchoked: bool,
        am_interested: bool,
    },
}

#[derive(Debug)]
pub enum RequestCmd {
    LoadAndSendPiece {
        index: usize,
        block_begin: usize,
        block_length: usize,
        piece_hash: [u8; HASH_SIZE],
    },
    Ignore,
}

#[derive(Debug)]
pub enum PieceDoneCmd {
    SendRequest {
        index: usize,
        piece_length: usize,
        piece_hash: [u8; HASH_SIZE],
    },
    SendNotInterested,
    PrepareKill,
    Ignore,
}
