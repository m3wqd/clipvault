#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Database operation failed: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Invalid(String),
}
pub type Result<T> = std::result::Result<T, Error>;
