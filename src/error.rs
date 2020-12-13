use std::fmt;

/// Rdest lin errors
#[derive(PartialEq, Clone, Debug)]
pub enum Error {
    /// To avoid DDoS and exhaustion of RAM by peer, maximal size of message is limited to 65536 bytes.
    MsgToLarge,
    /// Not supported ID message (probably from not supported standard extension)
    UnknownId(u8),
    /// Incomplete message (details as argument e.g: message name)
    Incomplete(String),
    /// Invalid protocol ID in Handshake message. Check [BEP3](https://www.bittorrent.org/beps/bep_0003.html#peer%20protocol)
    InvalidProtocolId,
    /// Peer return invalid info hash in Handshake message.
    InvalidInfoHash,
    InvalidLength(String),
    InvalidIndex(String),
    FileNotFound,
    PeerNotFound,
    PieceNotRequested,
    PieceNotLoaded,
    PieceOutOfRange,
    PieceBuffMissing,
    PieceHashMismatch,
    BlockNotRequested,
    KeepAliveTimeout,
    InfoMissing,
    CantReadFromSocket,
    ConnectionReset,
    ConnectionClosed,
    DecodeUnexpectedChar(&'static str, u32, usize),
    DecodeIncorrectChar(&'static str, u32, usize),
    DecodeUnableConvert(&'static str, u32, &'static str, usize),
    DecodeNotEnoughChars(&'static str, u32, usize),
    DecodeMissingTerminalChars(&'static str, u32, usize),
    DecodeLeadingZero(&'static str, u32, usize),
    DecodeOddNumOfElements(&'static str, u32, usize),
    DecodeKeyNotString(&'static str, u32, usize),
    TrackerFileNotFound,
    TrackerBEncodeMissing,
    TrackerDataMissing,
    TrackerIncorrectOrMissing(String),
    TrackerRespFail(String),
    TrackerInvalidU64(String),
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
            Error::InvalidLength(msg) => write!(f, "Invalid length in {}", msg),
            Error::InvalidIndex(msg) => write!(f, "Invalid index in {}", msg),
            Error::FileNotFound => write!(f, "File not found"),
            Error::PeerNotFound => write!(f, "Peer not found"),
            Error::PieceNotRequested => write!(f, "Piece not requested"),
            Error::PieceNotLoaded => write!(f, "Piece not loaded"),
            Error::PieceOutOfRange => write!(f, "Piece out of range"),
            Error::PieceBuffMissing => write!(f, "Piece buff missing"),
            Error::PieceHashMismatch => write!(f, "Piece hash mismatch"),
            Error::BlockNotRequested => write!(f, "Block not requested"),
            Error::KeepAliveTimeout => write!(f, "Keep alive timeout"),
            Error::CantReadFromSocket => write!(f, "Can't read from socket"),
            Error::InfoMissing => write!(f, "Info field missing"),
            Error::ConnectionReset => write!(f, "Connection reset by peer"),
            Error::ConnectionClosed => write!(f, "Connection closed by peer"),
            Error::DecodeUnexpectedChar(file, line, pos) => {
                write!(f, "{}:{}, unexpected end character at {}", file, line, pos)
            }
            Error::DecodeIncorrectChar(file, line, pos) => {
                write!(f, "{}:{}, incorrect character at {}", file, line, pos)
            }
            Error::DecodeUnableConvert(file, line, name, pos) => write!(
                f,
                "{}:{}, unable convert to {} at {}",
                file, line, name, pos
            ),
            Error::DecodeNotEnoughChars(file, line, pos) => {
                write!(f, "{}:{}, not enough characters at {}", file, line, pos)
            }
            Error::DecodeMissingTerminalChars(file, line, pos) => write!(
                f,
                "{}:{}, missing terminate character at {}",
                file, line, pos
            ),
            Error::DecodeLeadingZero(file, line, pos) => {
                write!(f, "{}:{}, leading zero at {}", file, line, pos)
            }
            Error::DecodeOddNumOfElements(file, line, pos) => {
                write!(f, "{}:{}, odd number of elements at {}", file, line, pos)
            }
            Error::DecodeKeyNotString(file, line, pos) => {
                write!(f, "{}:{}, key is not string at {}", file, line, pos)
            }
            Error::TrackerFileNotFound => write!(f, "Tracker, file not found"),
            Error::TrackerBEncodeMissing => write!(f, "Tracker, bencode is missing"),
            Error::TrackerDataMissing => write!(f, "Tracker, data is missing"),
            Error::TrackerIncorrectOrMissing(name) => {
                write!(f, "Tracker, incorrect or missing '{}' value", name)
            }
            Error::TrackerRespFail(reason) => write!(f, "Tracker fail: {}", reason),
            Error::TrackerInvalidU64(name) => write!(f, "Tracker, can't convert '{}' to u64", name),
            Error::MetaFileNotFound => write!(f, "Metainfo, file not found"),
            Error::MetaBEncodeMissing => write!(f, "Metainfo, bencode is missing"),
            Error::MetaDataMissing => write!(f, "Metainfo, data is missing"),
            Error::MetaLenAndFilesConflict => write!(
                f,
                "Metainfo, conflicting 'length' and 'files' values present. Only one is allowed"
            ),
            Error::MetaLenOrFilesMissing => write!(f, "Metainfo, missing 'length' or 'files'"),
            Error::MetaInvalidUtf8(name) => {
                write!(f, "Metainfo, Can't convert '{}' to UTF-8", name)
            }
            Error::MetaIncorrectOrMissing(name) => {
                write!(f, "Metainfo, incorrect or missing '{}' value", name)
            }
            Error::MetaInvalidU64(name) => write!(f, "Metainfo, can't convert '{}' to u64", name),
            Error::MetaNotDivisible(name) => write!(f, "Metainfo, '{}' not divisible by 20", name),
        }
    }
}

impl std::error::Error for Error {}
