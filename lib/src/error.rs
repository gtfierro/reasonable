use oxrdf;

use std::fmt;
use std::io;
use std::sync::mpsc::RecvError;

#[derive(Debug)]
pub enum ReasonableError {
    ManagerError(String),
    IO(io::Error),
    IriParse(oxrdf::IriParseError),
    BNodeParse(oxrdf::BlankNodeIdParseError),
    ChannelRecv(RecvError),
}

impl fmt::Display for ReasonableError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ReasonableError::ManagerError(e) => write!(f, "{}", e),
            ReasonableError::IO(e) => write!(f, "{}", e.to_string()),
            ReasonableError::IriParse(e) => write!(f, "{}", e.to_string()),
            ReasonableError::BNodeParse(e) => write!(f, "{}", e.to_string()),
            ReasonableError::ChannelRecv(e) => write!(f, "{}", e.to_string()),
        }
    }
}

impl From<io::Error> for ReasonableError {
    fn from(err: io::Error) -> ReasonableError {
        ReasonableError::IO(err)
    }
}

impl From<oxrdf::IriParseError> for ReasonableError {
    fn from(err: oxrdf::IriParseError) -> ReasonableError {
        ReasonableError::IriParse(err)
    }
}

impl From<oxrdf::BlankNodeIdParseError> for ReasonableError {
    fn from(err: oxrdf::BlankNodeIdParseError) -> ReasonableError {
        ReasonableError::BNodeParse(err)
    }
}

impl From<RecvError> for ReasonableError {
    fn from(err: RecvError) -> ReasonableError {
        ReasonableError::ChannelRecv(err)
    }
}

pub type Result<T> = std::result::Result<T, ReasonableError>;
