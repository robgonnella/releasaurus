use crate::{
    config::package::PackageConfig,
    path_helpers::{package_path, workspace_path},
    updater::{manager::ManifestTarget, traits::ManifestTargets},
};

pub struct NodeManifests {}

impl ManifestTargets for NodeManifests {
    fn manifest_targets(pkg: &PackageConfig) -> Vec<ManifestTarget> {
        let package_json_pkg_path = package_path(pkg, Some("package.json"));

        let package_json_wrkspc_path =
            workspace_path(pkg, Some("package.json"));

        let is_workspace_pkg =
            package_json_pkg_path != package_json_wrkspc_path;

        let package_files =
            vec!["package.json", "package-lock.json", "yarn.lock"];

        let workspace_files = ["package-lock.json", "yarn.lock"];

        let mut targets = vec![];

        for file in package_files {
            let full_path = package_path(pkg, Some(file));
            targets.push(ManifestTarget {
                path: full_path,
                basename: file.to_string(),
            })
        }

        if is_workspace_pkg {
            for file in workspace_files {
                let full_path = workspace_path(pkg, Some(file));

                targets.push(ManifestTarget {
                    path: full_path,
                    basename: file.to_string(),
                })
            }
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
            release_type: Some(ReleaseType::Node),
            ..Default::default()
        }
    }

    #[test]
    fn root_package_returns_only_package_manifests() {
        let pkg = create_test_package(".");
        let targets = NodeManifests::manifest_targets(&pkg);

        assert_eq!(targets.len(), 3);

        let basenames: Vec<_> = targets.iter().map(|t| &t.basename).collect();
        assert!(basenames.contains(&&"package.json".to_string()));
        assert!(basenames.contains(&&"package-lock.json".to_string()));
        assert!(basenames.contains(&&"yarn.lock".to_string()));
    }

    #[test]
    fn workspace_package_includes_workspace_lock_files() {
        let pkg = create_test_package("packages/my-app");
        let targets = NodeManifests::manifest_targets(&pkg);
        assert_eq!(targets.len(), 5);
    }

    #[test]
    fn generates_correct_paths_for_workspace_package() {
        let pkg = create_test_package("packages/my-app");
        let targets = NodeManifests::manifest_targets(&pkg);

        let paths: Vec<_> = targets.iter().map(|t| t.path.as_str()).collect();
        assert!(paths.contains(&"packages/my-app/package.json"));
        assert!(paths.contains(&"packages/my-app/package-lock.json"));
        assert!(paths.contains(&"packages/my-app/yarn.lock"));
        assert!(paths.contains(&"package-lock.json"));
        assert!(paths.contains(&"yarn.lock"));
    }
}
