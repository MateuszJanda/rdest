use std::fmt;
use std::io;

#[derive(Debug)]
pub enum Error {
    Incomplete,
    S(String),
    I(io::Error),
    R(reqwest::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Incomplete => write!(f, "Data Incomplete"),
            Error::S(s) => write!(f, "{}", s),
            Error::I(i) => fmt::Display::fmt(i, f),
            Error::R(r) => fmt::Display::fmt(r, f),
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::I(error)
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Error::R(error)
    }
}

impl From<String> for Error {
    fn from(error: String) -> Self {
        Error::S(error)
    }
}
