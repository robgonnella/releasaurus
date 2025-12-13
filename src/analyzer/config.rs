//! Configuration for changelog generation and commit analysis.

use regex::Regex;

/// Default changelog body template.
pub const DEFAULT_BODY: &str = r#"# [{{ version  }}]({{ link }}) - {{ timestamp | date(format="%Y-%m-%d") }}
{% for group, commits in commits | filter(attribute="merge_commit", value=false) | group_by(attribute="group") %}
### {{ group | striptags | trim }}
{% for commit in commits %}
{% if commit.breaking -%}
{% if commit.scope %}_({{ commit.scope }})_ {% endif -%}[**breaking**]: {{ commit.title }} [_({{ commit.short_id }})_]({{ commit.link }}){% if include_author %} ({{ commit.author_name }}){% endif %}
{% if commit.body -%}
> {{ commit.body }}
{% endif -%}
{% if commit.breaking_description -%}
> {{ commit.breaking_description }}
{% endif -%}
{% else -%}
- {% if commit.scope %}_({{ commit.scope }})_ {% endif %}{{ commit.title }} [_({{ commit.short_id }})_]({{ commit.link }}){% if include_author %} ({{ commit.author_name }}){% endif %}
{% endif -%}
{% endfor %}
{% endfor %}
 "#;

/// Configuration for commit analysis and changelog generation.
#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    /// Tera template string for changelog body format.
    pub body: String,
    /// Skips including ci commits in changelog (default: false)
    pub skip_ci: bool,
    /// Skips including ci commits in changelog (default: false)
    pub skip_chore: bool,
    /// Skips including miscellaneous commits in changelog (default: false)
    pub skip_miscellaneous: bool,
    /// Skips including merge commits in changelog (default: true)
    pub skip_merge_commits: bool,
    /// Skips including release commits in changelog (default: true)
    pub skip_release_commits: bool,
    /// Includes commit author in default body template (default: false)
    pub include_author: bool,
    /// Optional prefix for package tags.
    pub tag_prefix: Option<String>,
    /// Base URL for release links in changelog.
    pub release_link_base_url: String,
    /// Prerelease identifier (e.g., "alpha", "beta", "rc").
    pub prerelease: Option<String>,
    /// Whether to append .1, .2, etc. to prerelease versions
    pub prerelease_version: bool,
    /// regex to match and exclude release commits
    pub release_commit_matcher: Option<Regex>,
    /// Always increments major version on breaking commits
    pub breaking_always_increment_major: bool,
    /// Always increments minor version on feature commits
    pub features_always_increment_minor: bool,
    /// Custom commit type regex matcher to increment major version
    pub custom_major_increment_regex: Option<String>,
    /// Custom commit type regex matcher to increment minor version
    pub custom_minor_increment_regex: Option<String>,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            body: DEFAULT_BODY.into(),
            skip_ci: false,
            skip_chore: false,
            skip_miscellaneous: false,
            skip_merge_commits: true,
            skip_release_commits: true,
            include_author: false,
            tag_prefix: None,
            release_link_base_url: "".into(),
            prerelease: None,
            prerelease_version: true,
            release_commit_matcher: None,
            breaking_always_increment_major: true,
            features_always_increment_minor: true,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
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
