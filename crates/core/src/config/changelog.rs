use derive_builder::Builder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Default changelog body template.
pub const DEFAULT_BODY: &str = r#"# [{{ version  }}]{% if tag_compare_link %}({{ tag_compare_link }}){% else %}({{ link }}){% endif %} - {{ timestamp | date(format="%Y-%m-%d") }}
{% for group, commits in commits | filter(attribute="merge_commit", value=false) | sort(attribute="group") | group_by(attribute="group") %}
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

/// Rewords messages in changelog for targeted commit shas
#[derive(
    Debug, Clone, Default, JsonSchema, Serialize, Deserialize, Builder,
)]
#[builder(setter(into))]
pub struct RewordedCommit {
    /// Sha (or prefix) of the commit to reword. Matches any commit whose SHA
    /// starts with this value
    pub sha: String,
    /// The new message to display in changelog
    pub message: String,
}

/// Changelog configuration (applies to all packages)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Builder)]
#[builder(setter(into, strip_option), default)]
#[serde(default)] // Use default for missing fields
pub struct ChangelogConfig {
    /// Main changelog body template.
    pub body: String,
    /// Skips including ci commits in changelog
    pub skip_ci: bool,
    /// Skips including chore commits in changelog
    pub skip_chore: bool,
    /// Skips including doc commits in changelog
    pub skip_doc: bool,
    /// Skips including test commits in changelog
    pub skip_test: bool,
    /// Skips including style commits in changelog
    pub skip_style: bool,
    /// Skips including refactor commits in changelog
    pub skip_refactor: bool,
    /// Skips including perf commits in changelog
    pub skip_perf: bool,
    /// Skips including revert commits in changelog
    pub skip_revert: bool,
    /// Skips including miscellaneous commits in changelog
    pub skip_miscellaneous: bool,
    /// Skips including merge commits in changelog
    pub skip_merge_commits: bool,
    /// Skips including release commits in changelog
    pub skip_release_commits: bool,
    /// Skips targeted commit shas (or prefixes) when generating next version
    /// and changelog. Each value matches any commit whose SHA starts with the
    /// provided value
    pub skip_shas: Option<Vec<String>>,
    /// Rewords commit messages for targeted shas when generated changelog.
    /// Each SHA can be a prefix - matches any commit whose SHA starts with the
    /// provided value
    pub reword: Option<Vec<RewordedCommit>>,
    /// Includes commit author name in default body template
    pub include_author: bool,
    /// Aggregates changelogs from prior prereleases when graduating
    pub aggregate_prereleases: bool,
}

impl Default for ChangelogConfig {
    fn default() -> Self {
        Self {
            body: DEFAULT_BODY.into(),
            skip_ci: false,
            skip_chore: false,
            skip_doc: false,
            skip_test: false,
            skip_style: false,
            skip_refactor: false,
            skip_perf: false,
            skip_revert: false,
            skip_miscellaneous: false,
            skip_merge_commits: true,
            skip_release_commits: true,
            skip_shas: None,
            reword: None,
            include_author: false,
            aggregate_prereleases: false,
        }
    }
}
