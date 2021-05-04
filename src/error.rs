use oxigraph::sparql;
use oxigraph::model;

use std::io;
use std::sync::mpsc::RecvError;

#[cfg(feature = "sqlite")]
#[derive(Debug)]
pub enum ReasonableError {
    ManagerError(String),
    IO(io::Error),
    SPARQLParse(sparql::ParseError),
    IriParse(model::IriParseError),
    BNodeParse(model::BlankNodeIdParseError),
    Eval(sparql::EvaluationError),
    SQLite(rusqlite::Error),
    ChannelRecv(RecvError),
}

#[cfg(not(feature = "sqlite"))]
#[derive(Debug)]
pub enum ReasonableError {
    ManagerError(String),
    IO(io::Error),
    SPARQLParse(sparql::ParseError),
    IriParse(model::IriParseError),
    BNodeParse(model::BlankNodeIdParseError),
    Eval(sparql::EvaluationError),
    ChannelRecv(RecvError),
}

impl From<io::Error> for ReasonableError {
    fn from(err: io::Error) -> ReasonableError {
        ReasonableError::IO(err)
    }
}

impl From<sparql::ParseError> for ReasonableError {
    fn from(err: sparql::ParseError) -> ReasonableError {
        ReasonableError::SPARQLParse(err)
    }
}

impl From<sparql::EvaluationError> for ReasonableError {
    fn from(err: sparql::EvaluationError) -> ReasonableError {
        ReasonableError::Eval(err)
    }
}

impl From<model::IriParseError> for ReasonableError {
    fn from(err: model::IriParseError) -> ReasonableError {
        ReasonableError::IriParse(err)
    }
}

impl From<model::BlankNodeIdParseError> for ReasonableError {
    fn from(err: model::BlankNodeIdParseError) -> ReasonableError {
        ReasonableError::BNodeParse(err)
    }
}

#[cfg(feature = "sqlite")]
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
