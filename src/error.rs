use thiserror::Error;
use std::path::PathBuf;

/// Custom Error for dirsize
#[derive(Debug, Error, PartialEq)]
pub enum DirsizeError {
    #[error("PermissionDenied: Cannot access `{0}`")]
    PermissionDenied(PathBuf),
    #[error("Unable to process `{0}`")]
    Other(PathBuf),
    #[error("MetadataError: Unable to retrieve metadata for `{0}`")]
    Metadata(PathBuf),
    #[error("UnknownError: Cannot record path")]
    UnknownError,
}
