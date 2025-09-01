use color_eyre::eyre::{Context, Result};
use log::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::updater::framework::Framework;
use crate::updater::framework::{Package, PackageMetadata};
use crate::updater::node::types::{DependencyType, LocalDependency};
use crate::updater::traits::PackageUpdater;

/// Node.js package updater supporting npm, yarn, and pnpm
pub struct NodeUpdater {
    /// Root path of the repository
    root_path: PathBuf,
}

/// Represents a package.json file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageJson {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspaces: Option<WorkspaceConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "devDependencies")]
    pub dev_dependencies: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "peerDependencies")]
    pub peer_dependencies: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "optionalDependencies")]
    pub optional_dependencies: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scripts: Option<HashMap<String, String>>,
    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

/// Workspace configuration in package.json
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WorkspaceConfig {
    /// Simple array of workspace patterns
    Array(Vec<String>),
    /// Object with packages and other config
    Object {
        packages: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        nohoist: Option<Vec<String>>,
        #[serde(flatten)]
        other: HashMap<String, serde_json::Value>,
    },
}

/// Lerna configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LernaJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packages: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "npmClient")]
    pub npm_client: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "useWorkspaces")]
    pub use_workspaces: Option<bool>,
    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

/// Nx workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NxJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projects: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "npmScope")]
    pub npm_scope: Option<String>,
    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

impl NodeUpdater {
    pub fn new<P: AsRef<Path>>(root_path: P) -> Self {
        Self {
            root_path: root_path.as_ref().to_path_buf(),
        }
    }

    /// Analyze Node.js packages to extract metadata
    pub fn analyze_packages(&self, packages: &mut [Package]) -> Result<()> {
        for package in packages.iter_mut() {
            if let Framework::Node(_) = package.framework {
                self.analyze_single_package(package).with_context(|| {
                    format!("Failed to analyze package at {}", package.path)
                })?;
            }
        }
        Ok(())
    }

    fn analyze_single_package(&self, package: &mut Package) -> Result<()> {
        let manifest_path = self.root_path.join(&package.manifest_path);
        let content =
            fs::read_to_string(&manifest_path).with_context(|| {
                format!("Failed to read {}", manifest_path.display())
            })?;

        let package_json: PackageJson = serde_json::from_str(&content)
            .with_context(|| {
                format!(
                    "Failed to parse package.json at {}",
                    manifest_path.display()
                )
            })?;

        // Update package name from manifest if not set
        if package.name.is_empty() {
            package.name = package_json.name.clone();
        }

        // Determine workspace status
        let (is_workspace_root, _workspace_members) =
            self.determine_workspace_status(&manifest_path)?;

        // Extract local dependencies
        let local_dependencies =
            self.extract_local_dependencies(&package_json)?;

        // Detect package manager
        let _detected_manager = self.detect_package_manager(&manifest_path)?;

        // Update package metadata
        if let PackageMetadata::Node(ref mut node_meta) = package.metadata {
            node_meta.is_monorepo_root = is_workspace_root;
            node_meta.local_dependencies = local_dependencies;
            // Note: NodePackageMetadata doesn't have workspace_members or
            // package_manager fields
        }

        Ok(())
    }

    fn determine_workspace_status(
        &self,
        manifest_path: &Path,
    ) -> Result<(bool, Vec<String>)> {
        let content = fs::read_to_string(manifest_path)?;
        let package_json: PackageJson = serde_json::from_str(&content)?;

        if let Some(workspaces) = package_json.workspaces {
            let workspace_patterns = match workspaces {
                WorkspaceConfig::Array(patterns) => patterns,
                WorkspaceConfig::Object { packages, .. } => packages,
            };

            let members = self.get_workspace_members(
                &workspace_patterns,
                manifest_path.parent().unwrap(),
            )?;
            Ok((true, members))
        } else {
            // Check for Lerna configuration
            let lerna_path = manifest_path.parent().unwrap().join("lerna.json");
            if lerna_path.exists() {
                let lerna_content = fs::read_to_string(&lerna_path)?;
                let lerna_json: LernaJson =
                    serde_json::from_str(&lerna_content)?;
                if let Some(packages) = lerna_json.packages {
                    let members = self.get_workspace_members(
                        &packages,
                        manifest_path.parent().unwrap(),
                    )?;
                    return Ok((true, members));
                }
            }

            Ok((false, Vec::new()))
        }
    }

    fn get_workspace_members(
        &self,
        patterns: &[String],
        base_dir: &Path,
    ) -> Result<Vec<String>> {
        let mut members = Vec::new();

        for pattern in patterns {
            let expanded = self.expand_glob_pattern(pattern, base_dir)?;
            members.extend(expanded);
        }

        Ok(members)
    }

