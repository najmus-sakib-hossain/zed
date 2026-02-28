use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
