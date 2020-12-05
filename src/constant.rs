// SHA1 hash size
pub const HASH_SIZE: usize = 20;
pub const PEER_ID_SIZE: usize = 20;
// BEP3 suggest 16 kiB as default size for request
pub const PIECE_BLOCK_SIZE: usize = 16384;
pub const PORT: u16 = 6881;

pub const MSG_LEN_SIZE: usize = 4;
pub const MSG_ID_POS: usize = MSG_LEN_SIZE;
pub const MSG_ID_SIZE: usize = 1;
