// Copyright 2020 Mateusz Janda.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![warn(missing_docs)]

//! Rust is simple BitTorrent client, currently supporting
//! [BEP3](https://www.bittorrent.org/beps/bep_0003.html) specification.
//!
//! # Example
//! ```no_run
//! use rdest::{Metainfo, Session};
//! use std::path::Path;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let path = Path::new("ubuntu-20.04.4-desktop-amd64.iso.torrent");
//! let torrent_file = Metainfo::from_file(path).unwrap();
//! let peer_id = b"AAAAABBBBBCCCCCDDDDD";
//!
//! let mut session = Session::new(torrent_file, *peer_id);
//! session.run().await;
//! # }
//! ```

mod bcodec;
mod commands;
mod connection;
mod constants;
mod error;
mod extractor;
mod frame;
mod messages;
mod metainfo;
mod peer;
mod peer_handler;
pub mod peer_id;
mod progress_view;
mod serializer;
mod session;
mod tracker_client;
mod tracker_resp;
mod utils;

pub use crate::error::Error;

pub use crate::bcodec::bdecoder::BDecoder;
pub use crate::bcodec::bvalue::BValue;
pub use crate::bcodec::deep_finder::DeepFinder;
pub use crate::bcodec::raw_finder::RawFinder;

pub use crate::metainfo::File;
pub use crate::metainfo::Metainfo;

pub use crate::tracker_client::TrackerClient;
pub use crate::tracker_resp::TrackerResp;

pub use crate::session::Session;
