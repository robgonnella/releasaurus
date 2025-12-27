//! File loading abstraction for manifest content retrieval.
//!
//! Provides a trait for loading file content from various sources (forge APIs,
//! local filesystem, test mocks, etc.) without coupling updaters to specific
//! implementations.

use async_trait::async_trait;

use crate::Result;

/// Abstraction for loading file content from a source.
///
/// This trait allows updaters to load manifest files without depending on
/// specific forge implementations. It can be implemented by ForgeManager,
/// local filesystem adapters, test mocks, or any other file source.
#[async_trait]
pub trait FileLoader: Send + Sync {
    /// Load the content of a file from the source.
    ///
    /// # Arguments
    ///
    /// * `branch` - Optional branch name to load the file from
    /// * `path` - Path to the file relative to the repository root
    ///
    /// # Returns
    ///
    /// * `Ok(Some(String))` - File was found and content loaded successfully
    /// * `Ok(None)` - File does not exist at the specified path
    /// * `Err(_)` - An error occurred while attempting to load the file
    async fn load_file(
        &self,
        branch: Option<String>,
        path: String,
    ) -> Result<Option<String>>;
}
