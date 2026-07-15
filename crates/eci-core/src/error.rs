use thiserror::Error;

#[derive(Error, Debug)]
pub enum EciError {
    #[error("Config error: {0}")]
    Config(String),
    #[error("GitHub error: {0}")]
    GitHub(String),
    #[error("Docker error: {0}")]
    Docker(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Deploy error: {0}")]
    Deploy(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

pub type Result<T> = std::result::Result<T, EciError>;
