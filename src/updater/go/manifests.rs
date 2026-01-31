use std::path::Path;

use crate::updater::{manager::ManifestTarget, traits::ManifestTargets};

pub struct GoManifests {}

impl ManifestTargets for GoManifests {
    fn manifest_targets(
        _pkg_name: &str,
        _workspace_path: &Path,
        pkg_path: &Path,
    ) -> Vec<ManifestTarget> {
        vec![
            ManifestTarget {
                path: pkg_path.join("version.go"),
                basename: "version.go".into(),
            },
            ManifestTarget {
                path: pkg_path.join("version/version.go"),
                basename: "version.go".into(),
            },
            ManifestTarget {
                path: pkg_path.join("internal/version.go"),
                basename: "version.go".into(),
            },
            ManifestTarget {
                path: pkg_path.join("internal/version/version.go"),
                basename: "version.go".into(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn returns_all_golang_manifest_targets() {
        let pkg_name = "gopher";
        let workspace_path = Path::new("").to_path_buf();
        let pkg_path = workspace_path.clone();

        let targets =
            GoManifests::manifest_targets(pkg_name, &workspace_path, &pkg_path);

        assert_eq!(targets.len(), 4);

        let basenames: Vec<_> = targets.iter().map(|t| &t.basename).collect();
        assert_eq!(basenames.iter().filter(|b| **b == "version.go").count(), 4);

        let paths: Vec<_> =
            targets.iter().map(|t| t.path.to_str().unwrap()).collect();

        assert!(paths.contains(&"version.go"));
        assert!(paths.contains(&"version/version.go"));
        assert!(paths.contains(&"internal/version.go"));
        assert!(paths.contains(&"internal/version/version.go"));
    }

    #[test]
    fn generates_correct_paths_for_nested_package() {
        let pkg_name = "gopher";
        let workspace_path = Path::new("").to_path_buf();
        let pkg_path = Path::new("pkg").to_path_buf();

        let targets =
            GoManifests::manifest_targets(pkg_name, &workspace_path, &pkg_path);

        let basenames: Vec<_> = targets.iter().map(|t| &t.basename).collect();
        assert_eq!(basenames.iter().filter(|b| **b == "version.go").count(), 4);

        let paths: Vec<_> =
            targets.iter().map(|t| t.path.to_str().unwrap()).collect();

        assert!(paths.contains(&"pkg/version.go"));
        assert!(paths.contains(&"pkg/version/version.go"));
        assert!(paths.contains(&"pkg/internal/version.go"));
        assert!(paths.contains(&"pkg/internal/version/version.go"));
    }
}
