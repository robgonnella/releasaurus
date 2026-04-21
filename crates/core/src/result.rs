mod error;

pub use error::*;

pub use crate::result::ReleasaurusError;

/// Result type alias using ReleasaurusError
pub type Result<T> = std::result::Result<T, ReleasaurusError>;
