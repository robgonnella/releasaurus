//! Common functionality shared between release commands
use std::path::Path;

use crate::{
    analyzer::config::AnalyzerConfig,
    config::{Config, PackageConfig},
    forge::config::RemoteConfig,
};

pub fn process_config(repo_name: &str, config: &mut Config) -> Config {
    for package in config.packages.iter_mut() {
        package.name = derive_package_name(package, repo_name);
    }
    // drop mutability
    config.clone()
}

/// Resolve tag prefix for package from config or generate default based on
/// package path for monorepo support.
pub fn get_tag_prefix(package: &PackageConfig, repo_name: &str) -> String {
    let mut default_for_package = "v".to_string();
    let name = derive_package_name(package, repo_name);
    if package.path != "." {
        default_for_package = format!("{}-v", name);
    }
    package.tag_prefix.clone().unwrap_or(default_for_package)
}

/// Generates [`AnalyzerConfig`] from [`Config`], [`RemoteConfig`],
/// and tag_prefix [`String`]
pub fn generate_analyzer_config(
    config: &Config,
    remote_config: &RemoteConfig,
    tag_prefix: String,
) -> AnalyzerConfig {
    AnalyzerConfig {
        body: config.changelog.body.clone(),
        include_author: config.changelog.include_author,
        skip_chore: config.changelog.skip_chore,
        skip_ci: config.changelog.skip_ci,
        skip_miscellaneous: config.changelog.skip_miscellaneous,
        release_link_base_url: remote_config.release_link_base_url.clone(),
        tag_prefix: Some(tag_prefix),
    }
}

/// Extract package name from its path, using repository name for root
/// packages.
pub fn derive_package_name(package: &PackageConfig, repo_name: &str) -> String {
    if !package.name.is_empty() {
        return package.name.to_string();
    }

    let path = Path::new(&package.path);

    if let Some(name) = path.file_name() {
        return name.display().to_string();
    }

    if package.path == "." {
        // For root package, use repository directory name as fallback
        repo_name.into()
    } else {
        // Extract name from path (e.g., "crates/my-package" -> "my-package")
        package
            .path
            .split('/')
            .next_back()
            .unwrap_or(&package.path)
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::{config::ReleaseType, test_helpers};

    use super::*;

    #[test]
    fn test_get_tag_prefix_using_standard_default() {
        let repo_name = "test-repo";

        let package = test_helpers::create_test_package_config(
            "my-package",
            ".",
            Some(ReleaseType::Generic),
            None,
        );

        let tag_prefix = get_tag_prefix(&package, repo_name);

        assert_eq!(tag_prefix, "v");
    }

    #[test]
    fn test_get_tag_prefix_using_name() {
        let repo_name = "test-repo";

        let package = test_helpers::create_test_package_config(
            "my-package",
            "packages/my-package",
            Some(ReleaseType::Generic),
            None,
        );

        let tag_prefix = get_tag_prefix(&package, repo_name);

        assert_eq!(tag_prefix, "my-package-v");
    }

    #[test]
    fn test_get_tag_prefix_using_path() {
        let repo_name = "test-repo";

        let package = test_helpers::create_test_package_config(
            "",
            "packages/my-package",
            Some(ReleaseType::Generic),
            None,
        );

        let tag_prefix = get_tag_prefix(&package, repo_name);

        assert_eq!(tag_prefix, "my-package-v");
    }

    #[test]
    fn test_get_tag_prefix_using_configured_prefix() {
        let repo_name = "test-repo";

        let package = test_helpers::create_test_package_config(
            "my-package",
            "packages/my-package",
            Some(ReleaseType::Generic),
            Some("my-special-tag-prefix-v".into()),
        );

        let tag_prefix = get_tag_prefix(&package, repo_name);

        assert_eq!(tag_prefix, "my-special-tag-prefix-v");
    }

    #[test]
    fn test_derive_package_name_from_directory() {
        let mut package = test_helpers::create_test_package_config(
            "",
            "packages/my-package",
            Some(ReleaseType::Generic),
            Some("v".into()),
        );
        // Test with simple directory name
        let name = derive_package_name(&package, "test-repo");
        assert_eq!(name, "my-package");

        // Test with nested path
        package.name = "".into();
        package.path = "crates/core/utils".into();
        let name = derive_package_name(&package, "test-repo");
        assert_eq!(name, "utils");

        // Test with root path
        package.name = "".into();
        package.path = ".".into();
        let name = derive_package_name(&package, "test-repo");
        assert_eq!(name, "test-repo");

        // Test with single directory
        package.name = "".into();
        package.path = "backend".into();
        let name = derive_package_name(&package, "test-repo");
        assert_eq!(name, "backend");
    }

    #[test]
    fn test_process_config_derives_package_names() {
        let repo_name = "test-repo";

        let mut config = test_helpers::create_test_config(vec![
            test_helpers::create_test_package_config(
                "",
                ".",
                Some(ReleaseType::Generic),
                None,
            ),
            test_helpers::create_test_package_config(
                "",
                "packages/api",
                Some(ReleaseType::Node),
                None,
            ),
        ]);

        let processed = process_config(repo_name, &mut config);

        assert_eq!(processed.packages[0].name, "test-repo");
        assert_eq!(processed.packages[1].name, "api");
    }

    #[test]
    fn test_process_config_preserves_existing_names() {
        let repo_name = "test-repo";

        let mut config = test_helpers::create_test_config(vec![
            test_helpers::create_test_package_config(
                "my-custom-name",
                ".",
                Some(ReleaseType::Generic),
                None,
            ),
            test_helpers::create_test_package_config(
                "another-name",
                "packages/api",
                Some(ReleaseType::Node),
                None,
            ),
        ]);

        let processed = process_config(repo_name, &mut config);

        assert_eq!(processed.packages[0].name, "my-custom-name");
        assert_eq!(processed.packages[1].name, "another-name");
    }

    #[test]
    fn test_process_config_mixed_names() {
        let repo_name = "test-repo";

        let mut config = test_helpers::create_test_config(vec![
            test_helpers::create_test_package_config(
                "explicit-name",
                "packages/frontend",
                Some(ReleaseType::Generic),
                None,
            ),
            test_helpers::create_test_package_config(
                "",
                "packages/backend",
                Some(ReleaseType::Node),
                None,
            ),
        ]);

        let processed = process_config(repo_name, &mut config);

        assert_eq!(processed.packages[0].name, "explicit-name");
        assert_eq!(processed.packages[1].name, "backend");
    }

    #[test]
    fn test_derive_package_name_with_explicit_name() {
        let package = test_helpers::create_test_package_config(
            "explicit-package-name",
            "packages/something",
            Some(ReleaseType::Generic),
            None,
        );

        let name = derive_package_name(&package, "test-repo");
        assert_eq!(name, "explicit-package-name");
    }
}
