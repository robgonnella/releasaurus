use async_trait::async_trait;

use crate::{
    forge::{request::FileChange, traits::FileLoader},
    result::Result,
    updater::framework::Package,
};

#[async_trait]
/// Common interface for updating version files in different language packages.
pub trait PackageUpdater {
    /// Update version files for packages in the repository.
    async fn update(
        &self,
        packages: Vec<Package>,
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>>;
}

#[cfg(test)]
pub mod mock {
    //! Mock implementation for the PackageUpdater trait.
    //!
    //! This module provides a manual mock implementation since mockall's `automock`
    //! has limitations with async_trait lifetimes.
    //!
    //! # Example
    //!
    //! ```rust,ignore
    //! use releasaurus::updater::traits::mock::MockPackageUpdater;
    //! use releasaurus::forge::request::{FileChange, FileUpdateType};
    //!
    //! let mut mock = MockPackageUpdater::new();
    //!
    //! // Setup expectation
    //! mock.expect_update(|packages, _loader| {
    //!     Ok(Some(vec![FileChange {
    //!         path: "Cargo.toml".to_string(),
    //!         content: "version = \"2.0.0\"".to_string(),
    //!         update_type: FileUpdateType::Replace,
    //!     }]))
    //! });
    //!
    //! // Use the mock
    //! let result = mock.update(packages, &loader).await.unwrap();
    //! ```

    use super::*;
    use std::sync::{Arc, Mutex};

    type UpdateFn = dyn Fn(Vec<Package>, &dyn FileLoader) -> Result<Option<Vec<FileChange>>>
        + Send
        + Sync;

    /// Manual mock implementation for PackageUpdater trait.
    ///
    /// This mock allows you to set expectations for the `update` method and control
    /// its return value during tests. Unlike mockall's automock, this implementation
    /// works seamlessly with async_trait.
    ///
    /// # Usage
    ///
    /// 1. Create a new mock with `MockPackageUpdater::new()`
    /// 2. Set up expectations with `expect_update()`
    /// 3. Call `update()` through the PackageUpdater trait
    ///
    /// # Panics
    ///
    /// Panics if `update()` is called without setting an expectation via `expect_update()`.
    pub struct MockPackageUpdater {
        update_fn: Arc<Mutex<Option<Box<UpdateFn>>>>,
    }

    impl MockPackageUpdater {
        pub fn new() -> Self {
            Self {
                update_fn: Arc::new(Mutex::new(None)),
            }
        }

        /// Set up an expectation for the `update` method.
        ///
        /// This method allows you to define the behavior of the mock when `update()` is called.
        /// The provided closure receives the packages and file loader, and should return
        /// the result that you want the mock to produce.
        ///
        /// # Arguments
        ///
        /// * `f` - A closure that takes `Vec<Package>` and `&dyn FileLoader` and returns `Result<Option<Vec<FileChange>>>`.
        /// * This closure can:
        ///   - Return `Ok(Some(changes))` to simulate successful updates
        ///   - Return `Ok(None)` to simulate no changes needed
        ///   - Return `Err(...)` to simulate an error condition
        ///   - Assert on the input parameters to verify test expectations
        ///
        /// # Examples
        ///
        /// Return successful changes:
        /// ```rust,ignore
        /// mock.expect_update(|_packages, _loader| {
        ///     Ok(Some(vec![FileChange { /* ... */ }]))
        /// });
        /// ```
        ///
        /// Return no changes:
        /// ```rust,ignore
        /// mock.expect_update(|_packages, _loader| Ok(None));
        /// ```
        ///
        /// Return an error:
        /// ```rust,ignore
        /// mock.expect_update(|_packages, _loader| {
        ///     Err(eyre::eyre!("Update failed"))
        /// });
        /// ```
        ///
        /// Verify input parameters:
        /// ```rust,ignore
        /// mock.expect_update(|packages, _loader| {
        ///     assert_eq!(packages.len(), 2);
        ///     assert_eq!(packages[0].name, "expected-name");
        ///     Ok(Some(vec![/* ... */]))
        /// });
        /// ```
        pub fn expect_update<F>(&mut self, f: F)
        where
            F: Fn(
                    Vec<Package>,
                    &dyn FileLoader,
                ) -> Result<Option<Vec<FileChange>>>
                + Send
                + Sync
                + 'static,
        {
            let mut guard = self.update_fn.lock().unwrap();
            *guard = Some(Box::new(f));
        }
    }

