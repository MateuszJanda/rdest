use std::fmt;

#[derive(PartialEq, Clone, Debug)]
pub enum Error {
    Incomplete,
    Invalid,
    HashCalculation,
    SocketWrite,
    ConnectionReset,
    UnknownId(u8),
    Decode(String),
    Meta(String),
    Tracker(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Incomplete => write!(f, "Incomplete"),
            Error::Invalid => write!(f, "Invalid"),
            Error::HashCalculation => write!(f, "HashCalculation"),
            Error::SocketWrite => write!(f, "SocketWrite"),
            Error::ConnectionReset => write!(f, "Connection reset by peer"),
            Error::UnknownId(msg_id) => write!(f, "UnknownId({})", msg_id),
            Error::Decode(s) => write!(f, "{}", s),
            Error::Meta(s) => write!(f, "{}", s),
            Error::Tracker(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for Error {}