    fn expand_glob_pattern(
        &self,
        pattern: &str,
        base_dir: &Path,
    ) -> Result<Vec<String>> {
        let mut results = Vec::new();
        let full_pattern = base_dir.join(pattern);

        // Simple glob expansion - in a real implementation you'd use the glob crate
        if pattern.ends_with('*') {
            let parent = full_pattern.parent().unwrap_or(base_dir);
            if parent.exists() {
                for entry in fs::read_dir(parent)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_dir() {
                        let package_json = path.join("package.json");
                        if package_json.exists()
                            && let Some(relative_path) =
                                path.strip_prefix(&self.root_path).ok()
                        {
                            results.push(
                                relative_path.to_string_lossy().to_string(),
                            );
                        }
                    }
                }
            }
        } else if full_pattern.join("package.json").exists()
            && let Some(relative_path) =
                full_pattern.strip_prefix(&self.root_path).ok()
        {
            results.push(relative_path.to_string_lossy().to_string());
        }

        Ok(results)
    }

    fn extract_local_dependencies(
        &self,
        package_json: &PackageJson,
    ) -> Result<Vec<LocalDependency>> {
        let mut local_deps = Vec::new();

        // Check all dependency types
        let dep_maps = [
            &package_json.dependencies,
            &package_json.dev_dependencies,
            &package_json.peer_dependencies,
            &package_json.optional_dependencies,
        ];

        for dep_map in dep_maps.into_iter().flatten() {
            for (name, version_spec) in dep_map {
                if self.is_local_dependency(version_spec)? {
                    local_deps.push(LocalDependency {
                        name: name.clone(),
                        current_version_req: version_spec.clone(),
                        new_version_req: version_spec.clone(), // Will be updated later
                        dependency_type: DependencyType::Runtime,
                    });
                }
            }
        }

        Ok(local_deps)
    }

    fn is_local_dependency(&self, version_spec: &str) -> Result<bool> {
        // Check for common local dependency patterns
        Ok(version_spec.starts_with("file:")
            || version_spec.starts_with("link:")
            || version_spec.starts_with("workspace:")
            || version_spec == "*"
            || (version_spec.starts_with("^")
                && self.is_workspace_version(version_spec))
            || (version_spec.starts_with("~")
                && self.is_workspace_version(version_spec))
            || self.is_workspace_version(version_spec))
    }

    fn is_workspace_version(&self, version_spec: &str) -> bool {
        // Simple heuristic: if it's a valid semver and we're in a workspace
        // context This would need more sophisticated logic in a real
        // implementation
        version_spec
            .chars()
            .next()
            .is_none_or(|c| c.is_ascii_digit())
    }

    fn detect_package_manager(&self, manifest_path: &Path) -> Result<String> {
        let dir = manifest_path.parent().unwrap();

        // Check for lockfiles
        if dir.join("pnpm-lock.yaml").exists() {
            return Ok("pnpm".to_string());
        }
        if dir.join("yarn.lock").exists() {
            return Ok("yarn".to_string());
        }
        if dir.join("package-lock.json").exists() {
            return Ok("npm".to_string());
        }

        // Default to npm
        Ok("npm".to_string())
    }

    /// Update local dependencies with new versions
    fn update_local_dependencies(
        &self,
        packages: &mut [Package],
    ) -> Result<()> {
        // Create a map of package names to their new versions
        let mut version_map = HashMap::new();

        // First, collect all package names and versions
        for package in packages.iter() {
            if let Framework::Node(_) = package.framework {
                // Read the package.json to get the actual package name
                let manifest_path = self.root_path.join(&package.manifest_path);
                if let Ok(content) = fs::read_to_string(&manifest_path)
                    && let Ok(package_json) =
                        serde_json::from_str::<PackageJson>(&content)
                {
                    version_map.insert(
                        package_json.name.clone(),
                        package.next_version.clone(),
                    );
                }
            }
        }

        // Update local dependency version requirements
        for package in packages.iter_mut() {
            if let PackageMetadata::Node(ref mut node_meta) = package.metadata {
                for local_dep in &mut node_meta.local_dependencies {
                    if let Some(new_version) = version_map.get(&local_dep.name)
                    {
                        // Update version requirement (keeping the specifier style)
                        local_dep.new_version_req = self
                            .update_version_specifier(
                                &local_dep.current_version_req,
                                new_version.semver.to_string().as_str(),
                            );
                    }
                }
            }
        }

        Ok(())
    }

    fn update_version_specifier(
        &self,
        current_spec: &str,
        new_version: &str,
    ) -> String {
        if current_spec.starts_with("^") {
            format!("^{}", new_version)
        } else if current_spec.starts_with("~") {
            format!("~{}", new_version)
        } else if current_spec.starts_with(">=") {
            format!(">={}", new_version)
        } else if current_spec.starts_with("<=") {
            format!("<={}", new_version)
        } else if current_spec.starts_with(">") {
            format!(">{}", new_version)
        } else if current_spec.starts_with("<") {
            format!("<{}", new_version)
        } else if current_spec.starts_with("workspace:") {
            format!("workspace:{}", new_version)
        } else if current_spec == "*" {
            "*".to_string()
        } else {
            new_version.to_string()
        }
    }

    fn update_package_json(&self, package: &Package) -> Result<()> {
        let manifest_path = self.root_path.join(&package.manifest_path);
        let content = fs::read_to_string(&manifest_path)?;
        let mut package_json: PackageJson = serde_json::from_str(&content)?;

        // Update the version
        package_json.version = package.next_version.semver.to_string();

        // Update local dependencies if this package has them
        if let PackageMetadata::Node(ref node_meta) = package.metadata {
            self.update_dependencies_in_package_json(
                &mut package_json,
                &node_meta.local_dependencies,
            )?;
        }

        // Write back the updated package.json
        let updated_content = serde_json::to_string_pretty(&package_json)?;
        fs::write(&manifest_path, updated_content)?;

        info!(
            "Updated {} to version {}",
            manifest_path.display(),
            package.next_version.semver
        );
        Ok(())
    }

    fn update_dependencies_in_package_json(
        &self,
        package_json: &mut PackageJson,
        local_deps: &[LocalDependency],
    ) -> Result<()> {
        let dep_maps = [
            &mut package_json.dependencies,
            &mut package_json.dev_dependencies,
            &mut package_json.peer_dependencies,
            &mut package_json.optional_dependencies,
        ];

        for dep_map in dep_maps.into_iter().flatten() {
            for local_dep in local_deps {
                if let Some(current_version) = dep_map.get_mut(&local_dep.name)
                {
                    *current_version = local_dep.new_version_req.clone();
                }
            }
        }

        Ok(())
    }
}

