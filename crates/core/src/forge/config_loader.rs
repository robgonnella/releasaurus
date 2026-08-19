//! Loads `releasaurus.toml` from a repository via a [`Forge`].

use crate::{
    config::{Config, DEFAULT_CONFIG_FILE},
    forge::{request::GetFileContentRequest, traits::Forge},
    result::{ReleasaurusError, Result},
};

/// Load and parse `releasaurus.toml` from the repository root.
///
/// `base_branch` defaults to the forge's default branch. A missing
/// `config_path` is an error, since the caller asked for that file
/// specifically; a missing default file is not, and yields
/// [`Config::default`].
///
/// Runs before a [`ForgeManager`][crate::forge::manager::ForgeManager]
/// exists, because the config supplies the search depths the forge is
/// configured with.
pub async fn load_config(
    forge: &dyn Forge,
    base_branch: Option<&str>,
    config_path: Option<&str>,
) -> Result<Config> {
    let branch = base_branch
        .map(String::from)
        .unwrap_or_else(|| forge.default_branch());

    let path = config_path.unwrap_or(DEFAULT_CONFIG_FILE);

    log::info!(
        "Loading configuration from forge (branch: {branch}, config: {path})"
    );

    let content = forge
        .get_file_content(GetFileContentRequest {
            branch: Some(branch),
            path: path.to_string(),
        })
        .await?;

    match content {
        Some(content) => Ok(::toml::from_str(&content)?),
        None if config_path.is_some() => Err(ReleasaurusError::invalid_config(
            format!("configuration file not found at: {path}"),
        )),
        None => {
            log::info!("repository configuration not found: using default");
            Ok(Config::default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{forge::traits::MockForge, result::ReleasaurusError};

    const TOML: &str = r#"
[repository]
base_branch = "develop"
first_release_search_depth = 50
tag_search_depth = 10
"#;

    /// A forge that serves `TOML` at `path` and nothing anywhere else.
    fn forge_serving(path: &'static str) -> MockForge {
        let mut mock = MockForge::new();

        mock.expect_get_file_content().returning(move |req| {
            Ok((req.path == path).then(|| TOML.to_string()))
        });

        mock
    }

    #[tokio::test]
    async fn reads_the_default_path_when_none_is_given() {
        let mock = forge_serving(DEFAULT_CONFIG_FILE);

        let config = load_config(&mock, Some("main"), None).await.unwrap();

        assert_eq!(config.repository.first_release_search_depth, 50);
        assert_eq!(config.repository.tag_search_depth, 10);
    }

    /// A custom path is read *instead of* the default, so a forge serving
    /// only the custom file still resolves.
    #[tokio::test]
    async fn reads_a_custom_path_instead_of_the_default() {
        let mock = forge_serving("my-config.toml");

        let config = load_config(&mock, Some("main"), Some("my-config.toml"))
            .await
            .unwrap();

        assert_eq!(config.repository.base_branch, Some("develop".into()));
    }

    /// Asking for a specific file that isn't there is an error; falling back
    /// to defaults would silently ignore what the caller asked for.
    #[tokio::test]
    async fn errors_when_a_custom_path_is_missing() {
        let mock = forge_serving(DEFAULT_CONFIG_FILE);

        let err = load_config(&mock, Some("main"), Some("nope.toml"))
            .await
            .unwrap_err();

        assert!(matches!(err, ReleasaurusError::InvalidConfig(_)));
    }

    #[tokio::test]
    async fn falls_back_to_defaults_when_no_config_file_exists() {
        let mock = forge_serving("something-else.toml");

        let config = load_config(&mock, Some("main"), None).await.unwrap();

        assert_eq!(
            config.repository.tag_search_depth,
            Config::default().repository.tag_search_depth
        );
    }

    /// Without an explicit base branch the lookup targets the forge's
    /// default branch, since that is where the config is read from.
    #[tokio::test]
    async fn defaults_the_branch_to_the_forge_default_branch() {
        let mut mock = MockForge::new();

        mock.expect_default_branch().returning(|| "trunk".into());
        mock.expect_get_file_content()
            .withf(|req| req.branch.as_deref() == Some("trunk"))
            .times(1)
            .returning(|_| Ok(None));

        load_config(&mock, None, None).await.unwrap();
    }
}
