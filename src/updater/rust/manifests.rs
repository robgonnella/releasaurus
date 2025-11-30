use crate::{
    config::package::PackageConfig,
    path_helpers::{package_path, workspace_path},
    updater::{manager::ManifestTarget, traits::ManifestTargets},
};

pub struct RustManifests {}

impl ManifestTargets for RustManifests {
    fn manifest_targets(pkg: &PackageConfig) -> Vec<ManifestTarget> {
        let cargo_toml_pkg_path = package_path(pkg, Some("Cargo.toml"));

        let cargo_toml_wrkspc_path = workspace_path(pkg, Some("Cargo.toml"));

        let is_workspace_pkg = cargo_toml_pkg_path != cargo_toml_wrkspc_path;

        let package_files = vec!["Cargo.toml", "Cargo.lock"];

        let workspace_files = ["Cargo.lock"];

        let mut targets = vec![];

        for file in package_files {
            let full_path = package_path(pkg, Some(file));
            targets.push(ManifestTarget {
                is_workspace: false,
                path: full_path,
                basename: file.to_string(),
            })
        }

        if is_workspace_pkg {
            for file in workspace_files {
                let full_path = workspace_path(pkg, Some(file));

                targets.push(ManifestTarget {
                    is_workspace: true,
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
            release_type: Some(ReleaseType::Rust),
            ..Default::default()
        }
    }

    #[test]
    fn root_package_returns_only_package_manifests() {
        let pkg = create_test_package(".");
        let targets = RustManifests::manifest_targets(&pkg);

        assert_eq!(targets.len(), 2);
        assert!(targets.iter().all(|t| !t.is_workspace));

        let basenames: Vec<_> = targets.iter().map(|t| &t.basename).collect();
        assert!(basenames.contains(&&"Cargo.toml".to_string()));
        assert!(basenames.contains(&&"Cargo.lock".to_string()));
    }

    #[test]
    fn workspace_package_includes_workspace_lock_file() {
        let pkg = create_test_package("crates/my-crate");
        let targets = RustManifests::manifest_targets(&pkg);

        assert_eq!(targets.len(), 3);

        let workspace_targets: Vec<_> =
            targets.iter().filter(|t| t.is_workspace).collect();
        assert_eq!(workspace_targets.len(), 1);
        assert_eq!(workspace_targets[0].basename, "Cargo.lock");
    }

    #[test]
    fn generates_correct_paths_for_workspace_package() {
        let pkg = create_test_package("crates/my-crate");
        let targets = RustManifests::manifest_targets(&pkg);

        let paths: Vec<_> = targets.iter().map(|t| t.path.as_str()).collect();
        assert!(paths.contains(&"crates/my-crate/Cargo.toml"));
        assert!(paths.contains(&"crates/my-crate/Cargo.lock"));
        assert!(paths.contains(&"Cargo.lock"));
    }
}
