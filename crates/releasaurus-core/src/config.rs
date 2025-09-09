//! Configuration for releasaurus-core
use serde::Deserialize;

/// The default body value for [`ChangelogConfig`]
const DEFAULT_BODY: &str = r#"
{% if version -%}
    ## [{{ version | trim_start_matches(pat="v") }}] - {{ timestamp | date(format="%Y-%m-%d") }}
{% else -%}
    ## [unreleased]
{% endif -%}
{% for group, commits in commits | group_by(attribute="group") %}
    ### {{ group | striptags | trim | upper_first }}
    {% for commit in commits %}
      {% if commit.breaking -%}
      {% if commit.scope %}*({{ commit.scope }})* {% endif %}[**breaking**]: {{ commit.message | upper_first }}
      body: {{ commit.body }}
      footer: {{ commit.breaking_description }}
      {% else -%}
      - {% if commit.scope %}*({{ commit.scope }})* {% endif %}{{ commit.message | upper_first -}}
      {% endif -%}
    {% endfor %}
{% endfor -%}
"#;

/// Changelog Configuration allowing you to customize changelog output format
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ChangelogConfig {
    /// [Tera](https://github.com/Keats/tera) template string allowing you
    /// to modify the format of the generated changelog. A sane default is
    /// provided which includes release versions and commit groupings by type
    ///
    /// default: [`DEFAULT_BODY`]
    pub body: String,
    /// Optional tera template to modify the changelog header
    ///
    /// default: [`None`]
    pub header: Option<String>,
    /// Optional tera template to modify the changelog footer
    ///
    /// default: [`None`]
    pub footer: Option<String>,
}

impl Default for ChangelogConfig {
    fn default() -> Self {
        Self {
            body: DEFAULT_BODY.to_string(),
            header: None,
            footer: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
/// Package configuration specifying which packages to track as separate
/// releases in this repository
pub struct PackageConfig {
    /// The name of the package. This can be an arbitrary name but it's common
    /// for it to match the directory name of the package
    pub name: String,
    /// Path to a valid directory for the package
    pub path: String,
    /// Optional prefix to use for the package
    ///
    /// default: <name>-v
    pub tag_prefix: Option<String>,
}

impl Default for PackageConfig {
    fn default() -> Self {
        Self {
            name: "".to_string(),
            path: ".".to_string(),
            tag_prefix: Some("v".to_string()),
        }
    }
}

/// Complete configuration for the core
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// [`ChangelogConfig`]
    pub changelog: ChangelogConfig,
    /// [`Vec<PackageConfig>`]
    pub packages: Vec<PackageConfig>,
}

impl Default for Config {
    fn default() -> Self {
        let pkgs = vec![PackageConfig::default()];
        let chglg = ChangelogConfig::default();

        Self {
            packages: pkgs,
            changelog: chglg,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_defaults() {
        let config = Config::default();
        assert!(!config.changelog.body.is_empty())
    }
}
