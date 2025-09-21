use log::*;
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::Path,
};
use toml_edit::{DocumentMut, value};

use crate::{result::Result, updater::framework::Package};

pub struct CargoToml {}

impl CargoToml {
    pub fn new() -> Self {
        Self {}
    }

    pub fn is_workspace(&self, root_path: &Path) -> Result<bool> {
        match self.load_doc(root_path.join("Cargo.toml")) {
            Ok(doc) => Ok(doc.get("workspace").is_some()),
            Err(_) => Ok(false),
        }
    }

    pub fn get_packages_with_names(
        &self,
        packages: Vec<Package>,
    ) -> Vec<(String, Package)> {
        packages
            .into_iter()
            .map(|p| {
                let manifest_path = Path::new(&p.path).join("Cargo.toml");
                if let Ok(doc) = self.load_doc(manifest_path) {
                    let pkg_name = self.get_package_name(&doc, &p);
                    return (pkg_name, p);
                }
                (p.name.clone(), p)
            })
            .collect::<Vec<(String, Package)>>()
    }

    pub fn process_packages(
        &self,
        packages: &[(String, Package)],
    ) -> Result<()> {
        for (package_name, package) in packages.iter() {
            let manifest_path = Path::new(&package.path).join("Cargo.toml");
            // let lock_path = Path::new(&package.path).join("Cargo.lock");

            let mut doc = self.load_doc(manifest_path.as_path())?;

            if doc.get("workspace").is_some() {
                debug!("skipping cargo workspace file");
                continue;
            }

            // let mut lock_doc = self.load_doc(lock_path.as_path()).ok();
            // let mut lock_updated = false;

            let next_version = package.next_version.semver.to_string();

            info!("setting version for {package_name} to {next_version}");

            doc["package"]["version"] = value(&next_version);

            let other_pkgs = packages
                .iter()
                .filter(|(n, _)| n != package_name)
                .cloned()
                .collect::<Vec<(String, Package)>>();

            // loop other packages to check if they current manifest deps
            for (dep_name, dep) in other_pkgs.iter() {
                let dep_next_version = dep.next_version.semver.to_string();

                self.process_dependencies(
                    &mut doc,
                    dep_name,
                    &dep_next_version,
                    "dependencies",
                );

                self.process_dependencies(
                    &mut doc,
                    dep_name,
                    &dep_next_version,
                    "dev-dependencies",
                );

                self.process_dependencies(
                    &mut doc,
                    dep_name,
                    &dep_next_version,
                    "build-dependencies",
                );

                // if let Some(lock_doc) = &mut lock_doc {
                //     self.process_lockfile_dependencies(
                //         lock_doc,
                //         dep_name,
                //         &dep_next_version,
                //     );
                //     lock_updated = true;
                // }
            }

            self.write_doc(&mut doc, manifest_path.as_path())?;

            // if let Some(mut lock_doc) = lock_doc
            //     && lock_updated
            // {
            //     self.write_doc(&mut lock_doc, lock_path.as_path())?;
            // }
        }

        Ok(())
    }

    fn process_dependencies(
        &self,
        doc: &mut DocumentMut,
        package_name: &str,
        next_version: &str,
        kind: &str,
    ) {
        let dep_exists = doc
            .get(kind)
            .and_then(|deps| deps.as_table())
            .and_then(|t| t.get(package_name))
            .is_some();

        let is_version_object = doc
            .get(kind)
            .and_then(|deps| deps.as_table())
            .and_then(|t| t.get(package_name))
            .map(|p| {
                // Check if it's a table with version field or inline table with
                //  version field
                p.as_table()
                    .map(|t| t.contains_key("version"))
                    .unwrap_or(false)
                    || p.as_inline_table()
                        .map(|t| t.contains_key("version"))
                        .unwrap_or(false)
            })
            .unwrap_or(false);

        if dep_exists {
            if is_version_object {
                doc[kind][&package_name]["version"] = value(next_version);
            } else {
                doc[kind][&package_name] = value(next_version);
            }
        }
    }

