use oxigraph::sparql;
use std::io;

#[derive(Debug)]
pub enum ReasonableError {
    ManagerError(String),
    IO(io::Error),
    Parse(sparql::ParseError),
    Eval(sparql::EvaluationError),
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

pub type Result<T> = std::result::Result<T, ReasonableError>;
