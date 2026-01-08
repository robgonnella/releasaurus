use std::path::Path;

use crate::updater::{manager::ManifestTarget, traits::ManifestTargets};

pub struct PhpManifests {}

impl ManifestTargets for PhpManifests {
    fn manifest_targets(
        _pkg_name: &str,
        _workspace_path: &Path,
        pkg_path: &Path,
    ) -> Vec<ManifestTarget> {
        vec![ManifestTarget {
            path: pkg_path.join("composer.json"),
            basename: "composer.json".into(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn returns_composer_json_manifest() {
        let workspace_path = Path::new("").to_path_buf();
        let pkg_path = workspace_path.clone();

        let targets = PhpManifests::manifest_targets(
            "tstpkg",
            &workspace_path,
            &pkg_path,
        );

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].basename, "composer.json");
        assert_eq!(targets[0].path.to_string_lossy(), "composer.json");
    }

    #[test]
    fn generates_correct_path_for_nested_package() {
        let workspace_path = Path::new("").to_path_buf();
        let pkg_path = Path::new("packages/my-php-lib").to_path_buf();

        let targets = PhpManifests::manifest_targets(
            "tstpkg",
            &workspace_path,
            &pkg_path,
        );

        assert_eq!(
            targets[0].path.to_string_lossy(),
            "packages/my-php-lib/composer.json"
        );
    }
}
