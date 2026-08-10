use std::path::Path;

use crate::updater::{manager::ManifestTarget, traits::ManifestTargets};

pub struct RustManifests {}

impl ManifestTargets for RustManifests {
    fn manifest_targets(
        _pkg_name: &str,
        workspace_path: &Path,
        pkg_path: &Path,
    ) -> Vec<ManifestTarget> {
        let rust_manifests = vec!["Cargo.toml", "Cargo.lock"];

        let mut targets = vec![];

        // potential package manifests
        for file in rust_manifests.iter() {
            let full_path = pkg_path.join(file);
            targets.push(ManifestTarget {
                path: full_path,
                basename: file.to_string(),
            })
        }

        // potential workspace manifests
        if workspace_path != pkg_path {
            for file in rust_manifests {
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

        let targets = RustManifests::manifest_targets(
            "tstpkg",
            &workspace_path,
            &pkg_path,
        );

        assert_eq!(targets.len(), 2);

        let basenames: Vec<_> = targets.iter().map(|t| &t.basename).collect();
        assert!(basenames.contains(&&"Cargo.toml".to_string()));
        assert!(basenames.contains(&&"Cargo.lock".to_string()));
    }

    #[test]
    fn workspace_package_includes_workspace_lock_file() {
        let workspace_path = Path::new("").to_path_buf();
        let pkg_path = Path::new("crates/my-crate").to_path_buf();

        let targets = RustManifests::manifest_targets(
            "tstpkg",
            &workspace_path,
            &pkg_path,
        );

        assert_eq!(targets.len(), 4);
    }

    #[test]
    fn generates_correct_paths_for_workspace_package() {
        let workspace_path = Path::new("").to_path_buf();
        let pkg_path = Path::new("crates/my-crate").to_path_buf();

        let targets = RustManifests::manifest_targets(
            "tstpkg",
            &workspace_path,
            &pkg_path,
        );

        let paths: Vec<_> =
            targets.iter().map(|t| t.path.to_str().unwrap()).collect();

        assert!(paths.contains(&"crates/my-crate/Cargo.toml"));
        assert!(paths.contains(&"crates/my-crate/Cargo.lock"));
        assert!(paths.contains(&"Cargo.toml"));
        assert!(paths.contains(&"Cargo.lock"));
    }
}
