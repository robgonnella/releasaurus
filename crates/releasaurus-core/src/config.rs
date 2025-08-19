use serde::Deserialize;

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

/// Changelog Configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ChangelogConfig {
    pub body: String,
    pub header: Option<String>,
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
pub struct PackageConfig {
    pub name: String,
    pub path: String,
    pub tag_prefix: String,
}

impl Default for PackageConfig {
    fn default() -> Self {
        Self {
            name: ".".to_string(),
            path: ".".to_string(),
            tag_prefix: "v".to_string(),
        }
    }
}

/// Configuration for the core
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub changelog: ChangelogConfig,
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
