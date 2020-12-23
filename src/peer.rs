use crate::commands::{
    BitfieldCmd, HaveCmd, InitCmd, NotInterestedCmd, PieceDoneCmd, ReqData, RequestCmd, UnchokeCmd,
};
use crate::constants::{MAX_UNCHOKED, PEER_ID_SIZE};
use crate::messages::bitfield::Bitfield;
use crate::session::Status;
use crate::Metainfo;
use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct Peer {
    pub id: Option<[u8; PEER_ID_SIZE]>,
    pub pieces: Vec<bool>,
    pub job: Option<JoinHandle<()>>,
    pub piece_index: Option<usize>,
    pub am_interested: bool,
    pub am_choked: bool,
    pub interested: bool,
    pub choked: bool,
    pub optimistic_unchoke: bool,
    pub download_rate: Option<u32>,
    pub uploaded_rate: Option<u32>,
}

impl Peer {
    pub fn new(id: Option<[u8; PEER_ID_SIZE]>, pieces_num: usize, job: JoinHandle<()>) -> Peer {
        Peer {
            id,
            pieces: vec![false; pieces_num],
            job: Some(job),
            piece_index: None,
            am_interested: false,
            am_choked: true,
            interested: false,
            choked: true,
            optimistic_unchoke: false,
            download_rate: None,
            uploaded_rate: None,
        }
    }

    pub fn update_pieces(&mut self, pieces: &Vec<bool>) {
        self.pieces.copy_from_slice(pieces);
    }

    pub fn handle_init(
        &mut self,
        peer_id: [u8; PEER_ID_SIZE],
        pieces_status: &Vec<Status>,
    ) -> InitCmd {
        self.id = Some(peer_id);

        let bitfield = Bitfield::from_vec(
            &pieces_status
                .iter()
                .map(|status| *status == Status::Have)
                .collect(),
        );

        InitCmd::SendBitfield { bitfield }
    }

    pub fn handle_choke(&mut self, pieces_status: &mut Vec<Status>) {
        self.choked = true;

        match self.piece_index {
            Some(piece_index) if pieces_status[piece_index] == Status::Reserved => {
                pieces_status[piece_index] = Status::Missing
            }
            _ => (),
        }
    }

    pub fn handle_unchoke(
        &mut self,
        chosen_index: Option<usize>,
        pieces_status: &mut Vec<Status>,
        metainfo: &Metainfo,
    ) -> UnchokeCmd {
        let cmd = match chosen_index {
            Some(chosen_index) => {
                pieces_status[chosen_index] = Status::Reserved;
                match self.am_interested {
                    true => UnchokeCmd::SendRequest(req_data(metainfo, chosen_index)),
                    false => UnchokeCmd::SendInterestedAndRequest(req_data(metainfo, chosen_index)),
                }
            }
            None => match self.am_interested {
                true => UnchokeCmd::SendNotInterested,
                false => UnchokeCmd::Ignore,
            },
        };

        self.choked = false;
        self.piece_index = chosen_index;
        self.am_interested = chosen_index.is_some();

        cmd
    }

    pub fn handle_interested(&mut self) {
        self.interested = true;
    }

    pub fn handle_not_interested(&mut self, chosen_index: Option<usize>) -> NotInterestedCmd {
        self.interested = false;

        match !self.am_interested && self.piece_index.is_none() && chosen_index.is_none() {
            true => NotInterestedCmd::PrepareKill,
            false => NotInterestedCmd::Ignore,
        }
    }

    pub fn handle_have(
        &mut self,
        piece_index: usize,
        pieces_status: &mut Vec<Status>,
        metainfo: &Metainfo,
    ) -> HaveCmd {
        self.pieces[piece_index] = true;

        if pieces_status[piece_index] == Status::Missing && !self.am_interested {
            if !self.choked && self.piece_index.is_none() {
                pieces_status[piece_index] = Status::Reserved;
                self.piece_index = Some(piece_index);
                self.am_interested = true;
                HaveCmd::SendInterestedAndRequest(req_data(metainfo, piece_index))
            } else {
                self.am_interested = true;
                HaveCmd::SendInterested
            }
        } else {
            HaveCmd::Ignore
        }
    }

    pub fn handle_bitfield(
        &mut self,
        chosen_index: Option<usize>,
        unchoked_num: usize,
    ) -> BitfieldCmd {
        // BEP3 says "whenever a downloader doesn't have something they currently would ask a peer
        // for in unchoked, they must express lack of interest, despite being choked"
        //
        // Sending NotInterested explicitly (this is default state) is mandatory according BEP3, but
        // Interested should be send only after Unchoke. It appears (unfortunately) that many
        // clients wait for this message (doesn't send Unchoke and send KeepAlive instead).
        let am_interested = match chosen_index {
            Some(_) => true,
            None => false,
        };

        // Change to unchoked or not
        let with_am_unchoked = unchoked_num < MAX_UNCHOKED && self.am_choked;

        // Update own state
        self.am_interested = am_interested;
        if with_am_unchoked {
            self.am_choked = false;
        }

        BitfieldCmd::SendState {
            with_am_unchoked,
            am_interested,
        }
    }

    pub fn handle_request(
        &mut self,
        piece_index: usize,
        pieces_status: &Vec<Status>,
        metainfo: &Metainfo,
    ) -> RequestCmd {
        if self.am_choked {
            return RequestCmd::Ignore;
        }

        if piece_index >= metainfo.pieces_num() {
            return RequestCmd::Ignore;
        }

        if pieces_status[piece_index] != Status::Have {
            return RequestCmd::Ignore;
        }

        RequestCmd::LoadAndSendPiece {
            piece_index,
            piece_hash: *metainfo.piece(piece_index),
        }
    }

    pub fn handle_piece_done(
        &mut self,
        chosen_index: Option<usize>,
        pieces_status: &mut Vec<Status>,
        metainfo: &Metainfo,
    ) -> PieceDoneCmd {
        match chosen_index {
            Some(chosen_index) => {
                pieces_status[chosen_index] = Status::Reserved;
                self.piece_index = Some(chosen_index);
                match self.choked {
                    true => PieceDoneCmd::Ignore,
                    false => PieceDoneCmd::SendRequest(req_data(&metainfo, chosen_index)),
                }
            }
            None => {
                self.piece_index = None;
                self.am_interested = false;
                match self.interested {
                    true => PieceDoneCmd::SendNotInterested,
                    false => PieceDoneCmd::PrepareKill,
                }
            }
        }
    }

    pub fn handle_sync_stats(
        &mut self,
        downloaded_rate: &Option<u32>,
        uploaded_rate: &Option<u32>,
    ) {
        self.download_rate = *downloaded_rate;
        self.uploaded_rate = *uploaded_rate;
    }

    pub fn status_abbreviation(&self) -> String {
        let own_state = if self.optimistic_unchoke {
            "o"
        } else if self.am_choked {
            "c"
        } else {
            "u"
        };

        let own_state = match self.am_interested {
            true => own_state.to_uppercase(),
            false => own_state.to_string(),
        };

        let peer_state = match self.choked {
            true => "c",
            false => "u",
        };

        let peer_state = match self.interested {
            true => peer_state.to_uppercase(),
            false => peer_state.to_string(),
        };

        own_state + &peer_state
    }
}

fn req_data(metainfo: &Metainfo, piece_index: usize) -> ReqData {
    ReqData {
        piece_index,
        piece_length: metainfo.piece_length(piece_index),
        piece_hash: *metainfo.piece(piece_index),
    }
}
