use std::fmt;

#[derive(PartialEq, Clone, Debug)]
pub enum Error {
    MsgToLarge,
    UnknownId(u8),
    Incomplete(String),
    InvalidProtocolId,
    InvalidInfoHash,
    InvalidPeerId,
    InvalidLength(String),
    InvalidIndex(String),
    FileNotFound,
    PeerNotFound,
    PieceNotRequested,
    PieceNotLoaded,
    PieceOutOfRange,
    PieceBuffMissing,
    BlockNotRequested,
    KeepAliveTimeout,
    InfoMissing,
    CantReadFromSocket,
    ConnectionReset,
    ConnectionClosed,
    Decode(String),
    Tracker(String),
    MetaFileNotFound,
    MetaBEncodeMissing,
    MetaDataMissing,
    MetaLenAndFilesConflict,
    MetaLenOrFilesMissing,
    MetaInvalidUtf8(String),
    MetaIncorrectOrMissing(String),
    MetaInvalidU64(String),
    MetaNotDivisible(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::MsgToLarge => write!(f, "Message to large"),
            Error::UnknownId(msg_id) => write!(f, "Unknown Id({})", msg_id),
            Error::Incomplete(msg) => write!(f, "Incomplete {}", msg),
            Error::InvalidProtocolId => write!(f, "Invalid protocol Id"),
            Error::InvalidInfoHash => write!(f, "Invalid info hash"),
            Error::InvalidPeerId => write!(f, "Invalid peer ID"),
            Error::InvalidLength(msg) => write!(f, "Invalid length in {}", msg),
            Error::InvalidIndex(msg) => write!(f, "Invalid index in {}", msg),
            Error::FileNotFound => write!(f, "File not found"),
            Error::PeerNotFound => write!(f, "Peer not found"),
            Error::PieceNotRequested => write!(f, "Piece not requested"),
            Error::PieceNotLoaded => write!(f, "Piece not loaded"),
            Error::PieceOutOfRange => write!(f, "Piece out of range"),
            Error::PieceBuffMissing => write!(f, "Piece buff missing"),
            Error::BlockNotRequested => write!(f, "Block not requested"),
            Error::KeepAliveTimeout => write!(f, "Keep alive timeout"),
            Error::CantReadFromSocket => write!(f, "Can't read from socket"),
            Error::InfoMissing => write!(f, "Info field missing"),
            Error::ConnectionReset => write!(f, "Connection reset by peer"),
            Error::ConnectionClosed => write!(f, "Connection closed by peer"),
            Error::Decode(s) => write!(f, "{}", s),
            Error::Tracker(s) => write!(f, "{}", s),
            Error::MetaFileNotFound => write!(f, "Metainfo file not found"),
            Error::MetaBEncodeMissing => write!(f, "Metainfo bencode is missing"),
            Error::MetaDataMissing => write!(f, "Metainfo data is missing"),
            Error::MetaLenAndFilesConflict => write!(
                f,
                "Conflicting 'length' and 'files' values present. Only one is allowed"
            ),
            Error::MetaLenOrFilesMissing => write!(f, "Missing 'length' or 'files'"),
            Error::MetaInvalidUtf8(name) => write!(f, "Can't convert '{}' to UTF-8", name),
            Error::MetaIncorrectOrMissing(name) => {
                write!(f, "Incorrect or missing '{}' value", name)
            }
            Error::MetaInvalidU64(name) => write!(f, "Can't convert '{}' to u64", name),
            Error::MetaNotDivisible(name) => write!(f, "'{}' not divisible by 20", name),
        }
    }
}

impl std::error::Error for Error {}
