//! HashMap-based collection for managing resolved packages.
//!
//! Provides efficient lookup by package name with proper error
//! handling for missing packages and duplicate detection.

use std::collections::HashMap;

use crate::{ReleasaurusError, ResolvedPackage, Result};

/// Collection of resolved packages indexed by name.
///
/// Ensures uniqueness of package names and provides efficient
/// lookup operations.
#[derive(Debug)]
pub struct ResolvedPackageHash {
    hash: HashMap<String, ResolvedPackage>,
}

impl ResolvedPackageHash {
    /// Creates a new hash from a vector of resolved packages.
    ///
    /// # Errors
    ///
    /// Returns an error if duplicate package names are detected.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use releasaurus::ResolvedPackageHash;
    /// let packages = vec![/* resolved packages */];
    /// let hash = ResolvedPackageHash::new(packages)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(package_configs: Vec<ResolvedPackage>) -> Result<Self> {
        let mut hash = HashMap::with_capacity(package_configs.len());

        for pkg in package_configs {
            let name = pkg.name.clone();
            if hash.insert(name.clone(), pkg).is_some() {
                return Err(ReleasaurusError::invalid_config(format!(
                    "Duplicate package name found: '{}'",
                    name
                )));
            }
        }

        Ok(Self { hash })
    }

    /// Returns a reference to the underlying HashMap.
    ///
    /// Useful for iterating over all packages.
    pub fn hash(&self) -> &HashMap<String, ResolvedPackage> {
        &self.hash
    }

    /// Gets a package by name.
    ///
    /// # Errors
    ///
    /// Returns an error if the package name is not found.
    pub fn get(&self, name: &str) -> Result<&ResolvedPackage> {
        self.hash.get(name).ok_or_else(|| {
            ReleasaurusError::invalid_config(format!(
                "Package not found: '{}'",
                name
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analyzer::config::AnalyzerConfig, config::release_type::ReleaseType,
    };
    use std::path::PathBuf;

    fn create_test_package(name: &str) -> ResolvedPackage {
        ResolvedPackage {
            name: name.to_string(),
            normalized_workspace_root: PathBuf::from("."),
            normalized_full_path: PathBuf::from("."),
            release_type: ReleaseType::default(),
            tag_prefix: "v".to_string(),
            sub_packages: vec![],
            prerelease: None,
            auto_start_next: false,
            normalized_additional_paths: vec![],
            compiled_additional_manifests: vec![],
            analyzer_config: AnalyzerConfig::default(),
        }
    }

    #[test]
    fn creates_hash_from_packages() {
        let packages =
            vec![create_test_package("pkg1"), create_test_package("pkg2")];

        let hash = ResolvedPackageHash::new(packages).unwrap();
        assert_eq!(hash.hash().len(), 2);
    }

    #[test]
    fn rejects_duplicate_names() {
        let packages =
            vec![create_test_package("pkg1"), create_test_package("pkg1")];

        let result = ResolvedPackageHash::new(packages);
        assert!(result.is_err());
    }

    #[test]
    fn gets_package_by_name() {
        let packages = vec![create_test_package("test-pkg")];
        let hash = ResolvedPackageHash::new(packages).unwrap();

        let pkg = hash.get("test-pkg").unwrap();
        assert_eq!(pkg.name, "test-pkg");
    }

    #[test]
    fn returns_error_for_missing_package() {
        let packages = vec![create_test_package("pkg1")];
        let hash = ResolvedPackageHash::new(packages).unwrap();

        let result = hash.get("pkg2");
        assert!(result.is_err());
    }
}
