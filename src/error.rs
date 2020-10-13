use std::fmt;
use std::io;

#[derive(Debug)]
pub enum Error {
    Incomplete,
    Str(String),
    Io(io::Error),
    HttpReq(reqwest::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Incomplete => write!(f, "Data Incomplete"),
            Error::Str(s) => write!(f, "{}", s),
            Error::Io(i) => fmt::Display::fmt(i, f),
            Error::HttpReq(r) => fmt::Display::fmt(r, f),
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Error::HttpReq(error)
    }
}

impl From<String> for Error {
    fn from(error: String) -> Self {
        Error::Str(error)
    }
}
