use std::fmt;

#[derive(PartialEq, Clone, Debug)]
pub enum Error {
    Incomplete,
    InvalidProtocolId,
    InvalidInfoHash,
    InvalidSize,
    InvalidIndex,
    NotFound,
    HashCalculation,
    SocketWrite,
    ConnectionReset,
    MsgToLarge,
    UnknownId(u8),
    Decode(String),
    Meta(String),
    Tracker(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Incomplete => write!(f, "Incomplete"),
            Error::InvalidProtocolId => write!(f, "Invalid protocol Id"),
            Error::InvalidInfoHash => write!(f, "Invalid info hash"),
            Error::InvalidSize => write!(f, "Invalid size"),
            Error::InvalidIndex => write!(f, "Invalid index"),
            Error::NotFound => write!(f, "Not found"),
            Error::HashCalculation => write!(f, "Can't calculate SHA1 hash"),
            Error::SocketWrite => write!(f, "Can't write to socket"),
            Error::MsgToLarge => write!(f, "Message to large"),
            Error::ConnectionReset => write!(f, "Connection reset by peer"),
            Error::UnknownId(msg_id) => write!(f, "Unknown Id({})", msg_id),
            Error::Decode(s) => write!(f, "{}", s),
            Error::Meta(s) => write!(f, "{}", s),
            Error::Tracker(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for Error {}
