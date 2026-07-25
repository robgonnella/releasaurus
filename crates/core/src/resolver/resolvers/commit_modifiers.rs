use crate::{
    config::{
        overrides::CommitModifiers,
        repository::{RepositoryConfig, RewordedCommit},
    },
    result::{ReleasaurusError, Result},
};

/// Validates that a string is a valid git commit SHA (7-40 hex characters)
///
/// Returns the normalized SHA: trimmed and lowercased. Callers must use the
/// returned value rather than the input, since forge commit IDs are lowercase
/// and matching is a prefix comparison on the raw string.
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

    Ok(trimmed.to_lowercase())
}

/// Validates a SHA, naming `source` in the error.
///
/// The two inputs are merged before use, so without this the message would
/// point at `releasaurus.toml` for a value that actually came off the command
/// line (and vice versa).
fn validate_sha_from(sha: &str, source: &str) -> Result<String> {
    validate_sha(sha).map_err(|e| {
        ReleasaurusError::invalid_config(format!(
            "Invalid SHA in {source}: {e}"
        ))
    })
}

pub fn resolve_commit_modifiers(
    config: &RepositoryConfig,
    modifiers: &CommitModifiers,
) -> Result<CommitModifiers> {
    // Store the normalized SHAs, not the raw input: these are matched against
    // forge commit IDs with `starts_with`, so untrimmed or uppercase values
    // would validate and then silently never match.
    let skip_shas = config
        .skip_shas
        .iter()
        .map(|sha| validate_sha_from(sha, "repository.skip_shas"))
        .chain(
            modifiers
                .skip_shas
                .iter()
                .map(|sha| validate_sha_from(sha, "--skip-sha")),
        )
        .collect::<Result<Vec<String>>>()?;

    let mut reword = config
        .reword
        .iter()
        .cloned()
        .map(|mut entry| {
            entry.sha = validate_sha_from(&entry.sha, "repository.reword")?;
            Ok(entry)
        })
        .collect::<Result<Vec<_>>>()?;

    for entry in modifiers.reword.iter() {
        let sha = validate_sha_from(&entry.sha, "--reword")?;

        // cli overrides existing config for same sha
        if let Some(r) = reword.iter_mut().find(|e| e.sha == sha) {
            r.message = entry.message.clone();
        } else {
            reword.push(RewordedCommit {
                sha,
                message: entry.message.clone(),
            });
        }
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

    /// The two sources are merged before use, so the error has to name the
    /// one the bad value actually came from - pointing a `--skip-sha` typo at
    /// `releasaurus.toml` sends the user to the wrong file.
    #[test]
    fn resolve_commit_modifiers_names_cli_as_source_for_invalid_skip_sha() {
        let cli = CommitModifiers {
            skip_shas: vec!["abc".to_string()], // too short
            ..Default::default()
        };

        let result =
            resolve_commit_modifiers(&RepositoryConfig::default(), &cli);

        assert!(matches!(result, Err(ReleasaurusError::InvalidConfig(_))));

        let err = result.unwrap_err().to_string();
        assert!(err.contains("--skip-sha"), "unexpected error: {err}");
        assert!(
            !err.contains("repository.skip_shas"),
            "error blames the config file: {err}"
        );
    }

    #[test]
    fn resolve_commit_modifiers_names_cli_as_source_for_invalid_reword_sha() {
        let cli = CommitModifiers {
            reword: vec![RewordedCommit {
                sha: "zzzzzzz".to_string(), // not hexadecimal
                message: "x".to_string(),
            }],
            ..Default::default()
        };

        let result =
            resolve_commit_modifiers(&RepositoryConfig::default(), &cli);

        assert!(matches!(result, Err(ReleasaurusError::InvalidConfig(_))));

        let err = result.unwrap_err().to_string();
        assert!(err.contains("--reword"), "unexpected error: {err}");
        assert!(
            !err.contains("repository.reword"),
            "error blames the config file: {err}"
        );
    }

    /// SHAs are normalized before the override lookup, so a CLI entry that
    /// differs from the config entry only in case still overrides it rather
    /// than pushing a duplicate that later loses to the config value.
    #[test]
    fn resolve_commit_modifiers_cli_reword_overrides_config_across_sha_case() {
        let config = RepositoryConfig {
            reword: vec![RewordedCommit {
                sha: "abc1234".to_string(),
                message: "from config".to_string(),
            }],
            ..Default::default()
        };
        let cli = CommitModifiers {
            reword: vec![RewordedCommit {
                sha: "ABC1234".to_string(),
                message: "from cli".to_string(),
            }],
            ..Default::default()
        };

        let result = resolve_commit_modifiers(&config, &cli).unwrap();

        assert_eq!(result.reword.len(), 1);
        assert_eq!(result.reword[0].sha, "abc1234");
        assert_eq!(result.reword[0].message, "from cli");
    }
}