impl PackageUpdater for NodeUpdater {
    fn update(&self, packages: Vec<Package>) -> Result<()> {
        info!(
            "Starting Node.js package updates for {} packages",
            packages.len()
        );

        // Filter to only Node.js packages
        let mut node_packages: Vec<Package> = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Node(_)))
            .collect();

        if node_packages.is_empty() {
            info!("No Node.js packages to update");
            return Ok(());
        }

        // Analyze packages to extract metadata
        self.analyze_packages(&mut node_packages)?;

        // Update local dependencies with new versions
        self.update_local_dependencies(&mut node_packages)?;

        // Update each package.json file
        for package in &node_packages {
            self.update_package_json(package).with_context(|| {
                format!("Failed to update package at {}", package.path)
            })?;
        }

        info!(
            "Successfully updated {} Node.js packages",
            node_packages.len()
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_specifier_update() {
        let updater = NodeUpdater::new(".");

        assert_eq!(
            updater.update_version_specifier("^1.0.0", "2.0.0"),
            "^2.0.0"
        );
        assert_eq!(
            updater.update_version_specifier("~1.0.0", "2.0.0"),
            "~2.0.0"
        );
        assert_eq!(
            updater.update_version_specifier(">=1.0.0", "2.0.0"),
            ">=2.0.0"
        );
        assert_eq!(updater.update_version_specifier("1.0.0", "2.0.0"), "2.0.0");
        assert_eq!(updater.update_version_specifier("*", "2.0.0"), "*");
        assert_eq!(
            updater.update_version_specifier("workspace:^1.0.0", "2.0.0"),
            "workspace:2.0.0"
        );
    }

    #[test]
    fn test_local_dependency_detection() {
        let updater = NodeUpdater::new(".");

        assert!(
            updater
                .is_local_dependency("file:../other-package")
                .unwrap()
        );
        assert!(
            updater
                .is_local_dependency("link:../other-package")
                .unwrap()
        );
        assert!(updater.is_local_dependency("workspace:^1.0.0").unwrap());
        assert!(updater.is_local_dependency("*").unwrap());

        assert!(!updater.is_local_dependency("^1.0.0").unwrap());
        assert!(!updater.is_local_dependency("npm:package@1.0.0").unwrap());
    }
}
