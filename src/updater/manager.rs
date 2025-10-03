//! UpdaterManager - High-level entry point for language-agnostic package updates
use color_eyre::eyre::{ContextCompat, WrapErr};
use log::{debug, info, warn};
use std::collections::HashMap;
use std::path::Path;

use crate::{
    analyzer::release::Release,
    config::{Config, ReleaseType},
    forge::{request::FileChange, traits::FileLoader},
    result::Result,
    updater::framework::{Framework, Package},
};

/// Update operation statistics.
#[derive(Debug, Default, Clone)]
pub struct UpdateStats {
    /// Total packages processed.
    pub total_packages: usize,
    /// Packages that had version updates.
    pub releasable_packages: usize,
    /// Packages successfully updated.
    pub updated_packages: usize,
}

impl std::fmt::Display for UpdateStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Update Statistics:")?;
        writeln!(f, "  Total packages: {}", self.total_packages)?;
        writeln!(f, "  Releasable packages: {}", self.releasable_packages)?;
        writeln!(f, "  Successfully updated: {}", self.updated_packages)?;
        Ok(())
    }
}

/// Coordinates package updates across different languages and frameworks.
pub struct UpdaterManager {
    repo_name: String,
}

impl UpdaterManager {
    /// Create updater manager for repository.
    pub fn new(repo_name: &str) -> Self {
        Self {
            repo_name: repo_name.to_string(),
        }
    }

