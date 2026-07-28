//! Configuration loading and parsing for `releasaurus.toml` files.
//!
//! Supports customizable changelog templates and multi-package repositories.
use derive_builder::Builder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::{
    defaults::DefaultsConfig, package::PackageConfig,
    repository::RepositoryConfig,
};

/// Default configuration filename
pub const DEFAULT_CONFIG_FILE: &str = "releasaurus.toml";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Builder)]
#[schemars(rename = "Releasaurus TOML Configuration Schema")]
#[serde(default, deny_unknown_fields)]
#[builder(setter(into, strip_option), default)]
/// Configuration properties for `releasaurus.toml`
pub struct Config {
    /// Repository configuration
    pub repository: RepositoryConfig,
    /// Default configuration applied to every package
    pub defaults: DefaultsConfig,
    /// Packages to manage in this repository (supports monorepos)
    #[serde(rename = "package")]
    pub packages: Vec<PackageConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            repository: RepositoryConfig::default(),
            defaults: DefaultsConfig::default(),
            packages: vec![PackageConfig::default()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Yields the contents of each ```toml fenced block in a markdown source.
    fn toml_blocks(source: &str) -> impl Iterator<Item = &str> {
        source
            .split("```toml\n")
            .skip(1)
            .filter_map(|rest| rest.split_once("\n```").map(|(block, _)| block))
    }

    /// The repository's own config must always parse. `deny_unknown_fields`
    /// means a config key that moves without this file being updated is a
    /// hard error rather than a silent no-op.
    #[test]
    fn parses_this_repositorys_config() {
        let raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../releasaurus.toml"
        ));

        let config: Config = toml::from_str(raw).unwrap();

        assert_eq!(config.repository.tag_search_depth, 25);
        assert_eq!(config.packages.len(), 1);

        let versioning = config.defaults.versioning.unwrap();
        let named_parsers = versioning.named_parsers.unwrap();

        assert_eq!(named_parsers.len(), 3);
    }

    /// Every ```toml example in the book must parse as a real `Config`.
    ///
    /// With `deny_unknown_fields`, this catches documented keys that don't
    /// exist or sit under the wrong table - the docs cannot drift from the
    /// config structs without failing here.
    #[test]
    fn parses_every_toml_example_in_the_book() {
        const PAGES: &[(&str, &str)] = &[
            (
                "configuration.md",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../book/src/configuration.md"
                )),
            ),
            (
                "configuration-reference.md",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../book/src/configuration-reference.md"
                )),
            ),
            (
                "changelog.md",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../book/src/changelog.md"
                )),
            ),
        ];

        let mut checked = 0;

        for (page, source) in PAGES {
            for (index, block) in toml_blocks(source).enumerate() {
                if let Err(e) = ::toml::from_str::<Config>(block) {
                    panic!(
                        "{page} toml block {index} is not a valid config: {e}\n\n{block}"
                    );
                }
                checked += 1;
            }
        }

        // guard against the extractor silently matching nothing
        assert!(checked > 10, "only found {checked} toml examples");
    }

    /// Options removed in favor of `named_parsers` must fail loudly so
    /// migrating users are not left with a config that silently filters
    /// nothing.
    #[test]
    fn rejects_removed_skip_options() {
        let raw = r#"
            [defaults.changelog]
            skip_ci = true
        "#;

        let err = toml::from_str::<Config>(raw).unwrap_err().to_string();

        assert!(err.contains("skip_ci"), "unexpected error: {err}");
    }

    /// Parsers live under `[defaults.versioning]`, not `[defaults.changelog]`.
    #[test]
    fn rejects_parsers_under_changelog() {
        let raw = r#"
            [defaults.changelog.named_parsers]
            ci.skip = true
        "#;

        let err = toml::from_str::<Config>(raw).unwrap_err().to_string();

        assert!(err.contains("named_parsers"), "unexpected error: {err}");
    }

    /// `custom_parser` is singular, matching the `[[package]]` convention.
    #[test]
    fn rejects_pluralized_custom_parser_key() {
        let raw = r#"
            [[defaults.versioning.custom_parsers]]
            pattern = "^deps"
            title = "Dependencies"
        "#;

        let err = toml::from_str::<Config>(raw).unwrap_err().to_string();

        assert!(err.contains("custom_parsers"), "unexpected error: {err}");
    }

    #[test]
    fn accepts_custom_parser_key() {
        let raw = r#"
            [[defaults.versioning.custom_parser]]
            pattern = "^deps"
            title = "📦 Dependencies"
            order = 3
            skip = false
        "#;

        let config: Config = toml::from_str(raw).unwrap();

        let parsers =
            config.defaults.versioning.unwrap().custom_parsers.unwrap();

        assert_eq!(parsers.0.len(), 1);
        assert_eq!(parsers.0[0].pattern.as_ref().unwrap().as_str(), "^deps");
        assert_eq!(parsers.0[0].order, Some(3));
    }

    #[test]
    fn parses_commit_and_pr_title_templates() {
        let raw = r#"
            [defaults]
            monorepo_commit_message_template = "chore: release {{ repo_name }}"
            monorepo_pr_title_template = "Release {{ repo_name }}"
            commit_message_template = "chore: {{ package_name }} {{ tag }}"
            pr_title_template = "Release {{ package_name }} {{ tag }}"

            [[package]]
            name = "frontend"
            commit_message_template = "web: {{ tag }}"
            pr_title_template = "Web {{ tag }}"
        "#;

        let config: Config = toml::from_str(raw).unwrap();

        assert_eq!(
            config.defaults.monorepo_commit_message_template.unwrap(),
            "chore: release {{ repo_name }}"
        );
        assert_eq!(
            config.defaults.monorepo_pr_title_template.unwrap(),
            "Release {{ repo_name }}"
        );
        assert_eq!(
            config.defaults.commit_message_template.unwrap(),
            "chore: {{ package_name }} {{ tag }}"
        );
        assert_eq!(
            config.defaults.pr_title_template.unwrap(),
            "Release {{ package_name }} {{ tag }}"
        );

        let package = &config.packages[0];

        assert_eq!(
            package.commit_message_template.as_deref().unwrap(),
            "web: {{ tag }}"
        );
        assert_eq!(
            package.pr_title_template.as_deref().unwrap(),
            "Web {{ tag }}"
        );
    }

    /// The `monorepo_*` templates describe a PR spanning several packages,
    /// so there is nothing for a single package to override. Setting one
    /// there must fail rather than be silently ignored.
    #[test]
    fn rejects_monorepo_template_under_package() {
        let raw = r#"
            [[package]]
            name = "frontend"
            monorepo_pr_title_template = "Release {{ repo_name }}"
        "#;

        let err = toml::from_str::<Config>(raw).unwrap_err().to_string();

        assert!(
            err.contains("monorepo_pr_title_template"),
            "unexpected error: {err}"
        );
    }
}
