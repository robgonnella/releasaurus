//! Unified error handling using `color-eyre` for enhanced error reporting.

use color_eyre::eyre::Result as EyreResult;

/// Type alias for `color_eyre::eyre::Result<T>` with enhanced error reporting.
pub type Result<T> = EyreResult<T>;
