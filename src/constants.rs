/// SHA1 hash size
pub const HASH_SIZE: usize = 20;
/// Peer ID size
pub const PEER_ID_SIZE: usize = 20;
/// BEP3 suggest 16 kiB as default size for request
pub const PIECE_BLOCK_SIZE: usize = 16384;
/// Piece length, BEP3 suggest 256K as default
pub const PIECE_LENGTH: usize = 262144;
/// Default port
pub const PORT: u16 = 6881;

/// Maximal buffer size for frame
pub const MAX_FRAME_SIZE: usize = 65536;
pub const MSG_LEN_SIZE: usize = 4;
pub const MSG_ID_POS: usize = MSG_LEN_SIZE;
pub const MSG_ID_SIZE: usize = 1;

pub const MAX_NOT_INTERESTED: usize = 4;
pub const MAX_OPTIMISTIC_ROUNDS: usize = 3;
pub const MAX_OPTIMISTIC: usize = 1;
pub const MAX_UNCHOKED: usize = 10;
