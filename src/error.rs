//! Custom error types for Releasaurus with improved type safety and error handling.

use thiserror::Error;

/// Main error type for Releasaurus operations.
#[derive(Error, Debug)]
pub enum ReleasaurusError {
    // Cli args errors
    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),

    // Configuration errors
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Base branch not configured")]
    BaseBranchNotConfigured,

    // Forge/Git errors
    #[error("Forge operation failed: {0}")]
    ForgeError(String),

    #[error(
        "Found pending release (PR #{pr_number}) on branch '{branch}' that has not been tagged yet: cannot continue, must finish previous release first"
    )]
    PendingRelease { branch: String, pr_number: u64 },

    #[error("Invalid git remote URL: {0}")]
    InvalidRemoteUrl(String),

    #[error("Git URL parse error: {0}")]
    GitUrlError(#[from] git_url_parse::GitUrlParseError),

    #[error("Git operation failed: {0}")]
    GitError(#[from] git2::Error),

    // Network/API errors
    #[error("Network request failed: {0}")]
    NetworkError(String),

    #[error("API authentication failed: {0}")]
    AuthenticationError(String),

    #[error("API rate limit exceeded")]
    RateLimitExceeded,

    // Version/parsing errors - automatic conversions via #[from]
    #[error("Invalid version format: {0}")]
    InvalidVersion(#[from] semver::Error),

    #[error("Template rendering failed: {0}")]
    TemplateError(#[from] tera::Error),

    // TOML parsing errors
    #[error("TOML parse error: {0}")]
    TomlParseError(#[from] toml::de::Error),

    #[error("TOML edit error: {0}")]
    TomlEditError(#[from] toml_edit::TomlError),

    // JSON parsing errors
    #[error("JSON parse error: {0}")]
    JsonParseError(#[from] serde_json::Error),

    // XML parsing errors
    #[error("XML parse error: {0}")]
    XmlError(#[from] quick_xml::Error),

    // Additional parsing errors
    #[error("Regular expression error: {0}")]
    RegexError(#[from] regex::Error),

    #[error("Datetime parse error: {0}")]
    ChronoParseError(#[from] chrono::ParseError),

    #[error("Base64 decode error: {0}")]
    Base64DecodeError(#[from] base64::DecodeError),

    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[error("URL parse error: {0}")]
    UrlError(#[from] url::ParseError),

    #[error("Logger initialization error: {0}")]
    LoggerError(#[from] log::SetLoggerError),

    // Generic wrapper for other errors
    #[error(transparent)]
    Other(#[from] color_eyre::Report),
}

/// Result type alias using ReleasaurusError
pub type Result<T> = std::result::Result<T, ReleasaurusError>;

impl ReleasaurusError {
    /// Create a forge error with context
    pub fn forge(msg: impl Into<String>) -> Self {
        Self::ForgeError(msg.into())
    }

    /// Create an invalid config error
    pub fn invalid_config(msg: impl Into<String>) -> Self {
        Self::InvalidConfig(msg.into())
    }

    /// Create a pending release error
    pub fn pending_release(branch: impl Into<String>, pr_number: u64) -> Self {
        Self::PendingRelease {
            branch: branch.into(),
            pr_number,
        }
    }
}

// Implement From for std::io::Error - wraps in Other variant for generic I/O errors
impl From<std::io::Error> for ReleasaurusError {
    fn from(err: std::io::Error) -> Self {
        Self::Other(color_eyre::Report::from(err))
    }
}

// Implement From for reqwest errors (network/API)
impl From<reqwest::Error> for ReleasaurusError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() || err.is_connect() {
            Self::NetworkError(err.to_string())
        } else if err.is_status() {
            if let Some(status) = err.status() {
                if status.as_u16() == 401 || status.as_u16() == 403 {
                    Self::AuthenticationError(err.to_string())
                } else if status.as_u16() == 429 {
                    Self::RateLimitExceeded
                } else {
                    Self::NetworkError(err.to_string())
                }
            } else {
                Self::NetworkError(err.to_string())
            }
        } else {
            Self::NetworkError(err.to_string())
        }
    }
}

// Implement From for reqwest header errors (needs custom message)
impl From<reqwest::header::InvalidHeaderValue> for ReleasaurusError {
    fn from(err: reqwest::header::InvalidHeaderValue) -> Self {
        Self::AuthenticationError(format!("Invalid header value: {}", err))
    }
}

// Implement From for octocrab errors (GitHub API)
impl From<octocrab::Error> for ReleasaurusError {
    fn from(err: octocrab::Error) -> Self {
        match &err {
            octocrab::Error::GitHub { source, .. }
                if source.message.contains("rate limit") =>
            {
                Self::RateLimitExceeded
            }
            _ => Self::ForgeError(format!("GitHub API error: {}", err)),
        }
    }
}

// Implement From for gitlab errors
impl From<gitlab::api::ApiError<gitlab::RestError>> for ReleasaurusError {
    fn from(err: gitlab::api::ApiError<gitlab::RestError>) -> Self {
        Self::ForgeError(format!("GitLab API error: {}", err))
    }
}

impl From<gitlab::GitlabError> for ReleasaurusError {
    fn from(err: gitlab::GitlabError) -> Self {
        Self::ForgeError(format!("GitLab error: {}", err))
    }
}

// Implement From for various builder errors from derive_builder
// These are used by octocrab and gitlab crates

impl From<gitlab::api::projects::merge_requests::MergeRequestsBuilderError>
    for ReleasaurusError
{
    fn from(
        err: gitlab::api::projects::merge_requests::MergeRequestsBuilderError,
    ) -> Self {
        Self::Other(color_eyre::Report::msg(format!("Builder error: {}", err)))
    }
}

impl From<gitlab::api::projects::repository::commits::CreateCommitBuilderError>
    for ReleasaurusError
{
    fn from(
        err: gitlab::api::projects::repository::commits::CreateCommitBuilderError,
    ) -> Self {
        Self::Other(color_eyre::Report::msg(format!("Builder error: {}", err)))
    }
}

impl From<gitlab::api::projects::repository::commits::CommitActionBuilderError>
    for ReleasaurusError
{
    fn from(
        err: gitlab::api::projects::repository::commits::CommitActionBuilderError,
    ) -> Self {
        Self::Other(color_eyre::Report::msg(format!("Builder error: {}", err)))
    }
}

impl From<gitlab::api::projects::repository::files::FileBuilderError>
    for ReleasaurusError
{
    fn from(
        err: gitlab::api::projects::repository::files::FileBuilderError,
    ) -> Self {
        Self::Other(color_eyre::Report::msg(format!("Builder error: {}", err)))
    }
}

impl From<gitlab::api::projects::merge_requests::CreateMergeRequestBuilderError>
    for ReleasaurusError
{
    fn from(
        err: gitlab::api::projects::merge_requests::CreateMergeRequestBuilderError,
    ) -> Self {
        Self::Other(color_eyre::Report::msg(format!("Builder error: {}", err)))
    }
}

impl From<gitlab::api::projects::merge_requests::EditMergeRequestBuilderError>
    for ReleasaurusError
{
    fn from(
        err: gitlab::api::projects::merge_requests::EditMergeRequestBuilderError,
    ) -> Self {
        Self::Other(color_eyre::Report::msg(format!("Builder error: {}", err)))
    }
}

impl From<gitlab::api::projects::labels::CreateLabelBuilderError>
    for ReleasaurusError
{
    fn from(
        err: gitlab::api::projects::labels::CreateLabelBuilderError,
    ) -> Self {
        Self::Other(color_eyre::Report::msg(format!("Builder error: {}", err)))
    }
}

impl From<gitlab::api::projects::repository::tags::TagsBuilderError>
    for ReleasaurusError
{
    fn from(
        err: gitlab::api::projects::repository::tags::TagsBuilderError,
    ) -> Self {
        Self::Other(color_eyre::Report::msg(format!("Builder error: {}", err)))
    }
}

impl From<gitlab::api::projects::repository::tags::TagBuilderError>
    for ReleasaurusError
{
    fn from(
        err: gitlab::api::projects::repository::tags::TagBuilderError,
    ) -> Self {
        Self::Other(color_eyre::Report::msg(format!("Builder error: {}", err)))
    }
}

impl From<gitlab::api::projects::repository::tags::CreateTagBuilderError>
    for ReleasaurusError
{
    fn from(
        err: gitlab::api::projects::repository::tags::CreateTagBuilderError,
    ) -> Self {
        Self::Other(color_eyre::Report::msg(format!("Builder error: {}", err)))
    }
}

impl From<gitlab::api::projects::releases::CreateReleaseBuilderError>
    for ReleasaurusError
{
    fn from(
        err: gitlab::api::projects::releases::CreateReleaseBuilderError,
    ) -> Self {
        Self::Other(color_eyre::Report::msg(format!("Builder error: {}", err)))
    }
}

// Additional builder errors that were missing
impl From<gitlab::api::projects::ProjectBuilderError> for ReleasaurusError {
    fn from(err: gitlab::api::projects::ProjectBuilderError) -> Self {
        Self::Other(color_eyre::Report::msg(format!("Builder error: {}", err)))
    }
}

impl From<gitlab::api::projects::labels::LabelsBuilderError>
    for ReleasaurusError
{
    fn from(err: gitlab::api::projects::labels::LabelsBuilderError) -> Self {
        Self::Other(color_eyre::Report::msg(format!("Builder error: {}", err)))
    }
}

impl From<gitlab::api::projects::repository::commits::CommitsBuilderError>
    for ReleasaurusError
{
    fn from(
        err: gitlab::api::projects::repository::commits::CommitsBuilderError,
    ) -> Self {
        Self::Other(color_eyre::Report::msg(format!("Builder error: {}", err)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_formats() {
        let err = ReleasaurusError::forge("API call failed");
        assert_eq!(err.to_string(), "Forge operation failed: API call failed");

        let err = ReleasaurusError::invalid_config("missing field");
        assert_eq!(err.to_string(), "Invalid configuration: missing field");
    }

    #[test]
    fn test_error_helpers() {
        let err = ReleasaurusError::forge("API call failed");
        assert!(matches!(err, ReleasaurusError::ForgeError(_)));

        let err = ReleasaurusError::invalid_config("missing field");
        assert!(matches!(err, ReleasaurusError::InvalidConfig(_)));

        let err = ReleasaurusError::pending_release("main", 42);
        assert!(matches!(err, ReleasaurusError::PendingRelease { .. }));
    }

    #[test]
    fn test_from_conversions() {
        let semver_err = semver::Version::parse("invalid");
        assert!(semver_err.is_err());
        let err: ReleasaurusError = semver_err.unwrap_err().into();
        assert!(matches!(err, ReleasaurusError::InvalidVersion(_)));
    }
}
