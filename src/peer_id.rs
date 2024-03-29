// Copyright 2020 Mateusz Janda.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Peer ID utilities.

use crate::constants::PEER_ID_SIZE;
use num_traits::AsPrimitive;
use rand::distributions::Alphanumeric;
use rand::Rng;

/// Generate random peer ID.
///
/// # Example
/// ```
/// use rdest::peer_id;
///
/// let id = peer_id::generate();
/// println!("{:?}", id);
/// ```
pub fn generate() -> [u8; PEER_ID_SIZE] {
    let mut peer_id: [u8; PEER_ID_SIZE] = [0; PEER_ID_SIZE];
    for (idx, ch) in rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(PEER_ID_SIZE)
        .enumerate()
    {
        peer_id[idx] = ch.as_();
    }

    return peer_id;
}
