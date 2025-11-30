use crate::{
    config::package::PackageConfig,
    path_helpers::package_path,
    updater::{manager::ManifestTarget, traits::ManifestTargets},
};

pub struct PhpManifests {}

impl ManifestTargets for PhpManifests {
    fn manifest_targets(pkg: &PackageConfig) -> Vec<ManifestTarget> {
        vec![ManifestTarget {
            is_workspace: false,
            path: package_path(pkg, Some("composer.json")),
            basename: "composer.json".into(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{package::PackageConfig, release_type::ReleaseType};

    fn create_test_package(path: &str) -> PackageConfig {
        PackageConfig {
            name: "test-package".to_string(),
            workspace_root: ".".to_string(),
            path: path.to_string(),
            release_type: Some(ReleaseType::Php),
            ..Default::default()
        }
    }

    #[test]
    fn returns_composer_json_manifest() {
        let pkg = create_test_package(".");
        let targets = PhpManifests::manifest_targets(&pkg);

        assert_eq!(targets.len(), 1);
        assert!(!targets[0].is_workspace);
        assert_eq!(targets[0].basename, "composer.json");
        assert_eq!(targets[0].path, "composer.json");
    }

    #[test]
    fn generates_correct_path_for_nested_package() {
        let pkg = create_test_package("packages/my-php-lib");
        let targets = PhpManifests::manifest_targets(&pkg);

        assert_eq!(targets[0].path, "packages/my-php-lib/composer.json");
    }
}
