//! Configuration for releasaurus-core
use secrecy::SecretString;
use serde::Deserialize;

pub const GITHUB_DEFAULT_BASE_URL: &str = "https://github.com";
pub const GITLAB_DEFAULT_BASE_URL: &str = "https://gitlab.com";
pub const GITEA_DEFAULT_BASE_URL: &str = "https://gitea.com";

/// The default body value for [`ChangelogConfig`]
const DEFAULT_BODY: &str = r#"{% if version -%}
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

/// Remote Repository configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RemoteConfig {
    /// The owner of the remote repo
    pub owner: String,
    /// The repo path i.e. <group>/<repo>
    pub repo: String,
    /// The access token for the remote repo
    pub token: SecretString,
    /// Optional base_url for the remote
    /// defaults to community version urls
    /// i.e. https://github.com, https://gitlab.com, https://gitea.com
    pub base_url: Option<String>,
}

/// Represents the valid types of remotes
#[derive(Debug, Clone)]
pub enum Remote {
    Github(RemoteConfig),
    Gitlab(RemoteConfig),
    Gitea(RemoteConfig),
}

/// Represents configuration for a single package which includes global
/// config, like changelog and remotes, common to all packages
#[derive(Debug, Clone, Default)]
pub struct SinglePackageConfig {
    /// [`ChangelogConfig`]
    pub changelog: ChangelogConfig,
    /// [`PackageConfig`]
    pub package: PackageConfig,
    ///  The enabled [`Remote`] for this package
    pub remote: Option<Remote>,
}

/// Complete configuration for the core
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// [`ChangelogConfig`]
    pub changelog: ChangelogConfig,
    /// [`Vec<PackageConfig>`]
    #[serde(rename = "package")]
    pub packages: Vec<PackageConfig>,
    /// gitlab [`Option<Remote>`]
    pub gitlab: Option<RemoteConfig>,
    /// github [`Option<Remote>`]
    pub github: Option<RemoteConfig>,
    /// gitea [`Option<Remote>`]
    pub gitea: Option<RemoteConfig>,
    // used to make this an iterator for SinglePackageConfigs
    next_pkg: usize,
}

impl Default for Config {
    fn default() -> Self {
        let pkgs = vec![PackageConfig::default()];
        let chglg = ChangelogConfig::default();

        Self {
            packages: pkgs,
            changelog: chglg,
            github: None,
            gitlab: None,
            gitea: None,
            next_pkg: 0,
        }
    }
}

// Implement iterator on Config allowing use to generate SinglePackageConfig
// in a loop. This makes it easier to share common parts of the config, like
// changelog format and remote repo config, across all packages
impl Iterator for Config {
    type Item = SinglePackageConfig;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.next_pkg;

        if idx >= self.packages.len() {
            return None;
        }

        let mut remote: Option<Remote> = None;

        if let Some(conf) = self.github.clone() {
            let mut remote_config = conf.clone();
            remote_config.base_url = Some(
                remote_config
                    .base_url
                    .unwrap_or(GITHUB_DEFAULT_BASE_URL.to_string()),
            );
            remote = Some(Remote::Github(remote_config));
        } else if let Some(conf) = self.gitlab.clone() {
            let mut remote_config = conf.clone();
            remote_config.base_url = Some(
                remote_config
                    .base_url
                    .unwrap_or(GITLAB_DEFAULT_BASE_URL.to_string()),
            );
            remote = Some(Remote::Gitlab(remote_config));
        } else if let Some(conf) = self.gitea.clone() {
            let mut remote_config = conf.clone();
            remote_config.base_url = Some(
                remote_config
                    .base_url
                    .unwrap_or(GITEA_DEFAULT_BASE_URL.to_string()),
            );
            remote = Some(Remote::Gitea(remote_config));
        }

        self.next_pkg += 1;
        Some(SinglePackageConfig {
            changelog: self.changelog.clone(),
            package: self.packages[idx].clone(),
            remote,
        })
    }
}

#[cfg(test)]
mod tests {
    use secrecy::Secret;

    use super::*;

    #[test]
    fn loads_defaults() {
        let config = Config::default();
        assert!(!config.changelog.body.is_empty())
    }

    #[test]
    fn iterates_to_single_package_config() {
        let config = Config::default();
        for c in config.into_iter() {
            assert!(!c.changelog.body.is_empty());
            assert!(!c.package.path.is_empty());
            assert!(c.remote.is_none());
        }
    }

    #[test]
    fn defaults_remote_host_for_github() {
        let config = Config {
            github: Some(RemoteConfig {
                base_url: None,
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                token: Secret::new("secret".to_string()),
            }),
            ..Config::default()
        };

        for c in config {
            assert!(!c.changelog.body.is_empty());
            assert!(!c.package.path.is_empty());
            let remote = c.remote.unwrap();
            assert!(matches!(remote, Remote::Github(_)));
            if let Remote::Github(conf) = remote {
                assert_eq!(
                    conf.base_url,
                    Some(GITHUB_DEFAULT_BASE_URL.to_string())
                );
            }
        }
    }

    #[test]
    fn defaults_remote_host_for_gitlab() {
        let config = Config {
            gitlab: Some(RemoteConfig {
                base_url: None,
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                token: Secret::new("secret".to_string()),
            }),
            ..Config::default()
        };

        for c in config {
            assert!(!c.changelog.body.is_empty());
            assert!(!c.package.path.is_empty());
            let remote = c.remote.unwrap();
            assert!(matches!(remote, Remote::Gitlab(_)));
            if let Remote::Gitlab(conf) = remote {
                assert_eq!(
                    conf.base_url,
                    Some(GITLAB_DEFAULT_BASE_URL.to_string())
                );
            }
        }
    }

    #[test]
    fn does_not_default_remote_host_for_gitea() {
        let config = Config {
            gitea: Some(RemoteConfig {
                base_url: None,
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                token: Secret::new("secret".to_string()),
            }),
            ..Config::default()
        };

        for c in config {
            assert!(!c.changelog.body.is_empty());
            assert!(!c.package.path.is_empty());
            let remote = c.remote.unwrap();
            assert!(matches!(remote, Remote::Gitea(_)));
            if let Remote::Gitea(conf) = remote {
                assert_eq!(
                    conf.base_url,
                    Some(GITEA_DEFAULT_BASE_URL.to_string())
                );
            }
        }
    }
}
