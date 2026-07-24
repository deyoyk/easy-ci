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

impl From<dialoguer::Error> for EciError {
    fn from(e: dialoguer::Error) -> Self {
        match e {
            dialoguer::Error::IO(io_err) => EciError::Io(io_err),
        }
    }
}

pub type Result<T> = std::result::Result<T, EciError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_config() {
        let err = EciError::Config("test config error".to_string());
        assert_eq!(err.to_string(), "Config error: test config error");
    }

    #[test]
    fn error_display_github() {
        let err = EciError::GitHub("not found".to_string());
        assert_eq!(err.to_string(), "GitHub error: not found");
    }

    #[test]
    fn error_display_docker() {
        let err = EciError::Docker("daemon not running".to_string());
        assert_eq!(err.to_string(), "Docker error: daemon not running");
    }

    #[test]
    fn error_display_database() {
        let err = EciError::Database("unsupported type".to_string());
        assert_eq!(err.to_string(), "Database error: unsupported type");
    }

    #[test]
    fn error_display_deploy() {
        let err = EciError::Deploy("build failed".to_string());
        assert_eq!(err.to_string(), "Deploy error: build failed");
    }

    #[test]
    fn error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = EciError::from(io_err);
        assert!(matches!(err, EciError::Io(_)));
    }

    #[test]
    fn error_from_sqlite() {
        let sql_err = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ffi::ErrorCode::DatabaseBusy,
                extended_code: 5,
            },
            Some("database is locked".to_string()),
        );
        let err = EciError::from(sql_err);
        assert!(matches!(err, EciError::Sqlite(_)));
    }

    #[test]
    fn error_debug_format() {
        let err = EciError::Config("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("test"));
    }
}
