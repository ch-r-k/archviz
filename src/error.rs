use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArchError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("syn parse error: {0}")]
    Syn(#[from] syn::Error),

    #[error("invalid module filter pattern: {0}")]
    InvalidPattern(String),
}
