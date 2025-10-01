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
