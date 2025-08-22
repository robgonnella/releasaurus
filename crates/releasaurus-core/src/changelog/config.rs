//! Configuration for releasaurus-core

use crate::config::{Remote, RemoteConfig};

/// The default body value for [`ChangelogConfig`]
pub const DEFAULT_BODY: &str = r#"{% if version -%}
    # [{{ version | trim_start_matches(pat="v") }}]{% if extra.version_link %}({{ extra.version_link }}){% endif %} - {{ timestamp | date(format="%Y-%m-%d") }}
{% else -%}
    # [unreleased]
{% endif -%}
{% for group, commits in commits | filter(attribute="merge_commit", value=false) | group_by(attribute="group") %}
    ### {{ group | striptags | trim | upper_first }}
    {% for commit in commits %}
      {% if commit.breaking -%}
        {% if commit.scope %}_({{ commit.scope }})_ {% endif -%}[**breaking**]: {{ commit.message | upper_first }} {% if commit.extra and commit.extra.link %}[_({{ commit.id | truncate(length=8, end="") }})_]({{ commit.extra.link }}){% endif %}
        {% if commit.body -%}
        > {{ commit.body }}
        {% endif -%}
        {% if commit.breaking_description -%}
        > {{ commit.breaking_description }}
        {% endif -%}
      {% else -%}
        - {% if commit.scope %}_({{ commit.scope }})_ {% endif %}{{ commit.message | upper_first }} {% if commit.extra and commit.extra.link %}[_({{ commit.id | truncate(length=8, end="") }})_]({{ commit.extra.link }}){% endif -%}
      {% endif -%}
    {% endfor %}
{% endfor %}
 "#;

#[derive(Debug, Clone)]
/// Package configuration specifying which packages to track as separate
/// releases in this repository
pub struct PackageConfig {
    /// The name of the package. This can be an arbitrary name but it's common
    /// for it to match the directory name of the package
    pub name: String,
    /// Path to a valid directory for the package
    pub path: String,
    /// Optional prefix to use for the package
    pub tag_prefix: Option<String>,
}

impl Default for PackageConfig {
    fn default() -> Self {
        Self {
            name: "".to_string(),
            path: ".".to_string(),
            tag_prefix: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChangelogConfig {
    /// [Tera](https://github.com/Keats/tera) template string allowing you
    /// to modify the format of the generated changelog.
    pub body: String,
    /// Optional tera template to modify the changelog header
    ///
    /// default: [`None`]
    pub header: Option<String>,
    /// Optional tera template to modify the changelog footer
    ///
    /// default: [`None`]
    pub footer: Option<String>,
    /// [`PackageConfig`]
    pub package: PackageConfig,
    ///  The enabled [`Remote`] for this package
    pub remote: Remote,
}

impl Default for ChangelogConfig {
    fn default() -> Self {
        Self {
            body: DEFAULT_BODY.to_string(),
            header: None,
            footer: None,
            package: PackageConfig::default(),
            remote: Remote::Github(RemoteConfig::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_defaults() {
        let config = ChangelogConfig::default();
        assert!(!config.body.is_empty())
    }
}
