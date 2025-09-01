use crate::updater::framework::{Framework, Package, PackageMetadata};
use crate::updater::rust::types::{
    DependencyType, LocalDependency, RustPackageMetadata,
};
use crate::updater::traits::PackageUpdater;
use color_eyre::eyre::{Context, Result};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use toml::Value;

pub struct CargoUpdater {
    /// Root directory of the repository
    root_path: PathBuf,
    /// Whether to run cargo check after updating manifests
    update_lockfile: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct CargoToml {
    package: Option<PackageSection>,
    workspace: Option<WorkspaceSection>,
    dependencies: Option<HashMap<String, Value>>,
    #[serde(rename = "dev-dependencies")]
    dev_dependencies: Option<HashMap<String, Value>>,
    #[serde(rename = "build-dependencies")]
    build_dependencies: Option<HashMap<String, Value>>,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PackageSection {
    name: String,
    version: String,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct WorkspaceSection {
    members: Option<Vec<String>>,
    dependencies: Option<HashMap<String, Value>>,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

impl CargoUpdater {
    pub fn new<P: AsRef<Path>>(root_path: P) -> Self {
        Self {
            root_path: root_path.as_ref().to_path_buf(),
            update_lockfile: true,
        }
    }

    #[cfg(test)]
    pub fn with_lockfile_update(mut self, update: bool) -> Self {
        self.update_lockfile = update;
        self
    }

    /// Analyze packages to determine workspace structure and dependencies
    pub fn analyze_packages(
        &self,
        packages: &[Package],
    ) -> Result<Vec<Package>> {
        let mut analyzed_packages = Vec::new();

        // Analyze each package individually
        for package in packages {
            // Only analyze Rust packages
            if let Framework::Rust(_) = package.framework {
                let analyzed_package = self.analyze_single_package(package)?;
                analyzed_packages.push(analyzed_package);
            } else {
                // Pass through non-Rust packages unchanged
                analyzed_packages.push(package.clone());
            }
        }

        Ok(analyzed_packages)
    }

    fn find_workspace_root(&self) -> Result<Option<PathBuf>> {
        let manifest_path = self.root_path.join("Cargo.toml");
        if manifest_path.exists() {
            let content =
                fs::read_to_string(&manifest_path).with_context(|| {
                    format!("Failed to read {}", manifest_path.display())
                })?;

            let cargo_toml: CargoToml =
                toml::from_str(&content).with_context(|| {
                    format!("Failed to parse {}", manifest_path.display())
                })?;

            if cargo_toml.workspace.is_some() {
                return Ok(Some(self.root_path.clone()));
            }
        }

        Ok(None)
    }

    fn get_workspace_members(
        &self,
        workspace_root: &Path,
    ) -> Result<Vec<String>> {
        let manifest_path = workspace_root.join("Cargo.toml");
        let content =
            fs::read_to_string(&manifest_path).with_context(|| {
                format!(
                    "Failed to read workspace manifest {}",
                    manifest_path.display()
                )
            })?;

        let cargo_toml: CargoToml =
            toml::from_str(&content).with_context(|| {
                format!(
                    "Failed to parse workspace manifest {}",
                    manifest_path.display()
                )
            })?;

        if let Some(workspace) = cargo_toml.workspace
            && let Some(members) = workspace.members
        {
            return Ok(members);
        }

        Ok(Vec::new())
    }

    fn analyze_single_package(&self, package: &Package) -> Result<Package> {
        let manifest_path = self.root_path.join(&package.manifest_path);
        let content =
            fs::read_to_string(&manifest_path).with_context(|| {
                format!("Failed to read {}", manifest_path.display())
            })?;

        let cargo_toml: CargoToml =
            toml::from_str(&content).with_context(|| {
                format!("Failed to parse {}", manifest_path.display())
            })?;

        let current_version =
            cargo_toml.package.as_ref().map(|p| p.version.clone());

        // Determine workspace information
        let workspace_root = self.find_workspace_root()?;
        let workspace_members = if let Some(ref root) = workspace_root {
            self.get_workspace_members(root)?
        } else {
            Vec::new()
        };

        let is_workspace_root = workspace_root
            .as_ref()
            .map(|root| {
                let package_path = if package.path == "." {
                    self.root_path.clone()
                } else {
                    self.root_path.join(&package.path)
                };
                package_path == *root
            })
            .unwrap_or(false);

        let is_workspace_member = is_workspace_root
            || workspace_members.iter().any(|member| {
                if let Some(ref root) = workspace_root {
                    let member_path = root.join(member);
                    let package_path = self.root_path.join(&package.path);
                    member_path == package_path
                } else {
                    false
                }
            });

        // Analyze local dependencies
        let local_dependencies =
            self.find_local_dependencies(&cargo_toml, &workspace_members)?;

        let rust_metadata = RustPackageMetadata {
            is_workspace_member,
            is_workspace_root,
            local_dependencies,
        };

        Ok(package
            .clone()
            .with_current_version(current_version)
            .with_metadata(PackageMetadata::Rust(rust_metadata)))
    }

    fn find_local_dependencies(
        &self,
        cargo_toml: &CargoToml,
        workspace_members: &[String],
    ) -> Result<Vec<LocalDependency>> {
        let mut local_deps = Vec::new();

        // Check regular dependencies
        if let Some(deps) = &cargo_toml.dependencies {
            local_deps.extend(self.extract_local_dependencies(
                deps,
                workspace_members,
                DependencyType::Runtime,
            )?);
        }

        // Check dev dependencies
        if let Some(deps) = &cargo_toml.dev_dependencies {
            local_deps.extend(self.extract_local_dependencies(
                deps,
                workspace_members,
                DependencyType::Development,
            )?);
        }

        // Check build dependencies
        if let Some(deps) = &cargo_toml.build_dependencies {
            local_deps.extend(self.extract_local_dependencies(
                deps,
                workspace_members,
                DependencyType::Build,
            )?);
        }

        Ok(local_deps)
    }

    fn extract_local_dependencies(
        &self,
        dependencies: &HashMap<String, Value>,
        workspace_members: &[String],
        dep_type: DependencyType,
    ) -> Result<Vec<LocalDependency>> {
        let mut local_deps = Vec::new();

        for (name, value) in dependencies {
            // Check if this is a local dependency (has path) or workspace member
            let is_local = self.is_local_dependency(value)
                || workspace_members.contains(name);

            if is_local
                && let Some(current_version) =
                    self.extract_version_from_dependency_value(value)
            {
                // For local workspace dependencies, we typically use the same version
                // This will be updated by the caller based on the package being released
                let local_dep = LocalDependency::new(
                    name.clone(),
                    current_version.clone(),
                    current_version, // Will be updated later
                    dep_type.clone(),
                );

                local_deps.push(local_dep);
            }
        }

        Ok(local_deps)
    }

    fn is_local_dependency(&self, value: &Value) -> bool {
        match value {
            Value::Table(table) => {
                // Has a local path
                table.get("path").is_some()
            }
            _ => false,
        }
    }

    fn extract_version_from_dependency_value(
        &self,
        value: &Value,
    ) -> Option<String> {
        match value {
            Value::String(version) => Some(version.clone()),
            Value::Table(table) => {
                table.get("version")?.as_str().map(String::from)
            }
            _ => None,
        }
    }

    /// Update local dependencies in packages based on version updates
    fn update_local_dependencies(
        &self,
        packages: &mut [Package],
    ) -> Result<()> {
        // Create a map of package names to their new versions
        let version_map: HashMap<String, String> = packages
            .iter()
            .map(|p| (p.name.clone(), p.next_version.clone()))
            .collect();

        // Update local dependency version requirements
        for package in packages.iter_mut() {
            if let PackageMetadata::Rust(ref mut rust_meta) = package.metadata {
                for local_dep in &mut rust_meta.local_dependencies {
                    if let Some(new_version) = version_map.get(&local_dep.name)
                    {
                        local_dep.new_version_req = new_version.clone();
                        debug!(
                            "Updating local dependency {} from {} to {} in package {}",
                            local_dep.name,
                            local_dep.current_version_req,
                            local_dep.new_version_req,
                            package.name
                        );
                    }
                }
            }
        }

        Ok(())
    }

    fn update_manifest_file(&self, package: &Package) -> Result<()> {
        let manifest_path = self.root_path.join(&package.manifest_path);
        info!("Updating manifest: {}", manifest_path.display());

        let content =
            fs::read_to_string(&manifest_path).with_context(|| {
                format!("Failed to read {}", manifest_path.display())
            })?;

        let mut cargo_toml: CargoToml =
            toml::from_str(&content).with_context(|| {
                format!("Failed to parse {}", manifest_path.display())
            })?;

        // Update package version
        if let Some(package_section) = &mut cargo_toml.package {
            package_section.version = package.next_version.clone();
            info!(
                "Updated package {} version to {}",
                package.name, package.next_version
            );
        }

        // Update local dependencies
        if let PackageMetadata::Rust(ref rust_meta) = package.metadata {
            for local_dep in &rust_meta.local_dependencies {
                match local_dep.dependency_type {
                    DependencyType::Runtime => {
                        self.update_dependency_in_section(
                            &mut cargo_toml.dependencies,
                            local_dep,
                        )?;
                    }
                    DependencyType::Development => {
                        self.update_dependency_in_section(
                            &mut cargo_toml.dev_dependencies,
                            local_dep,
                        )?;
                    }
                    DependencyType::Build => {
                        self.update_dependency_in_section(
                            &mut cargo_toml.build_dependencies,
                            local_dep,
                        )?;
                    }
                }
            }
        }

        // Write the updated manifest
        let updated_content = toml::to_string_pretty(&cargo_toml)
            .with_context(|| {
                format!("Failed to serialize {}", manifest_path.display())
            })?;

        fs::write(&manifest_path, updated_content).with_context(|| {
            format!("Failed to write {}", manifest_path.display())
        })?;

        Ok(())
    }

    fn update_dependency_in_section(
        &self,
        dependencies: &mut Option<HashMap<String, Value>>,
        local_dep: &LocalDependency,
    ) -> Result<()> {
        if let Some(deps) = dependencies
            && let Some(dep_value) = deps.get_mut(&local_dep.name)
        {
            match dep_value {
                Value::String(_) => {
                    *dep_value =
                        Value::String(local_dep.new_version_req.clone());
                }
                Value::Table(table) => {
                    if let Some(version) = table.get_mut("version") {
                        *version =
                            Value::String(local_dep.new_version_req.clone());
                    }
                }
                _ => {
                    warn!(
                        "Unexpected dependency format for {}: {:?}",
                        local_dep.name, dep_value
                    );
                }
            }

            info!(
                "Updated local dependency {} to version {}",
                local_dep.name, local_dep.new_version_req
            );
        }

        Ok(())
    }

    fn update_lockfile(&self) -> Result<()> {
        if !self.update_lockfile {
            debug!("Skipping lockfile update");
            return Ok(());
        }

        info!("Updating Cargo.lock...");
        let output = Command::new("cargo")
            .args(["check", "--workspace"])
            .current_dir(&self.root_path)
            .output()
            .context("Failed to run cargo check")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(color_eyre::eyre::eyre!(
                "Failed to update lockfile: {}",
                stderr
            ));
        }

        info!("Successfully updated Cargo.lock");
        Ok(())
    }
}

impl PackageUpdater for CargoUpdater {
    fn update(&self, packages: Vec<Package>) -> Result<()> {
        if packages.is_empty() {
            info!("No packages to update");
            return Ok(());
        }

        info!("Starting Cargo update for {} packages", packages.len());
        info!("analyzing packages: {:#?}", packages);

        // Filter to only Rust packages
        let rust_packages: Vec<Package> = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Rust(_)))
            .collect();

        if rust_packages.is_empty() {
            info!("No Rust packages to update");
            return Ok(());
        }

        // Analyze packages to understand workspace structure and dependencies
        let mut analyzed_packages = self.analyze_packages(&rust_packages)?;

        info!("analyzed packages: {:#?}", analyzed_packages);

        // Update local dependency version requirements
        self.update_local_dependencies(&mut analyzed_packages)?;

        // Update each package's manifest file
        for package in &analyzed_packages {
            info!(
                "Updating package '{}' from {:?} to {}",
                package.name, package.current_version, package.next_version
            );

            self.update_manifest_file(package).with_context(|| {
                format!("Failed to update package {}", package.name)
            })?;

            if package.has_local_dependencies() {
                let local_deps_count =
                    if let PackageMetadata::Rust(ref meta) = package.metadata {
                        meta.local_dependencies.len()
                    } else {
                        0
                    };
                info!(
                    "Updated {} local dependencies for package '{}'",
                    local_deps_count, package.name
                );
            }
        }

        // Update the lockfile
        self.update_lockfile()
            .context("Failed to update Cargo.lock")?;

        info!("Successfully completed Cargo update");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::updater::{framework::Language, rust::types::RustMetadata};

    use super::*;

    #[test]
    fn test_cargo_updater_creation() {
        let updater = CargoUpdater::new("/tmp");
        assert_eq!(updater.root_path, PathBuf::from("/tmp"));
        assert!(updater.update_lockfile);
    }

    #[test]
    fn test_extract_version_from_dependency_value() {
        let updater = CargoUpdater::new("/tmp");

        // Test string version
        let value = Value::String("1.0.0".to_string());
        assert_eq!(
            updater.extract_version_from_dependency_value(&value),
            Some("1.0.0".to_string())
        );

        // Test table with version
        let mut table = toml::Table::new();
        table.insert("version".to_string(), Value::String("2.0.0".to_string()));
        let value = Value::Table(table);
        assert_eq!(
            updater.extract_version_from_dependency_value(&value),
            Some("2.0.0".to_string())
        );

        // Test invalid value
        let value = Value::Integer(123);
        assert_eq!(updater.extract_version_from_dependency_value(&value), None);
    }

    #[test]
    fn test_create_single_package() {
        let rust_meta = RustMetadata {
            is_workspace: false,
            workspace_root: None,
            package_manager: "cargo".to_string(),
        };

        let package = Package::new(
            "my-crate".to_string(),
            ".".to_string(),
            "1.0.0".to_string(),
            Framework::Rust(Language {
                name: "rust".into(),
                manifest_path: Path::new(".").join("Cargo.toml"),
                metadata: rust_meta,
            }),
        );

        assert_eq!(package.name, "my-crate");
        assert_eq!(package.next_version, "1.0.0");
        assert!(!package.has_local_dependencies());
    }

    #[test]
    fn test_cargo_updater_integration() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create workspace Cargo.toml
        let workspace_toml = r#"[workspace]
members = [
    "crates/core",
    "crates/cli"
]

[workspace.dependencies]
serde = "1.0"
tokio = "1.0"
"#;
        fs::write(workspace_root.join("Cargo.toml"), workspace_toml).unwrap();

        // Create core crate
        fs::create_dir_all(workspace_root.join("crates/core")).unwrap();
        let core_toml = r#"[package]
name = "example-core"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
tokio = { workspace = true }
"#;
        fs::write(workspace_root.join("crates/core/Cargo.toml"), core_toml)
            .unwrap();

        // Create basic lib.rs file for core crate
        fs::create_dir_all(workspace_root.join("crates/core/src")).unwrap();
        fs::write(
            workspace_root.join("crates/core/src/lib.rs"),
            "pub fn hello_core() -> &'static str { \"Hello from core!\" }",
        )
        .unwrap();

        // Create CLI crate with local dependency
        fs::create_dir_all(workspace_root.join("crates/cli")).unwrap();
        let cli_toml = r#"[package]
name = "example-cli"
version = "0.1.0"
edition = "2021"

[dependencies]
example-core = { path = "../core", version = "0.1.0" }
serde = { workspace = true }
"#;
        fs::write(workspace_root.join("crates/cli/Cargo.toml"), cli_toml)
            .unwrap();

        // Create basic lib.rs file for cli crate
        fs::create_dir_all(workspace_root.join("crates/cli/src")).unwrap();
        fs::write(
            workspace_root.join("crates/cli/src/lib.rs"),
            "pub fn hello_cli() -> &'static str { \"Hello from CLI!\" }",
        )
        .unwrap();

        let rust_meta = RustMetadata {
            is_workspace: true,
            workspace_root: Some(workspace_root.to_path_buf()),
            package_manager: "cargo".to_string(),
        };

        // Create packages to update
        let packages = vec![
            Package::new(
                "example-core".to_string(),
                "crates/core".to_string(),
                "0.2.0".to_string(),
                Framework::Rust(Language {
                    name: "rust".into(),
                    manifest_path: workspace_root
                        .join("crates/core/Cargo.toml"),
                    metadata: rust_meta.clone(),
                }),
            )
            .with_current_version(Some("0.1.0".to_string())),
            Package::new(
                "example-cli".to_string(),
                "crates/cli".to_string(),
                "0.2.0".to_string(),
                Framework::Rust(Language {
                    name: "rust".into(),
                    manifest_path: workspace_root.join("crates/cli/Cargo.toml"),
                    metadata: rust_meta,
                }),
            )
            .with_current_version(Some("0.1.0".to_string())),
        ];

        // Create updater and analyze packages (disable lockfile updates for test)
        let updater =
            CargoUpdater::new(workspace_root).with_lockfile_update(false);
        let analyzed_packages = updater.analyze_packages(&packages).unwrap();

        // Verify package analysis
        assert_eq!(analyzed_packages.len(), 2);

        let core_package = analyzed_packages
            .iter()
            .find(|p| p.name == "example-core")
            .expect("Should find core package");

        if let PackageMetadata::Rust(ref meta) = core_package.metadata {
            assert!(meta.is_workspace_member);
            assert!(!meta.is_workspace_root);
        } else {
            panic!("Expected Rust metadata");
        }

        let cli_package = analyzed_packages
            .iter()
            .find(|p| p.name == "example-cli")
            .expect("Should find CLI package");

        if let PackageMetadata::Rust(ref meta) = cli_package.metadata {
            assert!(meta.is_workspace_member);
            assert!(!meta.is_workspace_root);
        } else {
            panic!("Expected Rust metadata");
        }

        // Test manifest updates (without lockfile update)
        let result = updater.update(packages);
        assert!(result.is_ok(), "Update should succeed: {:?}", result.err());

        // Verify that versions were updated in manifest files
        let updated_core_toml =
            fs::read_to_string(workspace_root.join("crates/core/Cargo.toml"))
                .unwrap();
        assert!(updated_core_toml.contains(r#"version = "0.2.0""#));

        let updated_cli_toml =
            fs::read_to_string(workspace_root.join("crates/cli/Cargo.toml"))
                .unwrap();

        assert!(updated_cli_toml.contains(r#"version = "0.2.0""#));
        // Verify local dependency was updated - check for table format
        assert!(
            updated_cli_toml.contains(r#"[dependencies.example-core]"#)
                && updated_cli_toml.contains(r#"version = "0.2.0""#)
                && updated_cli_toml.contains(r#"path = "../core""#),
            "Expected to find example-core dependency table with version 0.2.0 in CLI TOML, but got:\n{}",
            updated_cli_toml
        );
    }
}
