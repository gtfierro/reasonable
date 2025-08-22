use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReasonableError {
    #[error("{0}")]
    ManagerError(String),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    IriParse(#[from] oxrdf::IriParseError),
    #[error(transparent)]
    BNodeParse(#[from] oxrdf::BlankNodeIdParseError),
    #[error(transparent)]
    Turtle(#[from] rio_turtle::TurtleError),
    #[error(transparent)]
    ChannelRecv(#[from] std::sync::mpsc::RecvError),
}

pub type Result<T> = std::result::Result<T, ReasonableError>;
