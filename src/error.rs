use std::fmt;

/// rdest lib errors
#[derive(PartialEq, Clone, Debug)]
pub enum Error {
    /// To avoid DDoS and exhaustion of RAM by peer, maximal size of message is limited to MAX_FRAME_SIZE bytes.
    MsgToLarge,
    /// Not supported ID message (probably from not supported standard extension).
    UnknownId(u8),
    /// Incomplete message (details as argument e.g: message name).
    Incomplete(String),
    /// Invalid protocol ID in Handshake message. Check [BEP3](https://www.bittorrent.org/beps/bep_0003.html#peer%20protocol).
    InvalidProtocolId,
    /// Peer return invalid info hash in Handshake message.
    InvalidInfoHash,
    /// Invalid length of Piece or Request message (different than requested peer).
    InvalidLength(String),
    /// Invalid index (different than requested by peer).
    InvalidIndex(String),
    /// Can't load piece file.
    FileNotFound,
    /// Can't write to file.
    FileCannotWrite,
    /// Peer not found in manager.
    PeerNotFound,
    /// Piece not requested by client.
    PieceNotRequested,
    /// Piece not loaded by handler.
    PieceNotLoaded,
    /// Missing piece buffer on requested message.
    PieceBuffMissing,
    /// Piece hash mismatch.
    PieceHashMismatch,
    /// Peer send not requested block.
    BlockNotRequested,
    /// Peer doesn't send any message, keep-alive trigger.
    KeepAliveTimeout,
    /// Missing info field to calculate hash.
    InfoMissing,
    /// Can't read from socket.
    CantReadFromSocket,
    /// Connection reset.
    ConnectionReset,
    /// Connection closed.
    ConnectionClosed,
    /// Decoder encountered unexpected char.
    DecodeUnexpectedChar(&'static str, usize),
    /// Decoder encountered incorrect char.
    DecodeIncorrectChar(&'static str, usize),
    /// Decoder was unable to convert to `BValue`.
    DecodeUnableConvert(&'static str, &'static str, usize),
    /// Not enough chars to decode.
    DecodeNotEnoughChars(&'static str, usize),
    /// Decoder encountered missing terminal character "e".
    DecodeMissingTerminalChars(&'static str, usize),
    /// Incorrect integer with leading zero.
    DecodeLeadingZero(&'static str, usize),
    /// Odd number of elements in dictionary.
    DecodeOddNumOfElements(&'static str, usize),
    /// Key not string in dictionary
    DecodeKeyNotString(&'static str, usize),
    /// Missing [bencoded](https://en.wikipedia.org/wiki/Bencode) data to decode tracker response.
    TrackerBEncodeMissing,
    /// Not enough data in tracker response.
    TrackerDataMissing,
    /// Incorrect or missing fields in tracker response.
    TrackerIncorrectOrMissing(String),
    /// Tracker replay with error.
    TrackerRespFail(String),
    /// Missing metainfo file.
    MetaFileNotFound,
    /// Missing [bencoded](https://en.wikipedia.org/wiki/Bencode) data in metainfo.
    MetaBEncodeMissing,
    /// Missing data in metainfo.
    MetaDataMissing,
    /// Mutually exclusive length and files in metafile.
    MetaLenAndFilesConflict,
    /// Missing length or files in metafile.
    MetaLenOrFilesMissing,
    /// Can't convert metainfo fields to UTF-8.
    MetaInvalidUtf8(String),
    /// Missing field in metainfo.
    MetaIncorrectOrMissing(String),
    /// Can't convert metainfo field to u64
    MetaInvalidU64(String),
    /// Not enough data to extract SHA-1 hashes.
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
            Error::FileCannotWrite => write!(f, "Can't write to file"),
            Error::PeerNotFound => write!(f, "Peer not found"),
            Error::PieceNotRequested => write!(f, "Piece not requested"),
            Error::PieceNotLoaded => write!(f, "Piece not loaded"),
            Error::PieceBuffMissing => write!(f, "Piece buff missing"),
            Error::PieceHashMismatch => write!(f, "Piece hash mismatch"),
            Error::BlockNotRequested => write!(f, "Block not requested"),
            Error::KeepAliveTimeout => write!(f, "Keep alive timeout"),
            Error::CantReadFromSocket => write!(f, "Can't read from socket"),
            Error::InfoMissing => write!(f, "Info field missing"),
            Error::ConnectionReset => write!(f, "Connection reset by peer"),
            Error::ConnectionClosed => write!(f, "Connection closed by peer"),
            Error::DecodeUnexpectedChar(fun, pos) => {
                write!(f, "{}: unexpected end character at {}", fun, pos)
            }
            Error::DecodeIncorrectChar(fun, pos) => {
                write!(f, "{}: incorrect character at {}", fun, pos)
            }
            Error::DecodeUnableConvert(fun, name, pos) => {
                write!(f, "{}: unable convert to {} at {}", fun, name, pos)
            }
            Error::DecodeNotEnoughChars(fun, pos) => {
                write!(f, "{}: not enough characters at {}", fun, pos)
            }
            Error::DecodeMissingTerminalChars(fun, pos) => {
                write!(f, "{}: missing terminate character at {}", fun, pos)
            }
            Error::DecodeLeadingZero(fun, pos) => write!(f, "{}: leading zero at {}", fun, pos),
            Error::DecodeOddNumOfElements(fun, pos) => {
                write!(f, "{}: odd number of elements at {}", fun, pos)
            }
            Error::DecodeKeyNotString(fun, pos) => {
                write!(f, "{}: key is not string at {}", fun, pos)
            }
            Error::TrackerBEncodeMissing => write!(f, "Tracker, bencode is missing"),
            Error::TrackerDataMissing => write!(f, "Tracker, data is missing"),
            Error::TrackerIncorrectOrMissing(name) => {
                write!(f, "Tracker, incorrect or missing '{}' value", name)
            }
            Error::TrackerRespFail(reason) => write!(f, "Tracker fail: {}", reason),
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
