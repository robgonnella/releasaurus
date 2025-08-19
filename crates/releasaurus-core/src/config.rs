//! Configuration for releasaurus-core
use secrecy::SecretString;
use serde::Deserialize;

/// The default body value for [`ChangelogConfig`]
const DEFAULT_BODY: &str = r#"{% if version -%}
    # [{{ version | trim_start_matches(pat="v") }}] - {{ timestamp | date(format="%Y-%m-%d") }}
{% else -%}
    # [unreleased]
{% endif -%}
{% for group, commits in commits | group_by(attribute="group") %}
    ### {{ group | striptags | trim | upper_first }}
    {% for commit in commits %}
      {% if commit.breaking -%}
        {% if commit.scope %}_({{ commit.scope }})_ {% endif -%}[**breaking**]: {{ commit.message | upper_first }}
        > {{ commit.body }}
        > {{ commit.breaking_description }}
      {% else -%}
        - {% if commit.scope %}_({{ commit.scope }})_ {% endif %}{{ commit.message | upper_first }}
      {% endif -%}
    {% endfor -%}
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
pub struct Remote {
    /// The owner of the remote repo
    pub owner: String,
    /// The repo path i.e. <group>/<repo>
    pub repo: String,
    /// The access token for the remote repo
    pub token: SecretString,
}

/// Represents configuration for a single package which includes global
/// config, like changelog and remotes, common to all packages
#[derive(Debug, Clone, Default)]
pub struct SinglePackageConfig {
    /// [`ChangelogConfig`]
    pub changelog: ChangelogConfig,
    /// [`PackageConfig`]
    pub package: PackageConfig,
    /// gitlab [`Option<Remote>`]
    pub gitlab: Option<Remote>,
    /// github [`Option<Remote>`]
    pub github: Option<Remote>,
    /// bitbucket [`Option<Remote>`]
    pub bitbucket: Option<Remote>,
    /// gitea [`Option<Remote>`]
    pub gitea: Option<Remote>,
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
    pub gitlab: Option<Remote>,
    /// github [`Option<Remote>`]
    pub github: Option<Remote>,
    /// bitbucket [`Option<Remote>`]
    pub bitbucket: Option<Remote>,
    /// gitea [`Option<Remote>`]
    pub gitea: Option<Remote>,
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
            bitbucket: None,
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

        self.next_pkg += 1;
        Some(SinglePackageConfig {
            changelog: self.changelog.clone(),
            package: self.packages[idx].clone(),
            github: self.github.clone(),
            gitlab: self.gitlab.clone(),
            gitea: self.gitea.clone(),
            bitbucket: self.bitbucket.clone(),
        })
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
