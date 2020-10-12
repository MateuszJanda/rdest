use std::fmt;

#[derive(Debug)]
pub enum Error {
    Incomplete,
    S(String),
    I(std::io::Error),
    R(reqwest::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Incomplete => write!(f, "dddd"),
            Error::S(s) => write!(f, "ssss"),
            Error::I(i) => write!(f, "iii"),
            Error::R(r) => write!(f, "rrr"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::I(error)
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Error::R(error)
    }
}

