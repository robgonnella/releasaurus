//! Python updater for handling Python projects with various build systems and package managers

use crate::updater::framework::{Framework, Package, PackageMetadata};
use crate::updater::python::types::{
    DependencyType, LocalDependency, PythonPackageMetadata,
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

/// Python updater - handles various Python packaging formats and build systems
pub struct PythonUpdater {
    /// Root directory of the repository
    root_path: PathBuf,
    /// Whether to update lock files after manifest changes
    update_lockfiles: bool,
}

/// Represents a pyproject.toml file
#[derive(Debug, Clone, Deserialize, Serialize)]
struct PyProjectToml {
    #[serde(rename = "build-system")]
    build_system: Option<BuildSystem>,
    project: Option<ProjectSection>,
    tool: Option<ToolSection>,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

/// Build system configuration in pyproject.toml
#[derive(Debug, Clone, Deserialize, Serialize)]
struct BuildSystem {
    requires: Option<Vec<String>>,
    #[serde(rename = "build-backend")]
    build_backend: Option<String>,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

/// Project section in pyproject.toml (PEP 621)
#[derive(Debug, Clone, Deserialize, Serialize)]
struct ProjectSection {
    name: String,
    version: Option<String>,
    dependencies: Option<Vec<String>>,
    #[serde(rename = "optional-dependencies")]
    optional_dependencies: Option<HashMap<String, Vec<String>>>,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

/// Tool-specific sections in pyproject.toml
#[derive(Debug, Clone, Deserialize, Serialize)]
struct ToolSection {
    poetry: Option<PoetrySection>,
    setuptools: Option<SetuptoolsSection>,
    flit: Option<FlitSection>,
    pdm: Option<PdmSection>,
    hatch: Option<HatchSection>,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

/// Poetry configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
struct PoetrySection {
    name: String,
    version: String,
    dependencies: Option<HashMap<String, Value>>,
    #[serde(rename = "group")]
    groups: Option<HashMap<String, DependencyGroup>>,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

/// Poetry dependency group
#[derive(Debug, Clone, Deserialize, Serialize)]
struct DependencyGroup {
    dependencies: Option<HashMap<String, Value>>,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

/// Setuptools configuration in pyproject.toml
#[derive(Debug, Clone, Deserialize, Serialize)]
struct SetuptoolsSection {
    #[serde(rename = "dynamic")]
    dynamic_fields: Option<Vec<String>>,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

/// Flit configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
struct FlitSection {
    module: Option<String>,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

/// PDM configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
struct PdmSection {
    version: Option<Value>,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

/// Hatchling configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
struct HatchSection {
    version: Option<HashMap<String, Value>>,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

/// Represents a setup.py file content (simplified parsing)
#[derive(Debug, Clone)]
struct SetupPy {
    name: Option<String>,
    version: Option<String>,
    install_requires: Vec<String>,
    extras_require: HashMap<String, Vec<String>>,
}

impl PythonUpdater {
    /// Create a new Python updater
    pub fn new<P: AsRef<Path>>(root_path: P) -> Self {
        Self {
            root_path: root_path.as_ref().to_path_buf(),
            update_lockfiles: true,
        }
    }

    /// Analyze packages to understand Python project structure and dependencies
    pub fn analyze_packages(
        &self,
        packages: &[Package],
    ) -> Result<Vec<Package>> {
        let mut analyzed_packages = Vec::new();

        for package in packages {
            // Only analyze Python packages
            if let Framework::Python(_) = package.framework {
                let analyzed_package = self.analyze_single_package(package)?;
                analyzed_packages.push(analyzed_package);
            } else {
                // Pass through non-Python packages unchanged
                analyzed_packages.push(package.clone());
            }
        }

        Ok(analyzed_packages)
    }

    /// Analyze a single Python package to understand its structure
    fn analyze_single_package(&self, package: &Package) -> Result<Package> {
        let manifest_path = self.root_path.join(&package.manifest_path);

        debug!("Analyzing Python package: {}", manifest_path.display());

        // Determine the build system and package structure
        let local_dependencies = if manifest_path.file_name()
            == Some(std::ffi::OsStr::new("pyproject.toml"))
        {
            self.analyze_pyproject_toml(&manifest_path)?
        } else if manifest_path.file_name()
            == Some(std::ffi::OsStr::new("setup.py"))
        {
            self.analyze_setup_py(&manifest_path)?
        } else {
            Vec::new()
        };

        let python_metadata = PythonPackageMetadata {
            local_dependencies,
            python_requires: self.extract_python_requires(&manifest_path)?,
        };

        Ok(package
            .clone()
            .with_metadata(PackageMetadata::Python(python_metadata)))
    }

    /// Analyze pyproject.toml file
    fn analyze_pyproject_toml(
        &self,
        manifest_path: &Path,
    ) -> Result<Vec<LocalDependency>> {
        let content = fs::read_to_string(manifest_path).with_context(|| {
            format!("Failed to read {}", manifest_path.display())
        })?;

        let pyproject: PyProjectToml =
            toml::from_str(&content).with_context(|| {
                format!("Failed to parse {}", manifest_path.display())
            })?;

        let local_dependencies =
            self.extract_dependencies_from_pyproject(&pyproject)?;

        Ok(local_dependencies)
    }

    /// Analyze setup.py file (simplified - real parsing would need AST analysis)
    fn analyze_setup_py(
        &self,
        manifest_path: &Path,
    ) -> Result<Vec<LocalDependency>> {
        let content = fs::read_to_string(manifest_path).with_context(|| {
            format!("Failed to read {}", manifest_path.display())
        })?;

        let setup_py = self.parse_setup_py(&content)?;
        let local_dependencies =
            self.extract_dependencies_from_setup_py(&setup_py)?;

        Ok(local_dependencies)
    }

    /// Extract version from Python file content using regex
    fn extract_version_from_python_file(
        &self,
        content: &str,
    ) -> Option<String> {
        use regex::Regex;

        // Common version patterns in Python files
        let patterns = [
            r#"__version__\s*=\s*["']([^"']+)["']"#,
            r#"version\s*=\s*["']([^"']+)["']"#,
            r#"VERSION\s*=\s*["']([^"']+)["']"#,
        ];

        for pattern in &patterns {
            if let Ok(re) = Regex::new(pattern)
                && let Some(captures) = re.captures(content)
                && let Some(version_match) = captures.get(1)
            {
                return Some(version_match.as_str().to_string());
            }
        }

        None
    }

    /// Extract local dependencies from pyproject.toml
    fn extract_dependencies_from_pyproject(
        &self,
        pyproject: &PyProjectToml,
    ) -> Result<Vec<LocalDependency>> {
        let mut local_deps = Vec::new();

        // Extract from PEP 621 project dependencies
        if let Some(ref project) = pyproject.project {
            if let Some(ref dependencies) = project.dependencies {
                local_deps.extend(self.parse_dependency_strings(
                    dependencies,
                    DependencyType::Runtime,
                )?);
            }

            if let Some(ref optional_deps) = project.optional_dependencies {
                for (group_name, deps) in optional_deps {
                    let dep_type =
                        if group_name == "test" || group_name == "dev" {
                            DependencyType::Development
                        } else {
                            DependencyType::Optional
                        };
                    local_deps
                        .extend(self.parse_dependency_strings(deps, dep_type)?);
                }
            }
        }

        // Extract from Poetry dependencies
        if let Some(ref tool) = pyproject.tool
            && let Some(ref poetry) = tool.poetry
        {
            if let Some(ref dependencies) = poetry.dependencies {
                local_deps.extend(self.parse_poetry_dependencies(
                    dependencies,
                    DependencyType::Runtime,
                )?);
            }

            if let Some(ref groups) = poetry.groups {
                for (group_name, group) in groups {
                    let dep_type =
                        if group_name == "dev" || group_name == "test" {
                            DependencyType::Development
                        } else {
                            DependencyType::Optional
                        };

                    if let Some(ref group_deps) = group.dependencies {
                        local_deps.extend(
                            self.parse_poetry_dependencies(
                                group_deps, dep_type,
                            )?,
                        );
                    }
                }
            }
        }

        Ok(local_deps)
    }

    /// Extract dependencies from setup.py
    fn extract_dependencies_from_setup_py(
        &self,
        setup_py: &SetupPy,
    ) -> Result<Vec<LocalDependency>> {
        let mut local_deps = Vec::new();

        // Parse install_requires
        local_deps.extend(self.parse_dependency_strings(
            &setup_py.install_requires,
            DependencyType::Runtime,
        )?);

        // Parse extras_require
        for (extra_name, deps) in &setup_py.extras_require {
            let dep_type = if extra_name == "dev" || extra_name == "test" {
                DependencyType::Development
            } else {
                DependencyType::Optional
            };
            local_deps.extend(self.parse_dependency_strings(deps, dep_type)?);
        }

        Ok(local_deps)
    }

    /// Parse dependency strings (PEP 508 format)
    fn parse_dependency_strings(
        &self,
        deps: &[String],
        dep_type: DependencyType,
    ) -> Result<Vec<LocalDependency>> {
        let mut local_deps = Vec::new();

        for dep_str in deps {
            if let Some(local_dep) =
                self.parse_dependency_string(dep_str, dep_type.clone())?
            {
                local_deps.push(local_dep);
            }
        }

        Ok(local_deps)
    }

    /// Parse a single dependency string to check if it's a local dependency
    fn parse_dependency_string(
        &self,
        dep_str: &str,
        dep_type: DependencyType,
    ) -> Result<Option<LocalDependency>> {
        // Simple parsing - real implementation would use a PEP 508 parser
        let parts: Vec<&str> = dep_str.split_whitespace().collect();
        if let Some(name_part) = parts.first() {
            // Remove version specifiers
            let name = name_part
                .split(&['>', '<', '=', '!', '~'][..])
                .next()
                .unwrap_or(name_part);

            // Check if this looks like a local dependency (simple heuristic)
            if self.is_likely_local_dependency(name) {
                return Ok(Some(LocalDependency::new(
                    name.to_string(),
                    dep_str.to_string(), // Current requirement
                    dep_str.to_string(), // Will be updated later
                    dep_type,
                )));
            }
        }

        Ok(None)
    }

    /// Parse Poetry-style dependencies
    fn parse_poetry_dependencies(
        &self,
        deps: &HashMap<String, Value>,
        dep_type: DependencyType,
    ) -> Result<Vec<LocalDependency>> {
        let mut local_deps = Vec::new();

        for (name, value) in deps {
            if name == "python" {
                continue; // Skip Python version requirement
            }

            if self.is_likely_local_dependency(name) {
                let version_req = match value {
                    Value::String(version) => version.clone(),
                    Value::Table(table) => table
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("*")
                        .to_string(),
                    _ => "*".to_string(),
                };

                local_deps.push(LocalDependency::new(
                    name.clone(),
                    version_req.clone(),
                    version_req, // Will be updated later
                    dep_type.clone(),
                ));
            }
        }

        Ok(local_deps)
    }

    /// Heuristic to determine if a dependency name is likely a local package
    fn is_likely_local_dependency(&self, name: &str) -> bool {
        // Check if there's a directory with this name that looks like a Python package
        let possible_paths = [
            self.root_path.join(name),
            self.root_path.join("src").join(name),
            self.root_path.join("packages").join(name),
            self.root_path.join("libs").join(name),
        ];

        for path in &possible_paths {
            if path.is_dir() {
                // Check for Python package indicators
                let indicators = [
                    path.join("pyproject.toml"),
                    path.join("setup.py"),
                    path.join("setup.cfg"),
                    path.join("__init__.py"),
                ];

                if indicators.iter().any(|p| p.exists()) {
                    return true;
                }
            }
        }

        false
    }

    /// Extract Python version requirement
    fn extract_python_requires(
        &self,
        manifest_path: &Path,
    ) -> Result<Option<String>> {
        if manifest_path.file_name()
            == Some(std::ffi::OsStr::new("pyproject.toml"))
        {
            let content = fs::read_to_string(manifest_path)?;
            let pyproject: PyProjectToml = toml::from_str(&content)?;

            // Check PEP 621 project.requires-python
            if let Some(ref project) = pyproject.project
                && let Some(requires_python) =
                    project.other.get("requires-python")
                && let Some(version_str) = requires_python.as_str()
            {
                return Ok(Some(version_str.to_string()));
            }

            // Check Poetry python dependency
            if let Some(ref tool) = pyproject.tool
                && let Some(ref poetry) = tool.poetry
                && let Some(ref deps) = poetry.dependencies
                && let Some(python_dep) = deps.get("python")
                && let Some(version_str) = python_dep.as_str()
            {
                return Ok(Some(version_str.to_string()));
            }
        }

        Ok(None)
    }

    /// Parse setup.py content (simplified - real implementation would need AST parsing)
    fn parse_setup_py(&self, content: &str) -> Result<SetupPy> {
        // This is a very simplified parser - real implementation would need proper AST parsing
        let mut setup_py = SetupPy {
            name: None,
            version: None,
            install_requires: Vec::new(),
            extras_require: HashMap::new(),
        };

        // Use regex to extract basic information
        if let Some(version) = self.extract_version_from_python_file(content) {
            setup_py.version = Some(version);
        }

        // Extract name (simplified)
        if let Some(name_match) =
            regex::Regex::new(r#"name\s*=\s*["']([^"']+)["']"#)
                .ok()
                .and_then(|re| re.captures(content))
                .and_then(|caps| caps.get(1))
        {
            setup_py.name = Some(name_match.as_str().to_string());
        }

        Ok(setup_py)
    }

    /// Update local dependencies with new versions
    fn update_local_dependencies(
        &self,
        packages: &mut [Package],
    ) -> Result<()> {
        // Create a map of package names to their new versions
        let version_map: HashMap<String, String> = packages
            .iter()
            .map(|p| (p.name.clone(), p.next_version.semver.to_string()))
            .collect();

        // Update local dependency version requirements
        for package in packages.iter_mut() {
            if let PackageMetadata::Python(ref mut python_meta) =
                package.metadata
            {
                for local_dep in &mut python_meta.local_dependencies {
                    if let Some(new_version) = version_map.get(&local_dep.name)
                    {
                        // Update version requirement (keeping the specifier style)
                        local_dep.new_version_req = self
                            .update_version_specifier(
                                &local_dep.current_version_req,
                                new_version,
                            );

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

    /// Update version specifier while preserving the original format
    fn update_version_specifier(
        &self,
        current: &str,
        new_version: &str,
    ) -> String {
        // If it's a simple version, replace it
        if current == new_version
            || current.chars().all(|c| c.is_ascii_digit() || c == '.')
        {
            return new_version.to_string();
        }

        // Handle version specifiers like ">=1.0.0", "~=1.0", "==1.0.0", etc.
        let specifier_chars = ['>', '<', '=', '!', '~'];
        if let Some(pos) = current.find(|c: char| !specifier_chars.contains(&c))
        {
            let (prefix, _) = current.split_at(pos);
            format!("{}{}", prefix, new_version)
        } else {
            new_version.to_string()
        }
    }

    /// Update manifest file with new version and dependencies
    fn update_manifest_file(&self, package: &Package) -> Result<()> {
        let manifest_path = self.root_path.join(&package.manifest_path);
        info!("Updating Python manifest: {}", manifest_path.display());

        if manifest_path.file_name()
            == Some(std::ffi::OsStr::new("pyproject.toml"))
        {
            self.update_pyproject_toml(&manifest_path, package)
        } else if manifest_path.file_name()
            == Some(std::ffi::OsStr::new("setup.py"))
        {
            self.update_setup_py(&manifest_path, package)
        } else {
            warn!(
                "Unsupported manifest file type: {}",
                manifest_path.display()
            );
            Ok(())
        }
    }

    /// Update pyproject.toml file
    fn update_pyproject_toml(
        &self,
        manifest_path: &Path,
        package: &Package,
    ) -> Result<()> {
        let content = fs::read_to_string(manifest_path)?;
        let mut pyproject: PyProjectToml = toml::from_str(&content)?;

        // Update version in appropriate section
        if let Some(ref mut project) = pyproject.project
            && project.version.is_some()
        {
            project.version = Some(package.next_version.semver.to_string());
            info!(
                "Updated PEP 621 project version to {}",
                package.next_version.semver
            );
        }

        if let Some(ref mut tool) = pyproject.tool
            && let Some(ref mut poetry) = tool.poetry
        {
            poetry.version = package.next_version.semver.to_string();
            info!("Updated Poetry version to {}", package.next_version.semver);
        }

        // Update local dependencies
        if let PackageMetadata::Python(ref python_meta) = package.metadata {
            self.update_pyproject_dependencies(
                &mut pyproject,
                &python_meta.local_dependencies,
            )?;
        }

        // Write updated content
        let updated_content =
            toml::to_string_pretty(&pyproject).with_context(|| {
                format!("Failed to serialize {}", manifest_path.display())
            })?;

        fs::write(manifest_path, updated_content).with_context(|| {
            format!("Failed to write {}", manifest_path.display())
        })?;

        Ok(())
    }

    /// Update dependencies in pyproject.toml
    fn update_pyproject_dependencies(
        &self,
        pyproject: &mut PyProjectToml,
        local_deps: &[LocalDependency],
    ) -> Result<()> {
        // Update PEP 621 dependencies
        if let Some(ref mut project) = pyproject.project
            && let Some(ref mut dependencies) = project.dependencies
        {
            self.update_dependency_list(
                dependencies,
                local_deps,
                DependencyType::Runtime,
            );
        }

        // Update Poetry dependencies
        if let Some(ref mut tool) = pyproject.tool
            && let Some(ref mut poetry) = tool.poetry
        {
            if let Some(ref mut dependencies) = poetry.dependencies {
                self.update_poetry_dependency_map(
                    dependencies,
                    local_deps,
                    DependencyType::Runtime,
                );
            }

            if let Some(ref mut groups) = poetry.groups {
                for (group_name, group) in groups {
                    if let Some(ref mut group_deps) = group.dependencies {
                        let dep_type =
                            if group_name == "dev" || group_name == "test" {
                                DependencyType::Development
                            } else {
                                DependencyType::Optional
                            };
                        self.update_poetry_dependency_map(
                            group_deps, local_deps, dep_type,
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Update dependency list (PEP 621 format)
    fn update_dependency_list(
        &self,
        dependencies: &mut [String],
        local_deps: &[LocalDependency],
        dep_type: DependencyType,
    ) {
        for dep in dependencies.iter_mut() {
            for local_dep in local_deps {
                if local_dep.dependency_type == dep_type
                    && dep.contains(&local_dep.name)
                {
                    *dep = local_dep.new_version_req.clone();
                    info!(
                        "Updated dependency {} to {}",
                        local_dep.name, local_dep.new_version_req
                    );
                }
            }
        }
    }

    /// Update Poetry dependency map
    fn update_poetry_dependency_map(
        &self,
        dependencies: &mut HashMap<String, Value>,
        local_deps: &[LocalDependency],
        dep_type: DependencyType,
    ) {
        for local_dep in local_deps {
            if local_dep.dependency_type == dep_type
                && let Some(dep_value) = dependencies.get_mut(&local_dep.name)
            {
                match dep_value {
                    Value::String(_) => {
                        *dep_value =
                            Value::String(local_dep.new_version_req.clone());
                    }
                    Value::Table(table) => {
                        if let Some(version) = table.get_mut("version") {
                            *version = Value::String(
                                local_dep.new_version_req.clone(),
                            );
                        }
                    }
                    _ => {}
                }
                info!(
                    "Updated Poetry dependency {} to {}",
                    local_dep.name, local_dep.new_version_req
                );
            }
        }
    }

    /// Update setup.py file (simplified - real implementation would need AST manipulation)
    fn update_setup_py(
        &self,
        manifest_path: &Path,
        package: &Package,
    ) -> Result<()> {
        let content = fs::read_to_string(manifest_path)?;

        // Simple regex-based replacement (not robust for all cases)
        let version_pattern =
            regex::Regex::new(r#"version\s*=\s*["']([^"']+)["']"#)
                .context("Failed to compile version regex")?;

        let updated_content = version_pattern.replace_all(
            &content,
            format!(r#"version="{}""#, package.next_version.semver).as_str(),
        );

        if updated_content != content {
            fs::write(manifest_path, updated_content.as_ref()).with_context(
                || format!("Failed to write {}", manifest_path.display()),
            )?;
            info!(
                "Updated setup.py version to {}",
                package.next_version.semver
            );
        }

        Ok(())
    }

    /// Update lock files if enabled
    fn update_lockfiles(&self, packages: &[Package]) -> Result<()> {
        if !self.update_lockfiles {
            debug!("Skipping lockfile updates");
            return Ok(());
        }

        for package in packages {
            let package_path = self.root_path.join(&package.path);
            self.update_package_lockfiles(&package_path)?;
        }

        Ok(())
    }

    /// Update lock files for a specific package
    fn update_package_lockfiles(&self, package_path: &Path) -> Result<()> {
        // Try Poetry first
        if package_path.join("poetry.lock").exists() {
            info!("Updating poetry.lock");
            let output = Command::new("poetry")
                .args(["lock", "--no-update"])
                .current_dir(package_path)
                .output();

            match output {
                Ok(result) if result.status.success() => {
                    info!("Successfully updated poetry.lock");
                }
                Ok(result) => {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    warn!("Poetry lock failed: {}", stderr);
                }
                Err(e) => {
                    warn!("Failed to run poetry lock: {}", e);
                }
            }
        }

        // Try pipenv
        if package_path.join("Pipfile").exists() {
            info!("Updating Pipfile.lock");
            let output = Command::new("pipenv")
                .args(["lock"])
                .current_dir(package_path)
                .output();

            match output {
                Ok(result) if result.status.success() => {
                    info!("Successfully updated Pipfile.lock");
                }
                Ok(result) => {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    warn!("Pipenv lock failed: {}", stderr);
                }
                Err(e) => {
                    warn!("Failed to run pipenv lock: {}", e);
                }
            }
        }

        // Try PDM
        if package_path.join("pdm.lock").exists() {
            info!("Updating pdm.lock");
            let output = Command::new("pdm")
                .args(["lock", "--update-reuse"])
                .current_dir(package_path)
                .output();

            match output {
                Ok(result) if result.status.success() => {
                    info!("Successfully updated pdm.lock");
                }
                Ok(result) => {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    warn!("PDM lock failed: {}", stderr);
                }
                Err(e) => {
                    warn!("Failed to run pdm lock: {}", e);
                }
            }
        }

        Ok(())
    }
}

impl PackageUpdater for PythonUpdater {
    fn update(&self, packages: Vec<Package>) -> Result<()> {
        if packages.is_empty() {
            info!("No packages to update");
            return Ok(());
        }

        info!("Starting Python update for {} packages", packages.len());

        // Filter to only Python packages
        let python_packages: Vec<Package> = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Python(_)))
            .collect();

        if python_packages.is_empty() {
            info!("No Python packages to update");
            return Ok(());
        }

        // Analyze packages to understand structure and dependencies
        let mut analyzed_packages = self.analyze_packages(&python_packages)?;

        // Update local dependency version requirements
        self.update_local_dependencies(&mut analyzed_packages)?;

        // Update each package's manifest file
        for package in &analyzed_packages {
            info!(
                "Updating Python package '{}' from {:?} to {}",
                package.name,
                package.current_version,
                package.next_version.semver
            );

            self.update_manifest_file(package).with_context(|| {
                format!("Failed to update package {}", package.name)
            })?;

            if package.has_local_dependencies() {
                let local_deps_count =
                    if let PackageMetadata::Python(ref meta) = package.metadata
                    {
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

        // Update lock files
        self.update_lockfiles(&analyzed_packages)
            .context("Failed to update lock files")?;

        info!("Successfully completed Python update");
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        analyzer::types::Version,
        updater::{framework::Language, python::types::PythonMetadata},
    };

    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_python_updater_creation() {
        let updater = PythonUpdater::new("/tmp");
        assert_eq!(updater.root_path, PathBuf::from("/tmp"));
        assert!(updater.update_lockfiles);
    }

    #[test]
    fn test_version_specifier_update() {
        let updater = PythonUpdater::new("/tmp");

        assert_eq!(updater.update_version_specifier("1.0.0", "1.1.0"), "1.1.0");
        assert_eq!(
            updater.update_version_specifier(">=1.0.0", "1.1.0"),
            ">=1.1.0"
        );
        assert_eq!(
            updater.update_version_specifier("~=1.0", "1.1.0"),
            "~=1.1.0"
        );
        assert_eq!(
            updater.update_version_specifier("==1.0.0", "1.1.0"),
            "==1.1.0"
        );
        assert_eq!(
            updater.update_version_specifier("!=1.0.0", "1.1.0"),
            "!=1.1.0"
        );
    }

    #[test]
    fn test_extract_version_from_python_file() {
        let updater = PythonUpdater::new("/tmp");

        let content1 = r#"__version__ = "1.2.3""#;
        assert_eq!(
            updater.extract_version_from_python_file(content1),
            Some("1.2.3".to_string())
        );

        let content2 = r#"VERSION = '2.0.0'"#;
        assert_eq!(
            updater.extract_version_from_python_file(content2),
            Some("2.0.0".to_string())
        );

        let content3 = r#"version = "0.1.0""#;
        assert_eq!(
            updater.extract_version_from_python_file(content3),
            Some("0.1.0".to_string())
        );

        let content4 = r#"no version here"#;
        assert_eq!(updater.extract_version_from_python_file(content4), None);
    }

    #[test]
    fn test_poetry_project_integration() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("my-project");
        fs::create_dir_all(&project_path).unwrap();

        // Create pyproject.toml with Poetry configuration
        let pyproject_content = r#"[build-system]
requires = ["poetry-core>=1.0.0"]
build-backend = "poetry.core.masonry.api"

[tool.poetry]
name = "my-project"
version = "0.1.0"
description = "Test project"

[tool.poetry.dependencies]
python = "^3.8"
local-dep = { path = "../local-dep", version = "0.1.0" }
requests = "^2.25.0"

[tool.poetry.group.dev.dependencies]
pytest = "^6.0"
local-test-dep = { path = "../test-utils", version = "0.1.0" }
"#;

        fs::write(project_path.join("pyproject.toml"), pyproject_content)
            .unwrap();

        // Create the local dependencies
        fs::create_dir_all(temp_dir.path().join("local-dep")).unwrap();
        fs::write(
            temp_dir.path().join("local-dep/pyproject.toml"),
            r#"[tool.poetry]
name = "local-dep"
version = "0.1.0"
"#,
        )
        .unwrap();

        fs::create_dir_all(temp_dir.path().join("test-utils")).unwrap();
        fs::write(
            temp_dir.path().join("test-utils/pyproject.toml"),
            r#"[tool.poetry]
name = "local-test-dep"
version = "0.1.0"
"#,
        )
        .unwrap();

        let python_meta = PythonMetadata {
            build_system: "poetry".to_string(),
            package_manager: "poetry".to_string(),
            uses_pyproject: true,
        };

        let semver_version = semver::Version::parse("0.2.0").unwrap();

        let package = Package::new(
            "my-project".to_string(),
            "my-project".to_string(),
            Version {
                tag: "v0.1.0".into(),
                semver: semver_version,
            },
            Framework::Python(Language {
                name: "python".into(),
                manifest_path: Path::new(".").join("pyproject.toml"),
                metadata: python_meta,
            }),
        );

        let updater = PythonUpdater::new(temp_dir.path());
        let analyzed_packages = updater.analyze_packages(&[package]).unwrap();

        assert_eq!(analyzed_packages.len(), 1);
        let analyzed_package = &analyzed_packages[0];

        assert_eq!(analyzed_package.next_version.semver.to_string(), "0.2.0");

        if let PackageMetadata::Python(ref meta) = analyzed_package.metadata {
            assert!(!meta.local_dependencies.is_empty());

            // Should detect local dependencies
            let local_dep_names: Vec<_> = meta
                .local_dependencies
                .iter()
                .map(|d| d.name.as_str())
                .collect();
            assert!(local_dep_names.contains(&"local-dep"));
        } else {
            panic!("Expected Python metadata");
        }

        // Test the update
        let result = updater.update(analyzed_packages);
        assert!(result.is_ok(), "Update should succeed: {:?}", result.err());

        // Verify that version was updated
        let updated_content =
            fs::read_to_string(project_path.join("pyproject.toml")).unwrap();
        assert!(updated_content.contains(r#"version = "0.2.0""#));
    }

    #[test]
    fn test_pep621_project_format() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("pep621-project");
        fs::create_dir_all(&project_path).unwrap();

        // Create pyproject.toml with PEP 621 format
        let pyproject_content = r#"[build-system]
requires = ["setuptools>=61.0", "wheel"]
build-backend = "setuptools.build_meta"

[project]
name = "pep621-project"
version = "1.0.0"
description = "PEP 621 test project"
dependencies = [
    "requests>=2.25.0",
    "local-utils>=0.1.0",
]

[project.optional-dependencies]
test = [
    "pytest>=6.0",
    "local-test-helpers>=0.1.0",
]
"#;

        fs::write(project_path.join("pyproject.toml"), pyproject_content)
            .unwrap();

        // Create local dependencies
        fs::create_dir_all(temp_dir.path().join("local-utils")).unwrap();
        fs::write(temp_dir.path().join("local-utils/__init__.py"), "").unwrap();

        let python_meta = PythonMetadata {
            build_system: "setuptools".to_string(),
            package_manager: "pip".to_string(),
            uses_pyproject: true,
        };

        let semver_version = semver::Version::parse("1.1.0").unwrap();

        let package = Package::new(
            "pep621-project".to_string(),
            "pep621-project".to_string(),
            Version {
                tag: "v1.0.0".into(),
                semver: semver_version,
            },
            Framework::Python(Language {
                name: "python".into(),
                manifest_path: Path::new(".").join("pyproject.toml"),
                metadata: python_meta,
            }),
        );

        let updater = PythonUpdater::new(temp_dir.path());
        let analyzed_packages = updater.analyze_packages(&[package]).unwrap();

        assert_eq!(analyzed_packages.len(), 1);
        let analyzed_package = &analyzed_packages[0];

        assert_eq!(analyzed_package.next_version.semver.to_string(), "1.1.0");

        // Test the update
        let result = updater.update(analyzed_packages);
        assert!(result.is_ok());

        // Verify that version was updated in PEP 621 format
        let updated_content =
            fs::read_to_string(project_path.join("pyproject.toml")).unwrap();
        assert!(updated_content.contains(r#"version = "1.1.0""#));
    }

    #[test]
    fn test_setup_py_project() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("setup-py-project");
        fs::create_dir_all(&project_path).unwrap();

        // Create setup.py
        let setup_py_content = r#"from setuptools import setup, find_packages

setup(
    name="setup-py-project",
    version="2.0.0",
    packages=find_packages(),
    install_requires=[
        "requests>=2.25.0",
        "click>=7.0",
    ],
    extras_require={
        "dev": ["pytest>=6.0", "black>=21.0"],
    },
)
"#;

        fs::write(project_path.join("setup.py"), setup_py_content).unwrap();

        let python_meta = PythonMetadata {
            build_system: "setuptools".to_string(),
            package_manager: "pip".to_string(),
            uses_pyproject: false,
        };

        let semver_version = semver::Version::parse("2.1.0").unwrap();

        let package = Package::new(
            "setup-py-project".to_string(),
            "setup-py-project".to_string(),
            Version {
                tag: "v2.0.0".into(),
                semver: semver_version,
            },
            Framework::Python(Language {
                name: "python".into(),
                manifest_path: Path::new(".").join("setup.py"),
                metadata: python_meta,
            }),
        );

        let updater = PythonUpdater::new(temp_dir.path());
        let analyzed_packages = updater.analyze_packages(&[package]).unwrap();

        assert_eq!(analyzed_packages.len(), 1);

        // Test the update
        let result = updater.update(analyzed_packages);
        assert!(result.is_ok());

        // Verify that version was updated in setup.py
        let updated_content =
            fs::read_to_string(project_path.join("setup.py")).unwrap();
        assert!(updated_content.contains(r#"version="2.1.0""#));
    }
}
