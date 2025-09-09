use color_eyre::eyre::Result;
use log::*;
use regex::Regex;
use serde_json::{Value, json};
use serde_yaml;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use crate::updater::framework::{Framework, Package};
use crate::updater::traits::PackageUpdater;

/// Node.js package updater supporting npm, yarn, and pnpm
pub struct NodeUpdater {}

impl NodeUpdater {
    pub fn new() -> Self {
        Self {}
    }

    fn load_doc<P: AsRef<Path>>(&self, file_path: P) -> Result<Value> {
        let file = OpenOptions::new().read(true).open(file_path)?;
        let doc: Value = serde_json::from_reader(file)?;
        Ok(doc)
    }

    fn write_doc<P: AsRef<Path>>(
        &self,
        doc: &Value,
        file_path: P,
    ) -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(file_path)?;
        file.write_all(doc.to_string().as_bytes())?;
        Ok(())
    }

    fn update_deps(
        &self,
        doc: &mut Value,
        dep_kind: &str,
        other_packages: &[(String, Package)],
    ) -> Result<()> {
        if let Some(deps) = doc[dep_kind].as_object_mut() {
            for (key, value) in deps {
                if let Some((_, other_package)) =
                    other_packages.iter().find(|(n, _)| n == key)
                {
                    *value =
                        json!(other_package.next_version.semver.to_string());
                }
            }
        }

        Ok(())
    }

    fn get_packages_with_names(
        &self,
        packages: Vec<Package>,
    ) -> Vec<(String, Package)> {
        packages
            .into_iter()
            .map(|p| {
                let manifest_path = Path::new(&p.path).join("package.json");
                if let Ok(doc) = self.load_doc(manifest_path)
                    && let Some(name) = doc["name"].as_str()
                {
                    return (name.to_string(), p);
                }
                (p.name.clone(), p)
            })
            .collect::<Vec<(String, Package)>>()
    }

    /// Update package-lock.json file for a specific package
    fn update_package_lock_json_for_package(
        &self,
        current_package: (&str, &Package),
        other_packages: &[(String, Package)],
    ) -> Result<()> {
        let lock_path =
            Path::new(&current_package.1.path).join("package-lock.json");

        if !lock_path.exists() {
            return Ok(());
        }

        let mut lock_doc = self.load_doc(&lock_path)?;

        // Get root package name for later use
        let root_name = lock_doc
            .get("name")
            .and_then(|n| n.as_str())
            .map(|s| s.to_string());

        // Update root level version if this lock file corresponds to the
        // current package
        if let Some(ref name) = root_name
            && current_package.0 == name
        {
            lock_doc["version"] =
                json!(current_package.1.next_version.semver.to_string());
        }

        // Update packages section
        if let Some(packages) = lock_doc.get_mut("packages")
            && let Some(packages_obj) = packages.as_object_mut()
        {
            for (key, package_info) in packages_obj {
                if key.is_empty() {
                    // Root package entry - update version if this corresponds
                    // to the current package
                    if let Some(ref name) = root_name
                        && current_package.0 == name
                    {
                        package_info["version"] = json!(
                            current_package.1.next_version.semver.to_string()
                        );
                    }

                    // Update dependencies within root package entry
                    if let Some(deps) = package_info.get_mut("dependencies")
                        && let Some(deps_obj) = deps.as_object_mut()
                    {
                        for (dep_name, dep_info) in deps_obj {
                            if current_package.0 == dep_name {
                                *dep_info = json!(format!(
                                    "^{}",
                                    current_package
                                        .1
                                        .next_version
                                        .semver
                                        .to_string()
                                ));
                            } else if let Some((_, package)) = other_packages
                                .iter()
                                .find(|(n, _)| n == dep_name)
                            {
                                *dep_info = json!(format!(
                                    "^{}",
                                    package.next_version.semver.to_string()
                                ));
                            }
                        }
                    }

                    // Update devDependencies within root package entry
                    if let Some(dev_deps) =
                        package_info.get_mut("devDependencies")
                        && let Some(dev_deps_obj) = dev_deps.as_object_mut()
                    {
                        for (dep_name, dep_info) in dev_deps_obj {
                            if current_package.0 == dep_name {
                                *dep_info = json!(format!(
                                    "^{}",
                                    current_package
                                        .1
                                        .next_version
                                        .semver
                                        .to_string()
                                ));
                            } else if let Some((_, package)) = other_packages
                                .iter()
                                .find(|(n, _)| n == dep_name)
                            {
                                *dep_info = json!(format!(
                                    "^{}",
                                    package.next_version.semver.to_string()
                                ));
                            }
                        }
                    }
                    continue;
                }

                // Extract package name from node_modules/ key
                if let Some(package_name) = key.strip_prefix("node_modules/") {
                    // Check if it's the current package
                    if current_package.0 == package_name {
                        package_info["version"] = json!(
                            current_package.1.next_version.semver.to_string()
                        );
                    }
                    // Check if it's one of the other packages
                    else if let Some((_, package)) =
                        other_packages.iter().find(|(n, _)| n == package_name)
                    {
                        package_info["version"] =
                            json!(package.next_version.semver.to_string());
                    }
                }
            }
        }

        // Update dependencies in the root level (old lockFileVersion 1)
        if let Some(dependencies) = lock_doc.get_mut("dependencies")
            && let Some(deps_obj) = dependencies.as_object_mut()
        {
            for (dep_name, dep_info) in deps_obj {
                // Check if it's the current package
                if current_package.0 == dep_name {
                    dep_info["version"] = json!(
                        current_package.1.next_version.semver.to_string()
                    );
                }
                // Check if it's one of the other packages
                else if let Some((_, package)) =
                    other_packages.iter().find(|(n, _)| n == dep_name)
                {
                    dep_info["version"] =
                        json!(package.next_version.semver.to_string());
                }
            }
        }

        self.write_doc(&lock_doc, &lock_path)?;
        Ok(())
    }

    /// Update package-lock.json file at root path
    fn update_package_lock_json_for_root(
        &self,
        root_path: &Path,
        all_packages: &[(String, Package)],
    ) -> Result<()> {
        let lock_path = root_path.join("package-lock.json");

        if !lock_path.exists() {
            return Ok(());
        }

        let mut lock_doc = self.load_doc(&lock_path)?;

        // Get root package name for later use
        let root_name = lock_doc
            .get("name")
            .and_then(|n| n.as_str())
            .map(|s| s.to_string());

        // Update root level version if this lock file corresponds to one of
        // our packages
        if let Some(ref name) = root_name
            && let Some((_, package)) =
                all_packages.iter().find(|(n, _)| n == name)
        {
            lock_doc["version"] =
                json!(package.next_version.semver.to_string());
        }

        // Update packages section
        if let Some(packages) = lock_doc.get_mut("packages")
            && let Some(packages_obj) = packages.as_object_mut()
        {
            for (key, package_info) in packages_obj {
                if key.is_empty() {
                    // Root package entry - update version if this corresponds
                    // to one of our packages
                    if let Some(ref name) = root_name
                        && let Some((_, package)) =
                            all_packages.iter().find(|(n, _)| n == name)
                    {
                        package_info["version"] =
                            json!(package.next_version.semver.to_string());
                    }

                    // Update dependencies within root package entry
                    if let Some(deps) = package_info.get_mut("dependencies")
                        && let Some(deps_obj) = deps.as_object_mut()
                    {
                        for (dep_name, dep_info) in deps_obj {
                            if let Some((_, package)) =
                                all_packages.iter().find(|(n, _)| n == dep_name)
                            {
                                *dep_info = json!(format!(
                                    "^{}",
                                    package.next_version.semver.to_string()
                                ));
                            }
                        }
                    }

                    // Update devDependencies within root package entry
                    if let Some(dev_deps) =
                        package_info.get_mut("devDependencies")
                        && let Some(dev_deps_obj) = dev_deps.as_object_mut()
                    {
                        for (dep_name, dep_info) in dev_deps_obj {
                            if let Some((_, package)) =
                                all_packages.iter().find(|(n, _)| n == dep_name)
                            {
                                *dep_info = json!(format!(
                                    "^{}",
                                    package.next_version.semver.to_string()
                                ));
                            }
                        }
                    }
                    continue;
                }

                // Extract package name from node_modules/ key
                if let Some(package_name) = key.strip_prefix("node_modules/")
                    && let Some((_, package)) =
                        all_packages.iter().find(|(n, _)| n == package_name)
                {
                    package_info["version"] =
                        json!(package.next_version.semver.to_string());
                }
            }
        }

        // Update dependencies in the root level (old lockFileVersion 1)
        if let Some(dependencies) = lock_doc.get_mut("dependencies")
            && let Some(deps_obj) = dependencies.as_object_mut()
        {
            for (dep_name, dep_info) in deps_obj {
                if let Some((_, package)) =
                    all_packages.iter().find(|(n, _)| n == dep_name)
                {
                    dep_info["version"] =
                        json!(package.next_version.semver.to_string());
                }
            }
        }

        self.write_doc(&lock_doc, &lock_path)?;
        Ok(())
    }

    /// Update pnpm-lock.yaml file for a specific package
    /// TODO: FIXME: logic needs to be updated to match pnpm-lock.yaml spec
    fn update_pnpm_lock_yaml_for_package(
        &self,
        current_package: (&str, &Package),
        other_packages: &[(String, Package)],
    ) -> Result<()> {
        let lock_path =
            Path::new(&current_package.1.path).join("pnpm-lock.yaml");

        if !lock_path.exists() {
            return Ok(());
        }

        let file = File::open(&lock_path)?;
        let mut lock_doc: serde_yaml::Value = serde_yaml::from_reader(file)?;

        // Update dependencies section
        if let Some(dependencies) = lock_doc.get_mut("dependencies")
            && let Some(deps_map) = dependencies.as_mapping_mut()
        {
            for (key, value) in deps_map {
                if let Some(dep_name) = key.as_str() {
                    // Check if it's the current package
                    if current_package.0 == dep_name {
                        *value = serde_yaml::Value::String(
                            current_package.1.next_version.semver.to_string(),
                        );
                    }
                    // Check if it's one of the other packages
                    else if let Some((_, package)) =
                        other_packages.iter().find(|(n, _)| n == dep_name)
                    {
                        *value = serde_yaml::Value::String(
                            package.next_version.semver.to_string(),
                        );
                    }
                }
            }
        }

        // Update devDependencies section
        if let Some(dev_dependencies) = lock_doc.get_mut("devDependencies")
            && let Some(dev_deps_map) = dev_dependencies.as_mapping_mut()
        {
            for (key, value) in dev_deps_map {
                if let Some(dep_name) = key.as_str() {
                    // Check if it's the current package
                    if current_package.0 == dep_name {
                        *value = serde_yaml::Value::String(
                            current_package.1.next_version.semver.to_string(),
                        );
                    }
                    // Check if it's one of the other packages
                    else if let Some((_, package)) =
                        other_packages.iter().find(|(n, _)| n == dep_name)
                    {
                        *value = serde_yaml::Value::String(
                            package.next_version.semver.to_string(),
                        );
                    }
                }
            }
        }

        // Update packages section
        if let Some(packages) = lock_doc.get_mut("packages")
            && let Some(packages_map) = packages.as_mapping_mut()
        {
            for (key, package_info) in packages_map {
                if let Some(package_key) = key.as_str() {
                    // Parse package key format like "/@scope/package/1.0.0"
                    let mut found = false;

                    // Check if it matches the current package
                    if package_key.contains(current_package.0) {
                        if let Some(info_map) = package_info.as_mapping_mut()
                            && let Some(version_key) =
                                info_map.get_mut(serde_yaml::Value::String(
                                    "version".to_string(),
                                ))
                        {
                            *version_key = serde_yaml::Value::String(
                                current_package
                                    .1
                                    .next_version
                                    .semver
                                    .to_string(),
                            );
                        }
                        found = true;
                    }

                    // Check if it matches one of the other packages
                    if !found {
                        for (name, package) in other_packages {
                            if package_key.contains(name) {
                                if let Some(info_map) =
                                    package_info.as_mapping_mut()
                                    && let Some(version_key) = info_map.get_mut(
                                        serde_yaml::Value::String(
                                            "version".to_string(),
                                        ),
                                    )
                                {
                                    *version_key = serde_yaml::Value::String(
                                        package.next_version.semver.to_string(),
                                    );
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&lock_path)?;
        file.write_all(serde_yaml::to_string(&lock_doc)?.as_bytes())?;
        Ok(())
    }

    /// Update pnpm-lock.yaml file at root path
    /// TODO: FIXME: logic needs to be updated to match pnpm-lock.yaml spec
    fn update_pnpm_lock_yaml_for_root(
        &self,
        root_path: &Path,
        all_packages: &[(String, Package)],
    ) -> Result<()> {
        let lock_path = root_path.join("pnpm-lock.yaml");
        if !lock_path.exists() {
            return Ok(());
        }

        let file = File::open(&lock_path)?;
        let mut lock_doc: serde_yaml::Value = serde_yaml::from_reader(file)?;

        // Update dependencies section
        if let Some(dependencies) = lock_doc.get_mut("dependencies")
            && let Some(deps_map) = dependencies.as_mapping_mut()
        {
            for (key, value) in deps_map {
                if let Some(dep_name) = key.as_str()
                    && let Some((_, package)) =
                        all_packages.iter().find(|(n, _)| n == dep_name)
                {
                    *value = serde_yaml::Value::String(
                        package.next_version.semver.to_string(),
                    );
                }
            }
        }

        // Update devDependencies section
        if let Some(dev_dependencies) = lock_doc.get_mut("devDependencies")
            && let Some(dev_deps_map) = dev_dependencies.as_mapping_mut()
        {
            for (key, value) in dev_deps_map {
                if let Some(dep_name) = key.as_str()
                    && let Some((_, package)) =
                        all_packages.iter().find(|(n, _)| n == dep_name)
                {
                    *value = serde_yaml::Value::String(
                        package.next_version.semver.to_string(),
                    );
                }
            }
        }

        // Update packages section
        if let Some(packages) = lock_doc.get_mut("packages")
            && let Some(packages_map) = packages.as_mapping_mut()
        {
            for (key, package_info) in packages_map {
                if let Some(package_key) = key.as_str() {
                    // Parse package key format like "/@scope/package/1.0.0"
                    for (name, package) in all_packages {
                        if package_key.contains(name) {
                            if let Some(info_map) =
                                package_info.as_mapping_mut()
                                && let Some(version_key) =
                                    info_map.get_mut(serde_yaml::Value::String(
                                        "version".to_string(),
                                    ))
                            {
                                *version_key = serde_yaml::Value::String(
                                    package.next_version.semver.to_string(),
                                );
                            }
                            break;
                        }
                    }
                }
            }
        }

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&lock_path)?;
        file.write_all(serde_yaml::to_string(&lock_doc)?.as_bytes())?;
        Ok(())
    }

    /// Update yarn.lock file for a specific package
    fn update_yarn_lock_for_package(
        &self,
        current_package: (&str, &Package),
        other_packages: &[(String, Package)],
    ) -> Result<()> {
        let lock_path = Path::new(&current_package.1.path).join("yarn.lock");
        if !lock_path.exists() {
            return Ok(());
        }

        let file = File::open(&lock_path)?;
        let reader = BufReader::new(file);
        let mut lines: Vec<String> =
            reader.lines().collect::<Result<_, _>>()?;

        // Regex to match package entries like "package@^1.0.0:"
        let package_regex = Regex::new(r#"^"?([^@"]+)@[^"]*"?:$"#)?;
        let version_regex = Regex::new(r#"^(\s+version\s+)"(.*)""#)?;

        let mut current_yarn_package: Option<String> = None;

        for line in lines.iter_mut() {
            // Check if this line starts a new package entry
            if let Some(caps) = package_regex.captures(line) {
                current_yarn_package = Some(caps[1].to_string());
                continue;
            }

            // Check if this is a version line and we're in a relevant package
            if let (Some(pkg_name), Some(caps)) =
                (current_yarn_package.as_ref(), version_regex.captures(line))
            {
                // Check if it matches the current package
                if current_package.0 == pkg_name {
                    *line = format!(
                        "{}\"{}\"",
                        &caps[1], current_package.1.next_version.semver
                    );
                }
                // Check if it matches one of the other packages
                else if let Some((_, package)) =
                    other_packages.iter().find(|(n, _)| n == pkg_name)
                {
                    *line = format!(
                        "{}\"{}\"",
                        &caps[1], package.next_version.semver
                    );
                }
            }

            // Reset current package when we hit an empty line or start of
            // new entry
            if line.trim().is_empty()
                || (!line.starts_with(' ')
                    && !line.starts_with('\t')
                    && line.contains(':'))
            {
                current_yarn_package = None;
            }
        }

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&lock_path)?;
        file.write_all(lines.join("\n").as_bytes())?;
        Ok(())
    }

    /// Update yarn.lock file at root path
    fn update_yarn_lock_for_root(
        &self,
        root_path: &Path,
        all_packages: &[(String, Package)],
    ) -> Result<()> {
        let lock_path = root_path.join("yarn.lock");
        if !lock_path.exists() {
            return Ok(());
        }

        let file = File::open(&lock_path)?;
        let reader = BufReader::new(file);
        let mut lines: Vec<String> =
            reader.lines().collect::<Result<_, _>>()?;

        // Regex to match package entries like "package@^1.0.0:"
        let package_regex = Regex::new(r#"^"?([^@"]+)@[^"]*"?:$"#)?;
        let version_regex = Regex::new(r#"^(\s+version\s+)"(.*)""#)?;

        let mut current_yarn_package: Option<String> = None;

        for line in lines.iter_mut() {
            // Check if this line starts a new package entry
            if let Some(caps) = package_regex.captures(line) {
                current_yarn_package = Some(caps[1].to_string());
                continue;
            }

            // Check if this is a version line and we're in a relevant package
            if let (Some(pkg_name), Some(caps)) =
                (current_yarn_package.as_ref(), version_regex.captures(line))
                && let Some((_, package)) =
                    all_packages.iter().find(|(n, _)| n == pkg_name)
            {
                *line =
                    format!("{}\"{}\"", &caps[1], package.next_version.semver);
            }

            // Reset current package when we hit an empty line or start of new entry
            if line.trim().is_empty()
                || (!line.starts_with(' ')
                    && !line.starts_with('\t')
                    && line.contains(':'))
            {
                current_yarn_package = None;
            }
        }

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&lock_path)?;
        file.write_all(lines.join("\n").as_bytes())?;
        Ok(())
    }

    /// Update lock files for a specific package
    fn update_package_lock_files(
        &self,
        current_package: (&str, &Package),
        other_packages: &[(String, Package)],
    ) -> Result<()> {
        let package_path = Path::new(&current_package.1.path);

        if package_path.join("package-lock.json").exists() {
            info!("Updating package-lock.json at {}", package_path.display());
            self.update_package_lock_json_for_package(
                current_package,
                other_packages,
            )?;
        }

        if package_path.join("pnpm-lock.yaml").exists() {
            info!("Updating pnpm-lock.yaml at {}", package_path.display());
            self.update_pnpm_lock_yaml_for_package(
                current_package,
                other_packages,
            )?;
        }

        if package_path.join("yarn.lock").exists() {
            info!("Updating yarn.lock at {}", package_path.display());
            self.update_yarn_lock_for_package(current_package, other_packages)?;
        }

        Ok(())
    }

    /// Update lock files at root path
    fn update_root_lock_files(
        &self,
        root_path: &Path,
        all_packages: &[(String, Package)],
    ) -> Result<()> {
        if root_path.join("package-lock.json").exists() {
            info!("Updating package-lock.json at {}", root_path.display());
            self.update_package_lock_json_for_root(root_path, all_packages)?;
        }

        if root_path.join("pnpm-lock.yaml").exists() {
            info!("Updating pnpm-lock.yaml at {}", root_path.display());
            self.update_pnpm_lock_yaml_for_root(root_path, all_packages)?;
        }

        if root_path.join("yarn.lock").exists() {
            info!("Updating yarn.lock at {}", root_path.display());
            self.update_yarn_lock_for_root(root_path, all_packages)?;
        }

        Ok(())
    }
}

impl PackageUpdater for NodeUpdater {
    fn update(&self, root_path: &Path, packages: Vec<Package>) -> Result<()> {
        let node_packages = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Node))
            .collect::<Vec<Package>>();

        info!(
            "Found {} node packages in {}",
            node_packages.len(),
            root_path.display(),
        );

        let packages_with_names = self.get_packages_with_names(node_packages);

        for (package_name, package) in packages_with_names.iter() {
            let pkg_json = Path::new(&package.path).join("package.json");
            let mut pkg_doc = self.load_doc(pkg_json.as_path())?;
            pkg_doc["version"] = json!(package.next_version.semver.to_string());

            let other_pkgs = packages_with_names
                .iter()
                .filter(|(n, _)| n != package_name)
                .cloned()
                .collect::<Vec<(String, Package)>>();

            self.update_deps(&mut pkg_doc, "dependencies", &other_pkgs)?;
            self.update_deps(&mut pkg_doc, "dev_dependencies", &other_pkgs)?;
            self.write_doc(&pkg_doc, pkg_json.as_path())?;

            // Update lock files in this package directory
            self.update_package_lock_files(
                (package_name, package),
                &other_pkgs,
            )?;
        }

        // Update lock files at root path
        self.update_root_lock_files(root_path, &packages_with_names)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::types::Version;
    use crate::updater::framework::Framework;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_version(version: &str) -> Version {
        Version {
            tag: format!("v{}", version),
            semver: semver::Version::parse(version).unwrap(),
        }
    }

    fn create_test_package(name: &str, path: &str, version: &str) -> Package {
        Package::new(
            name.to_string(),
            path.to_string(),
            create_test_version(version),
            Framework::Node,
        )
    }

    #[test]
    fn test_load_doc_success() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");

        fs::write(
            &package_json,
            r#"{
  "name": "test-package",
  "version": "1.0.0",
  "dependencies": {
    "express": "^4.17.1"
  }
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let doc = updater.load_doc(&package_json).unwrap();

        assert_eq!(doc["name"].as_str(), Some("test-package"));
        assert_eq!(doc["version"].as_str(), Some("1.0.0"));
        assert_eq!(doc["dependencies"]["express"].as_str(), Some("^4.17.1"));
    }

    #[test]
    fn test_load_doc_file_not_found() {
        let updater = NodeUpdater::new();
        let result = updater.load_doc("/nonexistent/package.json");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_doc_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");

        fs::write(&package_json, "invalid json content").unwrap();

        let updater = NodeUpdater::new();
        let result = updater.load_doc(&package_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_write_doc() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");

        // Create initial file
        fs::write(
            &package_json,
            r#"{
  "name": "test-package",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let mut doc = updater.load_doc(&package_json).unwrap();

        // Modify the document
        doc["version"] = json!("2.0.0");

        // Write it back
        updater.write_doc(&doc, &package_json).unwrap();

        // Verify the change
        let updated_content = fs::read_to_string(&package_json).unwrap();
        assert!(updated_content.contains("\"version\":\"2.0.0\""));
    }

    #[test]
    fn test_update_deps_dependencies() {
        let updater = NodeUpdater::new();
        let mut doc = json!({
            "name": "test-package",
            "version": "1.0.0",
            "dependencies": {
                "my-dep": "1.0.0",
                "external-dep": "2.0.0"
            }
        });

        let other_packages = vec![(
            "my-dep".to_string(),
            create_test_package("my-dep", "/path/to/my-dep", "1.5.0"),
        )];

        updater
            .update_deps(&mut doc, "dependencies", &other_packages)
            .unwrap();

        assert_eq!(doc["dependencies"]["my-dep"].as_str(), Some("1.5.0"));
        assert_eq!(doc["dependencies"]["external-dep"].as_str(), Some("2.0.0"));
    }

    #[test]
    fn test_update_deps_dev_dependencies() {
        let updater = NodeUpdater::new();
        let mut doc = json!({
            "name": "test-package",
            "version": "1.0.0",
            "dev_dependencies": {
                "test-dep": "1.0.0"
            }
        });

        let other_packages = vec![(
            "test-dep".to_string(),
            create_test_package("test-dep", "/path/to/test-dep", "2.1.0"),
        )];

        updater
            .update_deps(&mut doc, "dev_dependencies", &other_packages)
            .unwrap();

        assert_eq!(doc["dev_dependencies"]["test-dep"].as_str(), Some("2.1.0"));
    }

    #[test]
    fn test_update_deps_no_dependencies() {
        let updater = NodeUpdater::new();
        let mut doc = json!({
            "name": "test-package",
            "version": "1.0.0"
        });

        let other_packages = vec![(
            "my-dep".to_string(),
            create_test_package("my-dep", "/path/to/my-dep", "1.5.0"),
        )];

        // Should not error when dependencies section doesn't exist
        updater
            .update_deps(&mut doc, "dependencies", &other_packages)
            .unwrap();

        assert!(doc["dependencies"].is_null());
    }

    #[test]
    fn test_get_packages_with_names_from_package_json() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_path = temp_dir.path().join("my-package");
        fs::create_dir_all(&pkg_path).unwrap();

        fs::write(
            pkg_path.join("package.json"),
            r#"{
  "name": "@scope/actual-name",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![create_test_package(
            "my-package",
            pkg_path.to_str().unwrap(),
            "2.0.0",
        )];

        let packages_with_names = updater.get_packages_with_names(packages);

        assert_eq!(packages_with_names.len(), 1);
        assert_eq!(packages_with_names[0].0, "@scope/actual-name");
        assert_eq!(packages_with_names[0].1.name, "my-package");
    }

    #[test]
    fn test_get_packages_with_names_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_path = temp_dir.path().join("my-package");
        fs::create_dir_all(&pkg_path).unwrap();

        // Create invalid package.json or missing file
        let updater = NodeUpdater::new();
        let packages = vec![create_test_package(
            "my-package",
            pkg_path.to_str().unwrap(),
            "2.0.0",
        )];

        let packages_with_names = updater.get_packages_with_names(packages);

        assert_eq!(packages_with_names.len(), 1);
        assert_eq!(packages_with_names[0].0, "my-package");
    }

    #[test]
    fn test_update_single_package() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("my-package");
        fs::create_dir_all(&package_path).unwrap();

        let package_json = package_path.join("package.json");
        fs::write(
            &package_json,
            r#"{
  "name": "my-package",
  "version": "1.0.0",
  "dependencies": {
    "express": "^4.17.1"
  }
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![create_test_package(
            "my-package",
            package_path.to_str().unwrap(),
            "2.0.0",
        )];

        updater.update(temp_dir.path(), packages).unwrap();

        let updated_content = fs::read_to_string(&package_json).unwrap();
        assert!(updated_content.contains("\"version\":\"2.0.0\""));
        // Express dependency should remain unchanged
        assert!(updated_content.contains("\"express\":\"^4.17.1\""));
    }

    #[test]
    fn test_update_cross_package_dependencies() {
        let temp_dir = TempDir::new().unwrap();

        // Create package A
        let pkg_a_path = temp_dir.path().join("pkg-a");
        fs::create_dir_all(&pkg_a_path).unwrap();
        fs::write(
            pkg_a_path.join("package.json"),
            r#"{
  "name": "pkg-a",
  "version": "1.0.0",
  "dependencies": {
    "lodash": "^4.17.21"
  }
}"#,
        )
        .unwrap();

        // Create package B that depends on A
        let pkg_b_path = temp_dir.path().join("pkg-b");
        fs::create_dir_all(&pkg_b_path).unwrap();
        fs::write(
            pkg_b_path.join("package.json"),
            r#"{
  "name": "pkg-b",
  "version": "1.0.0",
  "dependencies": {
    "pkg-a": "1.0.0",
    "express": "^4.17.1"
  }
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![
            create_test_package("pkg-a", pkg_a_path.to_str().unwrap(), "2.0.0"),
            create_test_package("pkg-b", pkg_b_path.to_str().unwrap(), "1.1.0"),
        ];

        updater.update(temp_dir.path(), packages).unwrap();

        // Check that pkg-a was updated to 2.0.0
        let pkg_a_content =
            fs::read_to_string(pkg_a_path.join("package.json")).unwrap();
        assert!(pkg_a_content.contains("\"version\":\"2.0.0\""));

        // Check that pkg-b was updated to 1.1.0 and its dependency on pkg-a was updated
        let pkg_b_content =
            fs::read_to_string(pkg_b_path.join("package.json")).unwrap();
        assert!(pkg_b_content.contains("\"version\":\"1.1.0\""));
        assert!(pkg_b_content.contains("\"pkg-a\":\"2.0.0\""));
        // External dependency should remain unchanged
        assert!(pkg_b_content.contains("\"express\":\"^4.17.1\""));
    }

    #[test]
    fn test_update_with_dev_dependencies() {
        let temp_dir = TempDir::new().unwrap();

        let pkg_a_path = temp_dir.path().join("pkg-a");
        let pkg_b_path = temp_dir.path().join("pkg-b");
        fs::create_dir_all(&pkg_a_path).unwrap();
        fs::create_dir_all(&pkg_b_path).unwrap();

        fs::write(
            pkg_a_path.join("package.json"),
            r#"{
  "name": "pkg-a",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        fs::write(
            pkg_b_path.join("package.json"),
            r#"{
  "name": "pkg-b",
  "version": "1.0.0",
  "dev_dependencies": {
    "pkg-a": "1.0.0",
    "jest": "^27.0.0"
  }
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![
            create_test_package("pkg-a", pkg_a_path.to_str().unwrap(), "1.5.0"),
            create_test_package("pkg-b", pkg_b_path.to_str().unwrap(), "1.2.0"),
        ];

        updater.update(temp_dir.path(), packages).unwrap();

        let pkg_b_content =
            fs::read_to_string(pkg_b_path.join("package.json")).unwrap();
        assert!(pkg_b_content.contains("\"version\":\"1.2.0\""));
        assert!(pkg_b_content.contains("\"pkg-a\":\"1.5.0\""));
        assert!(pkg_b_content.contains("\"jest\":\"^27.0.0\""));
    }

    #[test]
    fn test_update_filters_non_node_packages() {
        let temp_dir = TempDir::new().unwrap();

        let pkg_path = temp_dir.path().join("pkg");
        fs::create_dir_all(&pkg_path).unwrap();
        fs::write(
            pkg_path.join("package.json"),
            r#"{
  "name": "pkg",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![
            create_test_package("pkg", pkg_path.to_str().unwrap(), "2.0.0"),
            Package::new(
                "rust-pkg".to_string(),
                pkg_path.to_str().unwrap().to_string(),
                create_test_version("1.1.0"),
                Framework::Rust,
            ),
        ];

        updater.update(temp_dir.path(), packages).unwrap();

        // Only the Node package should be updated
        let content =
            fs::read_to_string(pkg_path.join("package.json")).unwrap();
        assert!(content.contains("\"version\":\"2.0.0\""));
    }

    #[test]
    fn test_update_with_scoped_package_names() {
        let temp_dir = TempDir::new().unwrap();

        let pkg_a_path = temp_dir.path().join("pkg-a");
        let pkg_b_path = temp_dir.path().join("pkg-b");
        fs::create_dir_all(&pkg_a_path).unwrap();
        fs::create_dir_all(&pkg_b_path).unwrap();

        fs::write(
            pkg_a_path.join("package.json"),
            r#"{
  "name": "@myorg/pkg-a",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        fs::write(
            pkg_b_path.join("package.json"),
            r#"{
  "name": "@myorg/pkg-b",
  "version": "1.0.0",
  "dependencies": {
    "@myorg/pkg-a": "1.0.0"
  }
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![
            create_test_package("pkg-a", pkg_a_path.to_str().unwrap(), "2.0.0"),
            create_test_package("pkg-b", pkg_b_path.to_str().unwrap(), "1.5.0"),
        ];

        updater.update(temp_dir.path(), packages).unwrap();

        let pkg_b_content =
            fs::read_to_string(pkg_b_path.join("package.json")).unwrap();
        assert!(pkg_b_content.contains("\"version\":\"1.5.0\""));
        assert!(pkg_b_content.contains("\"@myorg/pkg-a\":\"2.0.0\""));
    }

    #[test]
    fn test_update_with_missing_package_json() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_path = temp_dir.path().join("pkg");
        fs::create_dir_all(&pkg_path).unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![create_test_package(
            "pkg",
            pkg_path.to_str().unwrap(),
            "2.0.0",
        )];

        let result = updater.update(temp_dir.path(), packages);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_package_lock_json() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_path = temp_dir.path().join("pkg");
        fs::create_dir_all(&pkg_path).unwrap();

        // Create package.json
        fs::write(
            pkg_path.join("package.json"),
            r#"{
  "name": "test-pkg",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        // Create package-lock.json
        fs::write(
            temp_dir.path().join("package-lock.json"),
            r#"{
  "name": "root-project",
  "lockfileVersion": 2,
  "requires": true,
  "packages": {
    "": {
      "name": "root-project"
    },
    "node_modules/test-pkg": {
      "name": "test-pkg",
      "version": "1.0.0"
    }
  },
  "dependencies": {
    "test-pkg": {
      "version": "1.0.0"
    }
  }
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![create_test_package(
            "test-pkg",
            pkg_path.to_str().unwrap(),
            "2.0.0",
        )];

        updater.update(temp_dir.path(), packages).unwrap();

        let lock_content =
            fs::read_to_string(temp_dir.path().join("package-lock.json"))
                .unwrap();
        assert!(lock_content.contains("\"version\":\"2.0.0\""));
    }

    #[test]
    fn test_update_package_lock_json_root_package() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_path = temp_dir.path().join("my-root-pkg");
        fs::create_dir_all(&pkg_path).unwrap();

        // Create package.json
        fs::write(
            pkg_path.join("package.json"),
            r#"{
  "name": "my-root-pkg",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        // Create package-lock.json with root package entry
        fs::write(
            temp_dir.path().join("package-lock.json"),
            r#"{
  "name": "my-root-pkg",
  "version": "1.0.0",
  "lockfileVersion": 2,
  "requires": true,
  "packages": {
    "": {
      "name": "my-root-pkg",
      "version": "1.0.0",
      "dependencies": {
        "lodash": "^4.17.21"
      }
    },
    "node_modules/lodash": {
      "version": "4.17.21"
    }
  },
  "dependencies": {
    "lodash": {
      "version": "4.17.21"
    }
  }
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![create_test_package(
            "my-root-pkg",
            pkg_path.to_str().unwrap(),
            "2.0.0",
        )];

        updater.update(temp_dir.path(), packages).unwrap();

        let lock_content =
            fs::read_to_string(temp_dir.path().join("package-lock.json"))
                .unwrap();

        // Root level version should be updated
        let lock_json: serde_json::Value =
            serde_json::from_str(&lock_content).unwrap();
        assert_eq!(lock_json["version"].as_str(), Some("2.0.0"));

        // Root package entry (key "") should be updated
        assert_eq!(
            lock_json["packages"][""]["version"].as_str(),
            Some("2.0.0")
        );

        // External dependencies should remain unchanged
        assert_eq!(
            lock_json["dependencies"]["lodash"]["version"].as_str(),
            Some("4.17.21")
        );
    }

    #[test]
    fn test_update_pnpm_lock_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_path = temp_dir.path().join("pkg");
        fs::create_dir_all(&pkg_path).unwrap();

        // Create package.json
        fs::write(
            pkg_path.join("package.json"),
            r#"{
  "name": "test-pkg",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        // Create pnpm-lock.yaml
        fs::write(
            temp_dir.path().join("pnpm-lock.yaml"),
            r#"lockfileVersion: 5.4
dependencies:
  test-pkg: 1.0.0
packages:
  /test-pkg/1.0.0:
    version: 1.0.0
"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![create_test_package(
            "test-pkg",
            pkg_path.to_str().unwrap(),
            "2.0.0",
        )];

        updater.update(temp_dir.path(), packages).unwrap();

        let lock_content =
            fs::read_to_string(temp_dir.path().join("pnpm-lock.yaml")).unwrap();
        assert!(
            lock_content.contains("test-pkg: 2.0.0")
                || lock_content.contains("version: 2.0.0")
        );
    }

    #[test]
    fn test_update_yarn_lock() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_path = temp_dir.path().join("pkg");
        fs::create_dir_all(&pkg_path).unwrap();

        // Create package.json
        fs::write(
            pkg_path.join("package.json"),
            r#"{
  "name": "test-pkg",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        // Create yarn.lock
        fs::write(
            temp_dir.path().join("yarn.lock"),
            r#"# yarn lockfile v1

"test-pkg@^1.0.0":
  version "1.0.0"
  resolved "https://registry.yarnpkg.com/test-pkg/-/test-pkg-1.0.0.tgz"
  integrity sha512-example

other-dep@^2.0.0:
  version "2.0.0"
  resolved "https://registry.yarnpkg.com/other-dep/-/other-dep-2.0.0.tgz"
"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![create_test_package(
            "test-pkg",
            pkg_path.to_str().unwrap(),
            "2.0.0",
        )];

        updater.update(temp_dir.path(), packages).unwrap();

        let lock_content =
            fs::read_to_string(temp_dir.path().join("yarn.lock")).unwrap();
        assert!(lock_content.contains("version \"2.0.0\""));
        // Ensure other packages weren't affected
        assert!(lock_content.contains("other-dep@^2.0.0"));
    }

    #[test]
    fn test_update_with_missing_lock_files() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_path = temp_dir.path().join("pkg");
        fs::create_dir_all(&pkg_path).unwrap();

        fs::write(
            pkg_path.join("package.json"),
            r#"{
  "name": "test-pkg",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![create_test_package(
            "test-pkg",
            pkg_path.to_str().unwrap(),
            "2.0.0",
        )];

        // Should not fail when lock files don't exist
        let result = updater.update(temp_dir.path(), packages);
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_package_lock_json_modern_structure() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_path = temp_dir.path().join("pkg");
        fs::create_dir_all(&pkg_path).unwrap();

        fs::write(
            pkg_path.join("package.json"),
            r#"{
  "name": "test-pkg",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        // Create package-lock.json with modern structure (lockfileVersion 3)
        // where dependencies are under packages[""] entry
        fs::write(
            temp_dir.path().join("package-lock.json"),
            r#"{
  "name": "root-project",
  "version": "1.0.0",
  "lockfileVersion": 3,
  "requires": true,
  "packages": {
    "": {
      "name": "root-project",
      "version": "1.0.0",
      "dependencies": {
        "test-pkg": "^1.0.0"
      },
      "devDependencies": {
        "test-pkg": "^1.0.0"
      }
    },
    "node_modules/test-pkg": {
      "name": "test-pkg",
      "version": "1.0.0"
    }
  }
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![create_test_package(
            "test-pkg",
            pkg_path.to_str().unwrap(),
            "2.0.0",
        )];

        let result = updater.update(temp_dir.path(), packages);
        assert!(result.is_ok());

        let lock_content =
            fs::read_to_string(temp_dir.path().join("package-lock.json"))
                .unwrap();

        // Verify dependencies under packages[""] were updated
        assert!(
            lock_content.contains(r#""dependencies":{"test-pkg":"^2.0.0"}"#)
        );
        // Verify devDependencies under packages[""] were updated
        assert!(
            lock_content.contains(r#""devDependencies":{"test-pkg":"^2.0.0"}"#)
        );
        // Verify node_modules entry was updated
        assert!(lock_content.contains(r#""version":"2.0.0""#));
    }

    #[test]
    fn test_update_package_lock_json_node_modules_entries() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_path = temp_dir.path().join("pkg");
        fs::create_dir_all(&pkg_path).unwrap();

        fs::write(
            pkg_path.join("package.json"),
            r#"{
  "name": "test-pkg",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        // Create package-lock.json with node_modules/ entries
        fs::write(
            temp_dir.path().join("package-lock.json"),
            r#"{
  "name": "root-project",
  "version": "1.0.0",
  "lockfileVersion": 2,
  "requires": true,
  "packages": {
    "": {
      "name": "root-project",
      "version": "1.0.0"
    },
    "node_modules/test-pkg": {
      "version": "1.0.0",
      "resolved": "https://registry.npmjs.org/test-pkg/-/test-pkg-1.0.0.tgz"
    },
    "node_modules/@scope/other-pkg": {
      "version": "2.0.0",
      "resolved": "https://registry.npmjs.org/@scope/other-pkg/-/other-pkg-2.0.0.tgz"
    }
  }
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![create_test_package(
            "test-pkg",
            pkg_path.to_str().unwrap(),
            "2.0.0",
        )];

        let result = updater.update(temp_dir.path(), packages);
        assert!(result.is_ok());

        let lock_content =
            fs::read_to_string(temp_dir.path().join("package-lock.json"))
                .unwrap();

        // Verify node_modules/test-pkg was updated (version should be 2.0.0)
        assert!(lock_content.contains(r#""node_modules/test-pkg""#));
        assert!(lock_content.contains(r#""version":"2.0.0""#));

        // Verify other packages remain unchanged
        assert!(lock_content.contains(r#""node_modules/@scope/other-pkg""#));
    }

    #[test]
    fn test_update_lock_files_in_root_and_package_paths() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_path = temp_dir.path().join("pkg");
        fs::create_dir_all(&pkg_path).unwrap();

        // Create package.json files
        fs::write(
            temp_dir.path().join("package.json"),
            r#"{
  "name": "root-project",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        fs::write(
            pkg_path.join("package.json"),
            r#"{
  "name": "test-pkg",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        // Create package-lock.json in root
        fs::write(
            temp_dir.path().join("package-lock.json"),
            r#"{
  "name": "root-project",
  "version": "1.0.0",
  "lockfileVersion": 2,
  "requires": true,
  "packages": {
    "": {
      "name": "root-project",
      "version": "1.0.0"
    },
    "node_modules/test-pkg": {
      "name": "test-pkg",
      "version": "1.0.0"
    }
  }
}"#,
        )
        .unwrap();

        // Create package-lock.json in package directory
        fs::write(
            pkg_path.join("package-lock.json"),
            r#"{
  "name": "test-pkg",
  "version": "1.0.0",
  "lockfileVersion": 2,
  "requires": true,
  "packages": {
    "": {
      "name": "test-pkg",
      "version": "1.0.0"
    }
  }
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![create_test_package(
            "test-pkg",
            pkg_path.to_str().unwrap(),
            "2.0.0",
        )];

        let result = updater.update(temp_dir.path(), packages);
        assert!(result.is_ok());

        // Verify root lock file was updated
        let root_lock_content =
            fs::read_to_string(temp_dir.path().join("package-lock.json"))
                .unwrap();
        assert!(root_lock_content.contains(r#""version":"2.0.0""#));

        // Verify package lock file was updated
        let pkg_lock_content =
            fs::read_to_string(pkg_path.join("package-lock.json")).unwrap();
        assert!(pkg_lock_content.contains(r#""version":"2.0.0""#));
    }

    #[test]
    fn test_update_multiple_lock_files() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_path = temp_dir.path().join("pkg");
        fs::create_dir_all(&pkg_path).unwrap();

        // Create package.json
        fs::write(
            pkg_path.join("package.json"),
            r#"{
  "name": "test-pkg",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        // Create multiple lock files
        fs::write(
            temp_dir.path().join("package-lock.json"),
            r#"{
  "packages": {
    "node_modules/test-pkg": {
      "name": "test-pkg",
      "version": "1.0.0"
    }
  }
}"#,
        )
        .unwrap();

        fs::write(
            temp_dir.path().join("yarn.lock"),
            r#""test-pkg@^1.0.0":
  version "1.0.0"
"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![create_test_package(
            "test-pkg",
            pkg_path.to_str().unwrap(),
            "2.0.0",
        )];

        updater.update(temp_dir.path(), packages).unwrap();

        // Both lock files should be updated
        let package_lock_content =
            fs::read_to_string(temp_dir.path().join("package-lock.json"))
                .unwrap();
        assert!(package_lock_content.contains("\"version\":\"2.0.0\""));

        let yarn_lock_content =
            fs::read_to_string(temp_dir.path().join("yarn.lock")).unwrap();
        assert!(yarn_lock_content.contains("version \"2.0.0\""));
    }
}
