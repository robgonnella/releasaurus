use std::path::Path;

use crate::updater::{manager::ManifestTarget, traits::ManifestTargets};

pub struct PythonManifests {}

impl ManifestTargets for PythonManifests {
    fn manifest_targets(
        _pkg_name: &str,
        _workspace_path: &Path,
        pkg_path: &Path,
    ) -> Vec<ManifestTarget> {
        let files = vec!["pyproject.toml", "setup.cfg", "setup.py"];

        let mut targets = vec![];

        for file in files {
            targets.push(ManifestTarget {
                path: pkg_path.join(file),
                basename: file.into(),
            })
        }

        targets
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn returns_all_python_manifest_targets() {
        let workspace_path = Path::new("").to_path_buf();
        let pkg_path = workspace_path.clone();

        let targets = PythonManifests::manifest_targets(
            "tstpkg",
            &workspace_path,
            &pkg_path,
        );

        assert_eq!(targets.len(), 3);

        let basenames: Vec<_> = targets.iter().map(|t| &t.basename).collect();
        assert!(basenames.contains(&&"pyproject.toml".to_string()));
        assert!(basenames.contains(&&"setup.cfg".to_string()));
        assert!(basenames.contains(&&"setup.py".to_string()));
    }

    #[test]
    fn generates_correct_paths_for_nested_package() {
        let workspace_path = Path::new("").to_path_buf();
        let pkg_path = Path::new("packages/my-python-lib").to_path_buf();

        let targets = PythonManifests::manifest_targets(
            "tstpkg",
            &workspace_path,
            &pkg_path,
        );

        let paths: Vec<_> =
            targets.iter().map(|t| t.path.to_str().unwrap()).collect();

        assert!(paths.contains(&"packages/my-python-lib/pyproject.toml"));
        assert!(paths.contains(&"packages/my-python-lib/setup.cfg"));
        assert!(paths.contains(&"packages/my-python-lib/setup.py"));
    }
}
