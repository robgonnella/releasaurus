use derive_builder::Builder;
use merge::Merge;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const DEFAULT_INCLUDE_AUTHOR: bool = false;
pub const DEFAULT_AGGREGATE_PRERELEASES: bool = false;
pub const DEFAULT_INCLUDE_PR_LINK: bool = false;

/// Default changelog body template.
pub const DEFAULT_BODY: &str = r#"# [{{ version  }}]{% if tag_compare_link %}({{ tag_compare_link }}){% else %}({{ link }}){% endif %} - {{ timestamp | date(format="%Y-%m-%d") }}
{% for group, commits in commits | filter(attribute="merge_commit", value=false) | sort(attribute="group") | group_by(attribute="group") %}
### {{ group | striptags | trim }}
{% for commit in commits %}
{% if commit.breaking -%}
{% if commit.scope %}_({{ commit.scope }})_ {% endif -%}[**breaking**]: {{ commit.title }} [_({{ commit.short_id }})_]({{ commit.link }}){% if include_author %} ({{ commit.author_name }}){% endif %}{% if include_pr_link and commit.pr %} ([PR {{ commit.pr.id }}]({{ commit.pr.link }})){% endif %}
{% if commit.body -%}
> {{ commit.body }}
{% endif -%}
{% if commit.breaking_description -%}
> {{ commit.breaking_description }}
{% endif -%}
{% else -%}
- {% if commit.scope %}_({{ commit.scope }})_ {% endif %}{{ commit.title }} [_({{ commit.short_id }})_]({{ commit.link }}){% if include_author %} ({{ commit.author_name }}){% endif %}{% if include_pr_link and commit.pr %} ([PR {{ commit.pr.id }}]({{ commit.pr.link }})){% endif %}
{% endif -%}
{% endfor %}
{% endfor %}
 "#;

fn default_body() -> String {
    DEFAULT_BODY.into()
}

fn default_include_author() -> bool {
    DEFAULT_INCLUDE_AUTHOR
}

fn default_aggregate_prereleases() -> bool {
    DEFAULT_AGGREGATE_PRERELEASES
}

fn default_include_pr_link() -> bool {
    DEFAULT_INCLUDE_PR_LINK
}

/// Changelog configuration (applies to all packages)
#[derive(
    Debug, Clone, Default, Serialize, Deserialize, JsonSchema, Builder, Merge,
)]
#[builder(setter(into, strip_option), default)]
#[serde(default, deny_unknown_fields)] // Use default for missing fields
pub struct ChangelogConfig {
    /// Main changelog body template.
    #[merge(strategy = merge::option::overwrite_none)]
    #[schemars(default = "default_body")]
    pub body: Option<String>,
    /// Includes commit author name in default body template
    #[merge(strategy = merge::option::overwrite_none)]
    #[schemars(default = "default_include_author")]
    pub include_author: Option<bool>,
    /// Includes related PR links for each commit if they exist
    #[merge(strategy = merge::option::overwrite_none)]
    #[schemars(default = "default_include_pr_link")]
    pub include_pr_link: Option<bool>,
    /// Aggregates changelogs from prior prereleases when graduating
    #[merge(strategy = merge::option::overwrite_none)]
    #[schemars(default = "default_aggregate_prereleases")]
    pub aggregate_prereleases: Option<bool>,
}