    #[async_trait]
    impl PackageUpdater for MockPackageUpdater {
        async fn update(
            &self,
            packages: Vec<Package>,
            loader: &dyn FileLoader,
        ) -> Result<Option<Vec<FileChange>>> {
            let guard = self.update_fn.lock().unwrap();
            if let Some(ref f) = *guard {
                f(packages, loader)
            } else {
                panic!(
                    "MockPackageUpdater::update called but no expectation set"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mock::MockPackageUpdater;
    use super::*;
    use crate::analyzer::release::Tag;
    use crate::forge::request::FileUpdateType;
    use crate::forge::traits::MockFileLoader;
    use crate::updater::framework::Framework;
    use semver::Version as SemVer;

    #[tokio::test]
    async fn test_mock_package_updater_returns_changes() {
        let mut mock_updater = MockPackageUpdater::new();
        let mock_loader = MockFileLoader::new();

        let package = Package {
            name: "test-package".to_string(),
            path: "test-path".to_string(),
            framework: Framework::Node,
            next_version: Tag {
                sha: "test-sha".to_string(),
                name: "v1.0.0".to_string(),
                semver: SemVer::parse("1.0.0").unwrap(),
            },
        };

        let expected_change = FileChange {
            path: "test-path/package.json".to_string(),
            content: "test content".to_string(),
            update_type: FileUpdateType::Replace,
        };

        mock_updater.expect_update(move |_packages, _loader| {
            Ok(Some(vec![FileChange {
                path: "test-path/package.json".to_string(),
                content: "test content".to_string(),
                update_type: FileUpdateType::Replace,
            }]))
        });

        let result = mock_updater
            .update(vec![package], &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, expected_change.path);
        assert_eq!(changes[0].content, expected_change.content);
    }

    #[tokio::test]
    async fn test_mock_package_updater_returns_none() {
        let mut mock_updater = MockPackageUpdater::new();
        let mock_loader = MockFileLoader::new();

        let package = Package {
            name: "test-package".to_string(),
            path: "test-path".to_string(),
            framework: Framework::Rust,
            next_version: Tag {
                sha: "test-sha".to_string(),
                name: "v2.0.0".to_string(),
                semver: SemVer::parse("2.0.0").unwrap(),
            },
        };

        mock_updater.expect_update(move |_packages, _loader| Ok(None));

        let result = mock_updater
            .update(vec![package], &mock_loader)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mock_package_updater_multiple_packages() {
        let mut mock_updater = MockPackageUpdater::new();
        let mock_loader = MockFileLoader::new();

        let packages = vec![
            Package {
                name: "package-one".to_string(),
                path: "packages/one".to_string(),
                framework: Framework::Python,
                next_version: Tag {
                    sha: "sha1".to_string(),
                    name: "v1.0.0".to_string(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                },
            },
            Package {
                name: "package-two".to_string(),
                path: "packages/two".to_string(),
                framework: Framework::Python,
                next_version: Tag {
                    sha: "sha2".to_string(),
                    name: "v2.0.0".to_string(),
                    semver: SemVer::parse("2.0.0").unwrap(),
                },
            },
        ];

        mock_updater.expect_update(move |pkgs, _loader| {
            assert_eq!(pkgs.len(), 2);
            Ok(Some(vec![
                FileChange {
                    path: "packages/one/pyproject.toml".to_string(),
                    content: "version = \"1.0.0\"".to_string(),
                    update_type: FileUpdateType::Replace,
                },
                FileChange {
                    path: "packages/two/pyproject.toml".to_string(),
                    content: "version = \"2.0.0\"".to_string(),
                    update_type: FileUpdateType::Replace,
                },
            ]))
        });

        let result = mock_updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_package_updater_with_error() {
        let mut mock_updater = MockPackageUpdater::new();
        let mock_loader = MockFileLoader::new();

        let package = Package {
            name: "test-package".to_string(),
            path: "test-path".to_string(),
            framework: Framework::Java,
            next_version: Tag {
                sha: "test-sha".to_string(),
                name: "v1.0.0".to_string(),
                semver: SemVer::parse("1.0.0").unwrap(),
            },
        };

        mock_updater.expect_update(move |_packages, _loader| {
            Err(color_eyre::eyre::eyre!("Simulated update error"))
        });

        let result = mock_updater.update(vec![package], &mock_loader).await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Simulated update error")
        );
    }

    #[tokio::test]
    async fn test_mock_package_updater_verifies_package_data() {
        let mut mock_updater = MockPackageUpdater::new();
        let mock_loader = MockFileLoader::new();

        let package = Package {
            name: "verify-package".to_string(),
            path: "packages/verify".to_string(),
            framework: Framework::Php,
            next_version: Tag {
                sha: "abc123".to_string(),
                name: "v3.5.0".to_string(),
                semver: SemVer::parse("3.5.0").unwrap(),
            },
        };

        mock_updater.expect_update(move |pkgs, _loader| {
            // Verify package data
            assert_eq!(pkgs.len(), 1);
            assert_eq!(pkgs[0].name, "verify-package");
            assert_eq!(pkgs[0].path, "packages/verify");
            assert_eq!(pkgs[0].next_version.semver.to_string(), "3.5.0");

            Ok(Some(vec![FileChange {
                path: "packages/verify/composer.json".to_string(),
                content: r#"{"version": "3.5.0"}"#.to_string(),
                update_type: FileUpdateType::Replace,
            }]))
        });

        let result = mock_updater
            .update(vec![package], &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
    }
}
