use crate::constant::PEER_ID_SIZE;
use num_traits::AsPrimitive;
use rand::distributions::Alphanumeric;
use rand::Rng;

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
