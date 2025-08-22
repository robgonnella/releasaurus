use releasaurus_core::changelog::config::DEFAULT_BODY;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
/// Changelog Configuration allowing you to customize changelog output format
pub struct CliChangelogConfig {
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

impl Default for CliChangelogConfig {
    fn default() -> Self {
        Self {
            body: DEFAULT_BODY.to_string(),
            header: None,
            footer: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
/// Package configuration specifying which packages to track as separate
/// releases in this repository
pub struct CliPackageConfig {
    /// The name of the package. This can be an arbitrary name but it's common
    /// for it to match the directory name of the package
    pub name: String,
    /// Path to a valid directory for the package
    pub path: String,
    /// Optional prefix to use for the package
    pub tag_prefix: Option<String>,
}

impl Default for CliPackageConfig {
    fn default() -> Self {
        Self {
            name: "".to_string(),
            path: ".".to_string(),
            tag_prefix: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
/// Complete configuration for the core
pub struct CliConfig {
    /// [`ChangelogConfig`]
    pub changelog: CliChangelogConfig,
    /// [`Vec<PackageConfig>`]
    #[serde(rename = "package")]
    pub packages: Vec<CliPackageConfig>,
    // used to make this an iterator for [`ChangelogConfig`]
    next_pkg: usize,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            changelog: CliChangelogConfig::default(),
            packages: vec![CliPackageConfig::default()],
            next_pkg: 0,
        }
    }
}

/// Represents the config for a single package within a repo
pub struct CliSinglePackageConfig {
    /// The global changelog config shared across all packages in this repo
    pub changelog: CliChangelogConfig,
    /// The specific package config for one package in this repo
    pub package: CliPackageConfig,
}

// Implement iterator on ValidatedCliConfig allowing use to generate
// ChangelogConfig in a loop. This makes it easier to share common parts of the
// config, like changelog format and remote repo config, across all packages
impl Iterator for CliConfig {
    type Item = CliSinglePackageConfig;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.next_pkg;

        if idx >= self.packages.len() {
            return None;
        }

        let package = self.packages[idx].clone();

        self.next_pkg += 1;

        Some(CliSinglePackageConfig {
            changelog: self.changelog.clone(),
            package,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_defaults() {
        let config = CliConfig::default();
        assert!(!config.changelog.body.is_empty())
    }

    #[test]
    fn iterates_to_single_package_config() {
        let config = CliConfig {
            packages: vec![
                CliPackageConfig {
                    name: "1".to_string(),
                    path: "path1".to_string(),
                    tag_prefix: Some("p1".to_string()),
                },
                CliPackageConfig {
                    name: "2".to_string(),
                    path: "path2".to_string(),
                    tag_prefix: Some("p2".to_string()),
                },
            ],
            ..CliConfig::default()
        };
        let mut count = 0;
        for c in config.into_iter() {
            count += 1;
            assert!(!c.changelog.body.is_empty());
            assert_eq!(c.package.name, format!("{count}"));
            assert_eq!(c.package.path, format!("path{count}"));
            assert_eq!(c.package.tag_prefix, Some(format!("p{count}")));
        }
    }
}
