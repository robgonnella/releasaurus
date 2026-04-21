use std::path::Path;

use crate::updater::{manager::ManifestTarget, traits::ManifestTargets};

pub struct NodeManifests {}

impl ManifestTargets for NodeManifests {
    fn manifest_targets(
        _pkg_name: &str,
        workspace_path: &Path,
        pkg_path: &Path,
    ) -> Vec<ManifestTarget> {
        let package_json_pkg_path =
            pkg_path.join("package.json").to_string_lossy().to_string();

        let package_json_wrkspc_path = workspace_path
            .join("package.json")
            .to_string_lossy()
            .to_string();

        let is_workspace_pkg =
            package_json_pkg_path != package_json_wrkspc_path;

        let package_files =
            vec!["package.json", "package-lock.json", "yarn.lock"];

        let workspace_files = ["package-lock.json", "yarn.lock"];

        let mut targets = vec![];

        for file in package_files {
            let full_path = pkg_path.join(file);
            targets.push(ManifestTarget {
                path: full_path,
                basename: file.to_string(),
            })
        }

        if is_workspace_pkg {
            for file in workspace_files {
                let full_path = workspace_path.join(file);
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
    use std::path::Path;

    use super::*;

    #[test]
    fn root_package_returns_only_package_manifests() {
        let workspace_path = Path::new("").to_path_buf();
        let pkg_path = workspace_path.clone();

        let targets = NodeManifests::manifest_targets(
            "tstpkg",
            &workspace_path,
            &pkg_path,
        );

        assert_eq!(targets.len(), 3);

        let basenames: Vec<_> = targets.iter().map(|t| &t.basename).collect();

        assert!(basenames.contains(&&"package.json".to_string()));
        assert!(basenames.contains(&&"package-lock.json".to_string()));
        assert!(basenames.contains(&&"yarn.lock".to_string()));
    }

    #[test]
    fn workspace_package_includes_workspace_lock_files() {
        let workspace_path = Path::new("").to_path_buf();
        let pkg_path = Path::new("packages/my-app").to_path_buf();

        let targets = NodeManifests::manifest_targets(
            "tstpkg",
            &workspace_path,
            &pkg_path,
        );

        assert_eq!(targets.len(), 5);
    }

    #[test]
    fn generates_correct_paths_for_workspace_package() {
        let workspace_path = Path::new("").to_path_buf();
        let pkg_path = Path::new("packages/my-app").to_path_buf();

        let targets = NodeManifests::manifest_targets(
            "tstpkg",
            &workspace_path,
            &pkg_path,
        );

        let paths: Vec<_> =
            targets.iter().map(|t| t.path.to_str().unwrap()).collect();

        assert!(paths.contains(&"packages/my-app/package.json"));
        assert!(paths.contains(&"packages/my-app/package-lock.json"));
        assert!(paths.contains(&"packages/my-app/yarn.lock"));
        assert!(paths.contains(&"package-lock.json"));
        assert!(paths.contains(&"yarn.lock"));
    }
}
