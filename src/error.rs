use std::fmt;

#[derive(PartialEq, Clone, Debug)]
pub enum Error {
    Incomplete,
    InvalidHeader,
    UnknownId,
    Decode(String),
    Meta(String),
    Tracker(String),
    Peer(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Incomplete => write!(f, "Incomplete"),
            Error::InvalidHeader => write!(f, "InvalidHeader"),
            Error::UnknownId => write!(f, "UnknownId"),
            Error::Decode(s) => write!(f, "{}", s),
            Error::Meta(s) => write!(f, "{}", s),
            Error::Tracker(s) => write!(f, "{}", s),
            Error::Peer(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for Error {
}

