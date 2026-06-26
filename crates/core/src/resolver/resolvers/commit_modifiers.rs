use crate::{
    config::{repository::RepositoryConfig, resolved::CommitModifiers},
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
    config: &RepositoryConfig,
    modifiers: &CommitModifiers,
) -> Result<CommitModifiers> {
    let mut skip_shas = config.skip_shas.clone();
    skip_shas.extend(modifiers.skip_shas.clone());

    for sha in &skip_shas {
        validate_sha(sha).map_err(|e| {
            ReleasaurusError::invalid_config(format!(
                "Invalid SHA in repository.skip_shas: {}",
                e
            ))
        })?;
    }

    let mut reword = config.reword.clone();

    for entry in modifiers.reword.iter() {
        // cli overrides existing config for same sha
        if let Some(r) = reword.iter_mut().find(|e| e.sha == entry.sha) {
            r.message = entry.message.clone();
        } else {
            reword.push(entry.clone());
        }
    }

    for entry in &reword {
        validate_sha(&entry.sha).map_err(|e| {
            ReleasaurusError::invalid_config(format!(
                "Invalid SHA in repository.reword: {}",
                e
            ))
        })?;
    }

    Ok(CommitModifiers { skip_shas, reword })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::repository::RewordedCommit;

    #[test]
    fn resolve_commit_modifiers_unions_skip_shas() {
        let config = RepositoryConfig {
            skip_shas: vec!["aaaaaaa".to_string()],
            ..Default::default()
        };
        let cli = CommitModifiers {
            skip_shas: vec!["bbbbbbb".to_string()],
            ..Default::default()
        };

        let result = resolve_commit_modifiers(&config, &cli).unwrap();

        assert_eq!(result.skip_shas.len(), 2);
        assert!(result.skip_shas.contains(&"aaaaaaa".to_string()));
        assert!(result.skip_shas.contains(&"bbbbbbb".to_string()));
    }

    #[test]
    fn resolve_commit_modifiers_cli_reword_overrides_config_for_same_sha() {
        let config = RepositoryConfig {
            reword: vec![RewordedCommit {
                sha: "abc1234".to_string(),
                message: "from config".to_string(),
            }],
            ..Default::default()
        };
        let cli = CommitModifiers {
            reword: vec![
                RewordedCommit {
                    sha: "abc1234".to_string(),
                    message: "from cli".to_string(),
                },
                RewordedCommit {
                    sha: "def5678".to_string(),
                    message: "cli only".to_string(),
                },
            ],
            ..Default::default()
        };

        let result = resolve_commit_modifiers(&config, &cli).unwrap();

        // Same SHA: CLI wins and the config entry is not duplicated.
        assert_eq!(result.reword.len(), 2);
        let same = result.reword.iter().find(|r| r.sha == "abc1234").unwrap();
        assert_eq!(same.message, "from cli");
        // The CLI-only entry is appended.
        let cli_only =
            result.reword.iter().find(|r| r.sha == "def5678").unwrap();
        assert_eq!(cli_only.message, "cli only");
    }

    #[test]
    fn resolve_commit_modifiers_rejects_invalid_skip_sha() {
        let config = RepositoryConfig {
            skip_shas: vec!["abc".to_string()], // too short
            ..Default::default()
        };

        let err =
            resolve_commit_modifiers(&config, &CommitModifiers::default())
                .unwrap_err();

        assert!(err.to_string().contains("repository.skip_shas"));
    }

    #[test]
    fn resolve_commit_modifiers_rejects_invalid_reword_sha() {
        let config = RepositoryConfig {
            reword: vec![RewordedCommit {
                sha: "zzzzzzz".to_string(), // not hexadecimal
                message: "x".to_string(),
            }],
            ..Default::default()
        };

        let err =
            resolve_commit_modifiers(&config, &CommitModifiers::default())
                .unwrap_err();

        assert!(err.to_string().contains("repository.reword"));
    }
}