    fn get_package_name(&self, doc: &DocumentMut, package: &Package) -> String {
        doc.get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("name"))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string())
            .unwrap_or(package.name.clone())
    }

    fn load_doc<P: AsRef<Path>>(&self, file_path: P) -> Result<DocumentMut> {
        let mut file = OpenOptions::new().read(true).open(file_path)?;
        let mut content = String::from("");
        file.read_to_string(&mut content)?;
        let doc = content.parse::<DocumentMut>()?;
        Ok(doc)
    }

    fn write_doc<P: AsRef<Path>>(
        &self,
        doc: &mut DocumentMut,
        file_path: P,
    ) -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(file_path)?;
        file.write_all(doc.to_string().as_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::release::Tag;
    use crate::updater::framework::{Framework, Package};
    use std::fs;
    use tempfile::TempDir;

    fn create_test_package(name: &str, path: &str, version: &str) -> Package {
        Package::new(
            name.to_string(),
            path.to_string(),
            Tag {
                sha: "abc123".into(),
                name: format!("v{}", version),
                semver: semver::Version::parse(version).unwrap(),
            },
            Framework::Rust,
        )
    }

    #[test]
    fn test_new() {
        let cargo_toml = CargoToml::new();
        // Just verify we can create an instance
        assert_eq!(std::mem::size_of_val(&cargo_toml), 0);
    }

    #[test]
    fn test_is_workspace_true() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create workspace Cargo.toml
        fs::write(
            path.join("Cargo.toml"),
            r#"[workspace]
members = [
    "crates/core",
    "crates/cli"
]

[workspace.dependencies]
serde = "1.0"
"#,
        )
        .unwrap();

        let cargo_toml = CargoToml::new();
        let result = cargo_toml.is_workspace(path).unwrap();
        assert!(result);
    }

    #[test]
    fn test_is_workspace_false() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create regular package Cargo.toml
        fs::write(
            path.join("Cargo.toml"),
            r#"[package]
name = "test-crate"
version = "1.0.0"

[dependencies]
serde = "1.0"
"#,
        )
        .unwrap();

        let cargo_toml = CargoToml::new();
        let result = cargo_toml.is_workspace(path).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_is_workspace_no_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        let cargo_toml = CargoToml::new();
        let result = cargo_toml.is_workspace(path).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_get_packages_with_names() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create package directory structure
        let pkg_dir = path.join("test-crate");
        fs::create_dir_all(&pkg_dir).unwrap();

        // Create Cargo.toml with custom name
        fs::write(
            pkg_dir.join("Cargo.toml"),
            r#"[package]
name = "custom-crate-name"
version = "1.0.0"
"#,
        )
        .unwrap();

        let package = create_test_package(
            "fallback-name",
            pkg_dir.to_str().unwrap(),
            "2.0.0",
        );

        let cargo_toml = CargoToml::new();
        let result = cargo_toml.get_packages_with_names(vec![package]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "custom-crate-name");
        assert_eq!(result[0].1.name, "fallback-name");
    }

    #[test]
    fn test_get_packages_with_names_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create package directory without Cargo.toml
        let pkg_dir = path.join("test-crate");
        fs::create_dir_all(&pkg_dir).unwrap();

        let package = create_test_package(
            "fallback-name",
            pkg_dir.to_str().unwrap(),
            "2.0.0",
        );

        let cargo_toml = CargoToml::new();
        let result = cargo_toml.get_packages_with_names(vec![package]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "fallback-name"); // Should use fallback
        assert_eq!(result[0].1.name, "fallback-name");
    }

    #[test]
    fn test_process_packages_single_package() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        let pkg_dir = path.join("test-crate");
        fs::create_dir_all(&pkg_dir).unwrap();

        // Create initial Cargo.toml
        fs::write(
            pkg_dir.join("Cargo.toml"),
            r#"[package]
name = "test-crate"
version = "1.0.0"

[dependencies]
serde = "1.0"
"#,
        )
        .unwrap();

        let package = create_test_package(
            "test-crate",
            pkg_dir.to_str().unwrap(),
            "2.0.0",
        );

        let cargo_toml = CargoToml::new();
        let packages_with_names = vec![("test-crate".to_string(), package)];

        cargo_toml.process_packages(&packages_with_names).unwrap();

        // Verify the version was updated
        let updated_content =
            fs::read_to_string(pkg_dir.join("Cargo.toml")).unwrap();
        assert!(updated_content.contains(r#"version = "2.0.0""#));
        assert!(updated_content.contains(r#"name = "test-crate""#));
    }

    #[test]
    fn test_process_packages_skip_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        let pkg_dir = path.join("workspace");
        fs::create_dir_all(&pkg_dir).unwrap();

        // Create workspace Cargo.toml
        fs::write(
            pkg_dir.join("Cargo.toml"),
            r#"[workspace]
members = ["crate1", "crate2"]

[workspace.dependencies]
serde = "1.0"
"#,
        )
        .unwrap();

        let package = create_test_package(
            "workspace",
            pkg_dir.to_str().unwrap(),
            "2.0.0",
        );

        let cargo_toml = CargoToml::new();
        let packages_with_names = vec![("workspace".to_string(), package)];

        // Should succeed but not modify workspace file
        cargo_toml.process_packages(&packages_with_names).unwrap();

        // Verify the content is unchanged (workspace files are skipped)
        let content = fs::read_to_string(pkg_dir.join("Cargo.toml")).unwrap();
        assert!(content.contains("[workspace]"));
        assert!(!content.contains(r#"version = "2.0.0""#));
    }

    #[test]
    fn test_process_packages_with_dependencies() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create two packages
        let pkg1_dir = path.join("crate1");
        let pkg2_dir = path.join("crate2");
        fs::create_dir_all(&pkg1_dir).unwrap();
        fs::create_dir_all(&pkg2_dir).unwrap();

        // Package 1 depends on package 2
        fs::write(
            pkg1_dir.join("Cargo.toml"),
            r#"[package]
name = "crate1"
version = "1.0.0"

[dependencies]
crate2 = "1.0.0"
serde = "1.0"
"#,
        )
        .unwrap();

        // Package 2 is standalone
        fs::write(
            pkg2_dir.join("Cargo.toml"),
            r#"[package]
name = "crate2"
version = "1.0.0"

[dependencies]
tokio = "1.0"
"#,
        )
        .unwrap();

        let package1 =
            create_test_package("crate1", pkg1_dir.to_str().unwrap(), "2.0.0");
        let package2 =
            create_test_package("crate2", pkg2_dir.to_str().unwrap(), "3.0.0");

        let cargo_toml = CargoToml::new();
        let packages_with_names = vec![
            ("crate1".to_string(), package1),
            ("crate2".to_string(), package2),
        ];

        cargo_toml.process_packages(&packages_with_names).unwrap();

        // Verify both packages were updated
        let pkg1_content =
            fs::read_to_string(pkg1_dir.join("Cargo.toml")).unwrap();
        let pkg2_content =
            fs::read_to_string(pkg2_dir.join("Cargo.toml")).unwrap();

        // Package 1 should have updated version and updated dependency on crate2
        assert!(pkg1_content.contains(r#"version = "2.0.0""#));
        assert!(pkg1_content.contains(r#"crate2 = "3.0.0""#));

        // Package 2 should have updated version
        assert!(pkg2_content.contains(r#"version = "3.0.0""#));
    }

    #[test]
    fn test_process_packages_with_dependency_object() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        let pkg1_dir = path.join("crate1");
        let pkg2_dir = path.join("crate2");
        fs::create_dir_all(&pkg1_dir).unwrap();
        fs::create_dir_all(&pkg2_dir).unwrap();

        // Package 1 with object-style dependency on package 2
        fs::write(
            pkg1_dir.join("Cargo.toml"),
            r#"[package]
name = "crate1"
version = "1.0.0"

[dependencies]
crate2 = { version = "1.0.0", features = ["extra"] }
"#,
        )
        .unwrap();

        fs::write(
            pkg2_dir.join("Cargo.toml"),
            r#"[package]
name = "crate2"
version = "1.0.0"
"#,
        )
        .unwrap();

        let package1 =
            create_test_package("crate1", pkg1_dir.to_str().unwrap(), "2.0.0");
        let package2 =
            create_test_package("crate2", pkg2_dir.to_str().unwrap(), "3.0.0");

        let cargo_toml = CargoToml::new();
        let packages_with_names = vec![
            ("crate1".to_string(), package1),
            ("crate2".to_string(), package2),
        ];

        cargo_toml.process_packages(&packages_with_names).unwrap();

        let pkg1_content =
            fs::read_to_string(pkg1_dir.join("Cargo.toml")).unwrap();

        // Should update the version field within the dependency object
        assert!(pkg1_content.contains(r#"version = "2.0.0""#)); // Package version
        assert!(pkg1_content.contains(
            r#"crate2 = { version = "3.0.0", features = ["extra"] }"#
        )); // Dependency with features preserved
    }

    #[test]
    fn test_process_packages_with_dev_dependencies() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        let pkg1_dir = path.join("crate1");
        let pkg2_dir = path.join("crate2");
        fs::create_dir_all(&pkg1_dir).unwrap();
        fs::create_dir_all(&pkg2_dir).unwrap();

        // Package 1 with dev-dependency on package 2
        fs::write(
            pkg1_dir.join("Cargo.toml"),
            r#"[package]
name = "crate1"
version = "1.0.0"

[dev-dependencies]
crate2 = "1.0.0"
"#,
        )
        .unwrap();

        fs::write(
            pkg2_dir.join("Cargo.toml"),
            r#"[package]
name = "crate2"
version = "1.0.0"
"#,
        )
        .unwrap();

        let package1 =
            create_test_package("crate1", pkg1_dir.to_str().unwrap(), "2.0.0");
        let package2 =
            create_test_package("crate2", pkg2_dir.to_str().unwrap(), "3.0.0");

        let cargo_toml = CargoToml::new();
        let packages_with_names = vec![
            ("crate1".to_string(), package1),
            ("crate2".to_string(), package2),
        ];

        cargo_toml.process_packages(&packages_with_names).unwrap();

        let pkg1_content =
            fs::read_to_string(pkg1_dir.join("Cargo.toml")).unwrap();

        // Should update dev-dependency
        assert!(pkg1_content.contains(r#"version = "2.0.0""#)); // Package version
        assert!(pkg1_content.contains(r#"crate2 = "3.0.0""#)); // Dev dependency version
    }

    #[test]
    fn test_process_packages_with_build_dependencies() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        let pkg1_dir = path.join("crate1");
        let pkg2_dir = path.join("crate2");
        fs::create_dir_all(&pkg1_dir).unwrap();
        fs::create_dir_all(&pkg2_dir).unwrap();

        // Package 1 with build-dependency on package 2
        fs::write(
            pkg1_dir.join("Cargo.toml"),
            r#"[package]
name = "crate1"
version = "1.0.0"

[build-dependencies]
crate2 = "1.0.0"
"#,
        )
        .unwrap();

        fs::write(
            pkg2_dir.join("Cargo.toml"),
            r#"[package]
name = "crate2"
version = "1.0.0"
"#,
        )
        .unwrap();

        let package1 =
            create_test_package("crate1", pkg1_dir.to_str().unwrap(), "2.0.0");
        let package2 =
            create_test_package("crate2", pkg2_dir.to_str().unwrap(), "3.0.0");

        let cargo_toml = CargoToml::new();
        let packages_with_names = vec![
            ("crate1".to_string(), package1),
            ("crate2".to_string(), package2),
        ];

        cargo_toml.process_packages(&packages_with_names).unwrap();

        let pkg1_content =
            fs::read_to_string(pkg1_dir.join("Cargo.toml")).unwrap();

        // Should update build-dependency
        assert!(pkg1_content.contains(r#"version = "2.0.0""#)); // Package version
        assert!(pkg1_content.contains(r#"crate2 = "3.0.0""#)); // Build dependency version
    }

    #[test]
    fn test_load_doc_success() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("test.toml"),
            r#"[package]
name = "test"
version = "1.0.0"
"#,
        )
        .unwrap();

        let cargo_toml = CargoToml::new();
        let doc = cargo_toml.load_doc(path.join("test.toml")).unwrap();

        assert!(doc.get("package").is_some());
    }

    #[test]
    fn test_load_doc_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        let cargo_toml = CargoToml::new();
        let result = cargo_toml.load_doc(path.join("nonexistent.toml"));

        assert!(result.is_err());
    }

    #[test]
    fn test_load_doc_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Write invalid TOML
        fs::write(path.join("invalid.toml"), "invalid toml content [[[")
            .unwrap();

        let cargo_toml = CargoToml::new();
        let result = cargo_toml.load_doc(path.join("invalid.toml"));

        assert!(result.is_err());
    }

    #[test]
    fn test_write_doc() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        let file_path = path.join("output.toml");

        // Create initial file
        fs::write(&file_path, "").unwrap();

        let cargo_toml = CargoToml::new();
        let mut doc = r#"[package]
name = "test"
version = "1.0.0"
"#
        .parse::<DocumentMut>()
        .unwrap();

        cargo_toml.write_doc(&mut doc, &file_path).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains(r#"name = "test""#));
        assert!(content.contains(r#"version = "1.0.0""#));
    }

    #[test]
    fn test_get_package_name_from_manifest() {
        let cargo_toml = CargoToml::new();
        let doc = r#"[package]
name = "custom-name"
version = "1.0.0"
"#
        .parse::<DocumentMut>()
        .unwrap();

        let package = create_test_package("fallback", "path", "1.0.0");
        let name = cargo_toml.get_package_name(&doc, &package);

        assert_eq!(name, "custom-name");
    }

    #[test]
    fn test_get_package_name_fallback() {
        let cargo_toml = CargoToml::new();
        let doc = r#"[lib]
name = "lib"
"#
        .parse::<DocumentMut>()
        .unwrap(); // No [package] section

        let package = create_test_package("fallback", "path", "1.0.0");
        let name = cargo_toml.get_package_name(&doc, &package);

        assert_eq!(name, "fallback");
    }

    #[test]
    fn test_process_dependencies_simple_version() {
        let cargo_toml = CargoToml::new();
        let mut doc = r#"[package]
name = "test"
version = "1.0.0"

[dependencies]
dep1 = "1.0.0"
dep2 = "2.0.0"
"#
        .parse::<DocumentMut>()
        .unwrap();

        cargo_toml.process_dependencies(
            &mut doc,
            "dep1",
            "1.5.0",
            "dependencies",
        );

        let content = doc.to_string();
        assert!(content.contains(r#"dep1 = "1.5.0""#));
        assert!(content.contains(r#"dep2 = "2.0.0""#)); // Unchanged
    }

    #[test]
    fn test_process_dependencies_object_version() {
        let cargo_toml = CargoToml::new();
        let mut doc = r#"[package]
name = "test"
version = "1.0.0"

[dependencies]
dep1 = { version = "1.0.0", features = ["extra"] }
"#
        .parse::<DocumentMut>()
        .unwrap();

        cargo_toml.process_dependencies(
            &mut doc,
            "dep1",
            "1.5.0",
            "dependencies",
        );

        let content = doc.to_string();

        assert!(
            content.contains(
                r#"dep1 = { version = "1.5.0", features = ["extra"] }"#
            )
        );
    }

    #[test]
    fn test_process_dependencies_nonexistent() {
        let cargo_toml = CargoToml::new();
        let mut doc = r#"[package]
name = "test"
version = "1.0.0"

[dependencies]
dep1 = "1.0.0"
"#
        .parse::<DocumentMut>()
        .unwrap();

        let original_content = doc.to_string();
        cargo_toml.process_dependencies(
            &mut doc,
            "nonexistent",
            "2.0.0",
            "dependencies",
        );

        // Should not change anything if dependency doesn't exist
        assert_eq!(doc.to_string(), original_content);
    }
}
