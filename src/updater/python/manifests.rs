use crate::{
    config::package::PackageConfig,
    path_helpers::package_path,
    updater::{manager::ManifestTarget, traits::ManifestTargets},
};

pub struct PythonManifests {}

impl ManifestTargets for PythonManifests {
    fn manifest_targets(pkg: &PackageConfig) -> Vec<ManifestTarget> {
        let files = vec!["pyproject.toml", "setup.cfg", "setup.py"];

        let mut targets = vec![];

        for file in files {
            targets.push(ManifestTarget {
                path: package_path(pkg, Some(file)),
                basename: file.into(),
            })
        }

        targets
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
            release_type: Some(ReleaseType::Python),
            ..Default::default()
        }
    }

    #[test]
    fn returns_all_python_manifest_targets() {
        let pkg = create_test_package(".");
        let targets = PythonManifests::manifest_targets(&pkg);

        assert_eq!(targets.len(), 3);

        let basenames: Vec<_> = targets.iter().map(|t| &t.basename).collect();
        assert!(basenames.contains(&&"pyproject.toml".to_string()));
        assert!(basenames.contains(&&"setup.cfg".to_string()));
        assert!(basenames.contains(&&"setup.py".to_string()));
    }

    #[test]
    fn generates_correct_paths_for_nested_package() {
        let pkg = create_test_package("packages/my-python-lib");
        let targets = PythonManifests::manifest_targets(&pkg);

        let paths: Vec<_> = targets.iter().map(|t| t.path.as_str()).collect();
        assert!(paths.contains(&"packages/my-python-lib/pyproject.toml"));
        assert!(paths.contains(&"packages/my-python-lib/setup.cfg"));
        assert!(paths.contains(&"packages/my-python-lib/setup.py"));
    }
}
