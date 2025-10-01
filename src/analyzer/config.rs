//! Configuration for changelog generation and commit analysis.

/// Default changelog body template.
pub const DEFAULT_BODY: &str = r#"# [{{ version  }}]({{ link }}) - {{ timestamp | date(format="%Y-%m-%d") }}
{% for group, commits in commits | filter(attribute="merge_commit", value=false) | group_by(attribute="group") %}
### {{ group | striptags | trim }}
{% for commit in commits %}
{% if commit.breaking -%}
{% if commit.scope %}_({{ commit.scope }})_ {% endif -%}[**breaking**]: {{ commit.message }} [_({{ commit.id | truncate(length=8, end="") }})_]({{ commit.link }})
{% if commit.body -%}
> {{ commit.body }}
{% endif -%}
{% if commit.breaking_description -%}
> {{ commit.breaking_description }}
{% endif -%}
{% else -%}
- {% if commit.scope %}_({{ commit.scope }})_ {% endif %}{{ commit.message }} [_({{ commit.id | truncate(length=8, end="") }})_]({{ commit.link }})
{% endif -%}
{% endfor %}
{% endfor %}
 "#;

/// Configuration for commit analysis and changelog generation.
#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    /// Tera template string for changelog body format.
    pub body: String,
    /// Optional prefix for package tags.
    pub tag_prefix: Option<String>,
    /// Base URL for release links in changelog.
    pub release_link_base_url: String,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            body: DEFAULT_BODY.into(),
            tag_prefix: None,
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
