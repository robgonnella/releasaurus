//! UpdaterManager - High-level entry point for language-agnostic package updates
use color_eyre::eyre::{ContextCompat, WrapErr};
use log::{debug, info, warn};
use std::collections::HashMap;
use std::path::Path;

use crate::{
    analyzer::types::Output,
    config::CliConfig,
    result::Result,
    updater::{
        detection::manager::DetectionManager,
        framework::{Framework, Package},
    },
};

/// Statistics about the update operation
#[derive(Debug, Default, Clone)]
pub struct UpdateStats {
    /// Total number of packages processed
    pub total_packages: usize,
    /// Number of packages that were releasable (had version updates)
    pub releasable_packages: usize,
    /// Number of packages successfully updated
    pub updated_packages: usize,
    /// Frameworks detected and their package counts
    pub frameworks_detected: HashMap<String, usize>,
    /// Any warnings encountered during processing
    pub warnings: Vec<String>,
}

impl std::fmt::Display for UpdateStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Update Statistics:")?;
        writeln!(f, "  Total packages: {}", self.total_packages)?;
        writeln!(f, "  Releasable packages: {}", self.releasable_packages)?;
        writeln!(f, "  Successfully updated: {}", self.updated_packages)?;

        if !self.frameworks_detected.is_empty() {
            writeln!(f, "  Frameworks detected:")?;
            for (framework, count) in &self.frameworks_detected {
                writeln!(f, "    {}: {} packages", framework, count)?;
            }
        }

        if !self.warnings.is_empty() {
            writeln!(f, "  Warnings:")?;
            for warning in &self.warnings {
                writeln!(f, "    - {}", warning)?;
            }
        }

        Ok(())
    }
}

/// High-level manager for coordinating package updates across different languages/frameworks
pub struct UpdaterManager {
    /// Root path of the repository
    root_path: std::path::PathBuf,
    /// Detects frameworks for packages
    detection_manager: DetectionManager,
}

impl UpdaterManager {
    /// Create a new updater manager for the given repository
    pub fn new<P: AsRef<Path>>(root_path: P) -> Self {
        let root_path = root_path.as_ref().to_path_buf();

        Self {
            root_path: root_path.clone(),
            detection_manager: Framework::detection_manager(root_path),
        }
    }

    /// Update packages based on analyzer output and configuration
    ///
    /// This is the main entry point for package updates. It:
    /// 1. Converts analyzer output to Package structs
    /// 2. Auto-detects frameworks for each package
    /// 3. Creates appropriate updaters from the detected framework
    /// 4. Executes the updates
    pub fn update_packages(
        &mut self,
        manifest: &HashMap<String, Output>,
        cli_config: &CliConfig,
    ) -> Result<UpdateStats> {
        info!(
            "Starting package updates for {} manifest entries",
            manifest.len()
        );

        // Convert analyzer output to packages
        let packages = self
            .convert_manifest_to_packages(manifest, cli_config)
            .context("Failed to convert manifest to packages")?;

        if packages.is_empty() {
            info!("No releasable packages found");
            return Ok(UpdateStats {
                total_packages: manifest.len(),
                releasable_packages: 0,
                updated_packages: 0,
                frameworks_detected: HashMap::new(),
                warnings: vec!["No packages were releasable".to_string()],
            });
        }

        // Collect statistics
        let mut stats = self.create_initial_stats(&packages, manifest.len());

        info!("Executing updates for {} packages", packages.len());

        for package in packages.iter() {
            let updater = package.framework.updater();
            match updater.update(&self.root_path, packages.clone()) {
                Ok(()) => {
                    stats.updated_packages += 1;
                    info!(
                        "Successfully updated {} packages",
                        stats.updated_packages
                    );
                }
                Err(err) => {
                    let error_msg =
                        format!("Failed to update packages: {}", err);
                    warn!("{}", error_msg);
                    stats.warnings.push(error_msg);
                    return Err(err.wrap_err("Package update failed"));
                }
            }
        }

        Ok(stats)
    }

    // Private helper methods
    fn convert_manifest_to_packages(
        &self,
        manifest: &HashMap<String, Output>,
        cli_config: &CliConfig,
    ) -> Result<Vec<Package>> {
        let mut packages = Vec::new();

        for (package_path, output) in manifest {
            // Only create Package if there's a next version (i.e., the package is releasable)
            if let Some(next_version) = &output.next_version {
                // Validate that we have a corresponding package config
                cli_config
                    .packages
                    .iter()
                    .find(|p| &p.path == package_path)
                    .context(format!(
                        "Could not find package config for path: {}",
                        package_path
                    ))?;

                let detection =
                    self.detection_manager.detect_framework(package_path)?;

                let package_name = self.derive_package_name(package_path);
                let package_path =
                    self.root_path.join(package_path).display().to_string();

                let package = Package::new(
                    package_name.clone(),
                    package_path.clone(),
                    next_version.clone(),
                    detection.framework, // Will be detected later
                );

                debug!(
                    "Prepared package '{}' at path '{}' for version update: {}",
                    package_name, package_path, next_version.semver,
                );

                packages.push(package);
            } else {
                debug!(
                    "Skipping package '{}' - no next version determined",
                    package_path
                );
            }
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
            self.root_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("root")
                .to_string()
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
        let mut frameworks_detected = HashMap::new();

        // Count packages by framework
        for package in packages {
            let framework_name = package.framework_name().to_string();
            *frameworks_detected
                .entry(framework_name.clone())
                .or_insert(0) += 1;
        }

        UpdateStats {
            total_packages: total_count,
            releasable_packages: packages.len(),
            updated_packages: 0, // Will be set after actual updates
            frameworks_detected,
            warnings: vec![],
        }
    }
}
