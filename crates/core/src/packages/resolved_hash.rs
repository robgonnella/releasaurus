//! HashMap-based collection for managing resolved packages.
//!
//! Provides efficient lookup by package name with proper error
//! handling for missing packages and duplicate detection.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::{
    packages::resolved::ResolvedPackage,
    result::{ReleasaurusError, Result},
};

pub type PackageName = String;

fn duplicate_name(name: &str) -> ReleasaurusError {
    ReleasaurusError::invalid_config(format!(
        "Duplicate package name found: all package and sub-package names \
         must be unique: '{name}'"
    ))
}

fn duplicate_path(path: &Path) -> ReleasaurusError {
    let display = path.to_string_lossy();
    // The repo root normalizes to an empty path, which would render as
    // a pair of bare quotes.
    let display = if display.is_empty() {
        "<repo root>".into()
    } else {
        display
    };

    ReleasaurusError::invalid_config(format!(
        "Duplicate package path found: all package and sub-package paths \
         must be unique: '{display}'"
    ))
}

/// Collection of resolved packages indexed by name.
///
/// Ensures uniqueness of package names and provides efficient
/// lookup operations.
#[derive(Debug)]
pub struct ResolvedPackageHash {
    hash: HashMap<PackageName, ResolvedPackage>,
}

impl ResolvedPackageHash {
    /// Creates a new hash from a vector of resolved packages.
    ///
    /// # Errors
    ///
    /// Returns an error if two packages or sub-packages share a name or
    /// a path. Sub-packages take part even though only top-level
    /// packages enter the map: a repeated name mis-targets lock file
    /// entries, and a repeated path means two packages write the same
    /// `CHANGELOG.md` and one is silently dropped.
    pub fn new(package_configs: Vec<ResolvedPackage>) -> Result<Self> {
        let mut hash = HashMap::with_capacity(package_configs.len());
        let mut names = HashSet::new();
        let mut paths = HashSet::new();

        for pkg in package_configs {
            let entries =
                std::iter::once((&pkg.name, &pkg.normalized_full_path)).chain(
                    pkg.sub_packages
                        .iter()
                        .map(|s| (&s.name, &s.normalized_full_path)),
                );

            for (name, path) in entries {
                if !names.insert(name.clone()) {
                    return Err(duplicate_name(name));
                }

                if !paths.insert(path.clone()) {
                    return Err(duplicate_path(path));
                }
            }

            hash.insert(pkg.name.clone(), pkg);
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
    use crate::{
        analyzer::config::AnalyzerConfig,
        config::{
            defaults::DEFAULT_COMMIT_AND_PR_TITLE_TEMPLATE,
            release_type::ReleaseType, versioning::VersioningConfig,
        },
    };

    use super::*;
    use std::path::PathBuf;

    fn create_test_package(name: &str, path: &str) -> ResolvedPackage {
        ResolvedPackage {
            name: name.to_string(),
            normalized_workspace_root: PathBuf::from("."),
            normalized_full_path: PathBuf::from(path),
            release_type: ReleaseType::default(),
            tag_prefix: "v".to_string(),
            sub_packages: vec![],
            aggregate_prereleases: false,
            normalized_additional_paths: vec![],
            additional_manifests: vec![],
            analyzer_config: AnalyzerConfig::default(),
            versioning_config: VersioningConfig::default(),
            commit_message_template: DEFAULT_COMMIT_AND_PR_TITLE_TEMPLATE
                .into(),
            pr_title_template: DEFAULT_COMMIT_AND_PR_TITLE_TEMPLATE.into(),
        }
    }

    #[test]
    fn creates_hash_from_packages() {
        let packages = vec![
            create_test_package("pkg1", "packages/pkg1"),
            create_test_package("pkg2", "packages/pkg2"),
        ];

        let hash = ResolvedPackageHash::new(packages).unwrap();
        assert_eq!(hash.hash().len(), 2);
    }

    #[test]
    fn accepts_a_package_with_distinctly_pathed_sub_packages() {
        let mut pkg = create_test_package("parent", ".");
        pkg.sub_packages = vec![
            create_test_package("sub-a", "crates/a"),
            create_test_package("sub-b", "crates/b"),
        ];

        let hash = ResolvedPackageHash::new(vec![pkg]).unwrap();
        assert_eq!(hash.hash().len(), 1);
    }

    #[test]
    fn rejects_duplicate_names() {
        let packages = vec![
            create_test_package("pkg1", "packages/a"),
            create_test_package("pkg1", "packages/b"),
        ];

        let err = ResolvedPackageHash::new(packages).unwrap_err();
        assert!(matches!(err, ReleasaurusError::InvalidConfig(_)));
    }

    /// Two packages on one path both write `<path>/CHANGELOG.md`, and one
    /// is silently dropped.
    #[test]
    fn rejects_duplicate_paths() {
        let packages = vec![
            create_test_package("pkg1", "packages/shared"),
            create_test_package("pkg2", "packages/shared"),
        ];

        let err = ResolvedPackageHash::new(packages).unwrap_err();
        assert!(matches!(err, ReleasaurusError::InvalidConfig(_)));
    }

    #[test]
    fn rejects_a_top_level_path_colliding_with_a_sub_package_path() {
        let mut parent = create_test_package("parent", ".");
        parent.sub_packages = vec![create_test_package("sub-a", "crates/a")];

        let packages = vec![parent, create_test_package("other", "crates/a")];

        let err = ResolvedPackageHash::new(packages).unwrap_err();
        assert!(matches!(err, ReleasaurusError::InvalidConfig(_)));
    }

    /// Sub-package names key lock file entries, so a repeat mis-targets
    /// the bump even though subs never enter the map.
    #[test]
    fn rejects_duplicate_sub_package_names() {
        let mut parent = create_test_package("parent", ".");
        parent.sub_packages = vec![
            create_test_package("sub-a", "crates/a"),
            create_test_package("sub-a", "crates/b"),
        ];

        let err = ResolvedPackageHash::new(vec![parent]).unwrap_err();
        assert!(matches!(err, ReleasaurusError::InvalidConfig(_)));
    }

    /// The repo root normalizes to an empty path; `''` tells the user
    /// nothing about which package to fix.
    #[test]
    fn names_the_repo_root_in_a_duplicate_path_error() {
        let packages = vec![
            create_test_package("pkg1", ""),
            create_test_package("pkg2", ""),
        ];

        let err = ResolvedPackageHash::new(packages).unwrap_err();
        assert!(err.to_string().contains("<repo root>"));
    }

    #[test]
    fn gets_package_by_name() {
        let packages = vec![create_test_package("test-pkg", ".")];
        let hash = ResolvedPackageHash::new(packages).unwrap();

        let pkg = hash.get("test-pkg").unwrap();
        assert_eq!(pkg.name, "test-pkg");
    }

    #[test]
    fn returns_error_for_missing_package() {
        let packages = vec![create_test_package("pkg1", ".")];
        let hash = ResolvedPackageHash::new(packages).unwrap();

        let result = hash.get("pkg2");
        assert!(result.is_err());
    }
}
