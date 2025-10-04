//! Unified error handling using `color-eyre` for enhanced error reporting.

use color_eyre::eyre::Result as EyreResult;

/// Type alias for Result with color-eyre error reporting and diagnostics.
pub type Result<T> = EyreResult<T>;
