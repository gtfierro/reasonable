use oxigraph::sparql;

use std::io;
use std::sync::mpsc::RecvError;

#[derive(Debug)]
pub enum ReasonableError {
    ManagerError(String),
    IO(io::Error),
    Parse(sparql::ParseError),
    Eval(sparql::EvaluationError),
    SQLite(rusqlite::Error),
    ChannelRecv(RecvError),
}


impl From<io::Error> for ReasonableError {
    fn from(err: io::Error) -> ReasonableError {
        ReasonableError::IO(err)
    }
}

impl From<sparql::ParseError> for ReasonableError {
    fn from(err: sparql::ParseError) -> ReasonableError {
        ReasonableError::Parse(err)
    }
}

impl From<sparql::EvaluationError> for ReasonableError {
    fn from(err: sparql::EvaluationError) -> ReasonableError {
        ReasonableError::Eval(err)
    }
}

impl From<rusqlite::Error> for ReasonableError {
    fn from(err: rusqlite::Error) -> ReasonableError {
        ReasonableError::SQLite(err)
    }
}

impl From<RecvError> for ReasonableError {
    fn from(err: RecvError) -> ReasonableError {
        ReasonableError::ChannelRecv(err)
    }
}

pub type Result<T> = std::result::Result<T, ReasonableError>;