    /// Update packages based on analyzer output and configuration.
    pub async fn update_packages(
        &mut self,
        manifest: &HashMap<String, Release>,
        config: &Config,
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        info!(
            "Starting package updates for {} manifest entries",
            manifest.len()
        );

        // Convert analyzer output to packages
        let packages = self
            .convert_manifest_to_packages(manifest, config)
            .context("Failed to convert manifest to packages")?;

        if packages.is_empty() {
            info!("No releasable packages found");
            let stats = UpdateStats {
                total_packages: manifest.len(),
                releasable_packages: 0,
                updated_packages: 0,
            };
            info!("{stats}");
            return Ok(None);
        }

        // Collect statistics
        let mut stats = self.create_initial_stats(&packages, manifest.len());

        info!("Executing updates for {} packages", packages.len());

        let mut file_changes: Vec<FileChange> = vec![];

        for package in packages.iter() {
            let updater = package.framework.updater();
            let updates = updater.update(packages.clone(), loader).await?;
            if let Some(updates) = updates {
                file_changes.extend(updates);
                stats.updated_packages += 1;
            }
        }

        info!("{stats}");

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    // Private helper methods
    fn convert_manifest_to_packages(
        &self,
        manifest: &HashMap<String, Release>,
        config: &Config,
    ) -> Result<Vec<Package>> {
        let mut packages = Vec::new();

        for (package_path, release) in manifest {
            // Only create Package if there's a next version (i.e., the package is releasable)
            if release.tag.is_none() {
                warn!(
                    "failed to find projected tag for release: {:#?}",
                    release
                );
                continue;
            }

            let tag = release.tag.clone().unwrap();

            // Validate that we have a corresponding package config
            let config_pkg = config
                .packages
                .iter()
                .find(|p| &p.path == package_path)
                .context(format!(
                    "Could not find package config for path: {}",
                    package_path
                ))?;

            let release_type = config_pkg
                .release_type
                .clone()
                .unwrap_or(ReleaseType::default());

            let framework = Framework::from(release_type);

            let package_name = self.derive_package_name(package_path);

            let package = Package::new(
                package_name.clone(),
                package_path.clone(),
                tag.clone(),
                framework, // Will be detected later
            );

            debug!(
                "Prepared package '{}' at path '{}' for version update: {}",
                package_name, package_path, tag.semver,
            );

            packages.push(package);
        }

        info!(
            "Converted {} releasable packages from manifest",
            packages.len()
        );
        Ok(packages)
    }

    fn derive_package_name(&self, package_path: &str) -> String {
        let path = Path::new(package_path);

        if let Some(name) = path.file_name() {
            return name.display().to_string();
        }

        if package_path == "." {
            // For root package, use repository directory name as fallback
            self.repo_name.clone()
        } else {
            // Extract name from path (e.g., "crates/my-package" -> "my-package")
            package_path
                .split('/')
                .next_back()
                .unwrap_or(package_path)
                .to_string()
        }
    }

    fn create_initial_stats(
        &self,
        packages: &[Package],
        total_count: usize,
    ) -> UpdateStats {
        UpdateStats {
            total_packages: total_count,
            releasable_packages: packages.len(),
            updated_packages: 0, // Will be set after actual updates
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::release::Tag;
    use crate::config::PackageConfig;
    use crate::forge::traits::MockFileLoader;
    use semver::Version as SemVer;

    fn create_test_release(version: &str, has_tag: bool) -> Release {
        Release {
            tag: if has_tag {
                Some(Tag {
                    sha: "test-sha".to_string(),
                    name: format!("v{}", version),
                    semver: SemVer::parse(version).unwrap(),
                })
            } else {
                None
            },
            link: String::new(),
            sha: "test-sha".to_string(),
            commits: vec![],
            notes: String::new(),
            timestamp: 0,
        }
    }

    fn create_test_config(packages: Vec<(&str, ReleaseType)>) -> Config {
        Config {
            packages: packages
                .into_iter()
                .map(|(path, release_type)| PackageConfig {
                    path: path.to_string(),
                    release_type: Some(release_type),
                    tag_prefix: None,
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn test_new_updater_manager() {
        let manager = UpdaterManager::new("test-repo");
        assert_eq!(manager.repo_name, "test-repo");
    }

    #[test]
    fn test_update_stats_display() {
        let stats = UpdateStats {
            total_packages: 10,
            releasable_packages: 8,
            updated_packages: 5,
        };

        let display = format!("{}", stats);
        assert!(display.contains("Total packages: 10"));
        assert!(display.contains("Releasable packages: 8"));
        assert!(display.contains("Successfully updated: 5"));
    }

    #[test]
    fn test_update_stats_default() {
        let stats = UpdateStats::default();
        assert_eq!(stats.total_packages, 0);
        assert_eq!(stats.releasable_packages, 0);
        assert_eq!(stats.updated_packages, 0);
    }

    #[test]
    fn test_derive_package_name_from_directory() {
        let manager = UpdaterManager::new("test-repo");

        // Test with simple directory name
        let name = manager.derive_package_name("packages/my-package");
        assert_eq!(name, "my-package");

        // Test with nested path
        let name = manager.derive_package_name("crates/core/utils");
        assert_eq!(name, "utils");

        // Test with root path
        let name = manager.derive_package_name(".");
        assert_eq!(name, "test-repo");

        // Test with single directory
        let name = manager.derive_package_name("backend");
        assert_eq!(name, "backend");
    }

    #[test]
    fn test_convert_manifest_to_packages() {
        let manager = UpdaterManager::new("test-repo");

        let mut manifest = HashMap::new();
        manifest.insert(
            "packages/one".to_string(),
            create_test_release("1.0.0", true),
        );
        manifest.insert(
            "packages/two".to_string(),
            create_test_release("2.0.0", true),
        );

        let config = create_test_config(vec![
            ("packages/one", ReleaseType::Node),
            ("packages/two", ReleaseType::Rust),
        ]);

        let result = manager.convert_manifest_to_packages(&manifest, &config);
        assert!(result.is_ok());

        let packages = result.unwrap();
        assert_eq!(packages.len(), 2);

        // Verify package properties
        assert!(
            packages
                .iter()
                .any(|p| p.name == "one" && p.path == "packages/one")
        );
        assert!(
            packages
                .iter()
                .any(|p| p.name == "two" && p.path == "packages/two")
        );

        assert!(
            packages
                .iter()
                .any(|p| p.name == "one"
                    && matches!(p.framework, Framework::Node))
        );

        assert!(
            packages
                .iter()
                .any(|p| p.name == "two"
                    && matches!(p.framework, Framework::Rust))
        );
    }

    #[test]
    fn test_convert_manifest_skips_packages_without_tags() {
        let manager = UpdaterManager::new("test-repo");

        let mut manifest = HashMap::new();
        manifest.insert(
            "packages/one".to_string(),
            create_test_release("1.0.0", true),
        );
        manifest.insert(
            "packages/two".to_string(),
            create_test_release("2.0.0", false), // No tag
        );

        let config = create_test_config(vec![
            ("packages/one", ReleaseType::Node),
            ("packages/two", ReleaseType::Rust),
        ]);

        let result = manager.convert_manifest_to_packages(&manifest, &config);
        assert!(result.is_ok());

        let packages = result.unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "one");
    }

    #[test]
    fn test_convert_manifest_error_when_config_missing() {
        let manager = UpdaterManager::new("test-repo");

        let mut manifest = HashMap::new();
        manifest.insert(
            "packages/one".to_string(),
            create_test_release("1.0.0", true),
        );

        // Config doesn't include packages/one
        let config =
            create_test_config(vec![("packages/different", ReleaseType::Node)]);

        let result = manager.convert_manifest_to_packages(&manifest, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_initial_stats() {
        let manager = UpdaterManager::new("test-repo");

        let packages = vec![
            Package {
                name: "pkg1".to_string(),
                path: "packages/pkg1".to_string(),
                framework: Framework::Node,
                next_version: Tag {
                    sha: "sha1".to_string(),
                    name: "v1.0.0".to_string(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                },
            },
            Package {
                name: "pkg2".to_string(),
                path: "packages/pkg2".to_string(),
                framework: Framework::Rust,
                next_version: Tag {
                    sha: "sha2".to_string(),
                    name: "v2.0.0".to_string(),
                    semver: SemVer::parse("2.0.0").unwrap(),
                },
            },
        ];

        let stats = manager.create_initial_stats(&packages, 5);

        assert_eq!(stats.total_packages, 5);
        assert_eq!(stats.releasable_packages, 2);
        assert_eq!(stats.updated_packages, 0);
    }

    #[tokio::test]
    async fn test_update_packages_with_no_releasable_packages() {
        let mut manager = UpdaterManager::new("test-repo");

        let mut manifest = HashMap::new();
        manifest.insert(
            "packages/one".to_string(),
            create_test_release("1.0.0", false), // No tag
        );

        let config =
            create_test_config(vec![("packages/one", ReleaseType::Node)]);

        let mock_loader = MockFileLoader::new();

        let result = manager
            .update_packages(&manifest, &config, &mock_loader)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_update_packages_with_empty_manifest() {
        let mut manager = UpdaterManager::new("test-repo");

        let manifest = HashMap::new();
        let config = Config::default();
        let mock_loader = MockFileLoader::new();

        let result = manager
            .update_packages(&manifest, &config, &mock_loader)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
