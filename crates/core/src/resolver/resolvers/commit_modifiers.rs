use crate::{
    config::{Config, resolved::CommitModifiers},
    result::{ReleasaurusError, Result},
};

/// Validates that a string is a valid git commit SHA (7-40 hex characters)
pub fn validate_sha(sha: &str) -> Result<String> {
    let trimmed = sha.trim();

    if trimmed.len() < 7 {
        return Err(ReleasaurusError::invalid_config(format!(
            "Invalid commit SHA: '{}'. Must be at least 7 characters",
            sha
        )));
    }

    if trimmed.len() > 40 {
        return Err(ReleasaurusError::invalid_config(format!(
            "Invalid commit SHA: '{}'. Must not exceed 40 characters",
            sha
        )));
    }

    if !trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ReleasaurusError::invalid_config(format!(
            "Invalid commit SHA: '{}'. Must contain only hexadecimal characters (0-9, a-f)",
            sha
        )));
    }

    Ok(trimmed.to_string())
}

pub fn resolve_commit_modifiers(
    config: &Config,
    modifiers: &CommitModifiers,
) -> Result<CommitModifiers> {
    let skip_shas = if !modifiers.skip_shas.is_empty() {
        modifiers.skip_shas.clone()
    } else if let Some(list) = config.changelog.skip_shas.clone() {
        for sha in &list {
            validate_sha(sha).map_err(|e| {
                ReleasaurusError::invalid_config(format!(
                    "Invalid SHA in changelog.skip_shas: {}",
                    e
                ))
            })?;
        }
        list
    } else {
        Vec::new()
    };

    let reword = if !modifiers.reword.is_empty() {
        modifiers.reword.clone()
    } else if let Some(list) = config.changelog.reword.clone() {
        for entry in &list {
            validate_sha(&entry.sha).map_err(|e| {
                ReleasaurusError::invalid_config(format!(
                    "Invalid SHA in changelog.reword: {}",
                    e
                ))
            })?;
        }
        list
    } else {
        Vec::new()
    };

    Ok(CommitModifiers { skip_shas, reword })
}
