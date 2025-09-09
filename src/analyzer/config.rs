//! Configuration used to implement changelog traits

use crate::repo::StartingPoint;

/// The default body value for [`ChangelogConfig`]
pub const DEFAULT_BODY: &str = r#"{% if version -%}
    # [{{ version | trim_start_matches(pat="v") }}]({{ extra.release_link_base }}/{{ version }}) - {{ timestamp | date(format="%Y-%m-%d") }}
{% else -%}
    # [unreleased]
{% endif -%}
{% for group, commits in commits | filter(attribute="merge_commit", value=false) | group_by(attribute="group") %}
    ### {{ group | striptags | trim | upper_first }}
    {% for commit in commits %}
      {% if commit.breaking -%}
        {% if commit.scope %}_({{ commit.scope }})_ {% endif -%}[**breaking**]: {{ commit.message | upper_first }} [_({{ commit.id | truncate(length=8, end="") }})_]({{ extra.commit_link_base }}/{{ commit.id }})
        {% if commit.body -%}
        > {{ commit.body }}
        {% endif -%}
        {% if commit.breaking_description -%}
        > {{ commit.breaking_description }}
        {% endif -%}
      {% else -%}
        - {% if commit.scope %}_({{ commit.scope }})_ {% endif %}{{ commit.message | upper_first }} [_({{ commit.id | truncate(length=8, end="") }})_]({{ extra.commit_link_base }}/{{ commit.id -}})
      {% endif -%}
    {% endfor %}
{% endfor %}
 "#;

#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    /// Path to cloned repository
    /// default "."
    pub repo_path: String,
    /// Path to the package directory within repository
    pub package_relative_path: String,
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
    /// Optional prefix to use for the package
    pub tag_prefix: Option<String>,
    /// The base url for commit links
    /// Used to display commit links in changelog
    pub commit_link_base_url: String,
    /// The base url for release links
    /// Used to display release links in changelog
    pub release_link_base_url: String,
    /// Only process since commits since provided commit sha
    /// (tagged_release_commit, tagged_release_commit_parent)
    pub starting_point: Option<StartingPoint>,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            repo_path: ".".into(),
            package_relative_path: ".".into(),
            body: DEFAULT_BODY.into(),
            header: None,
            footer: None,
            tag_prefix: None,
            starting_point: None,
            commit_link_base_url: "".into(),
            release_link_base_url: "".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_defaults() {
        let config = AnalyzerConfig::default();
        assert!(!config.body.is_empty())
    }
}
