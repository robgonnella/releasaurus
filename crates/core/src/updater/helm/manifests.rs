use std::path::Path;

use crate::updater::{manager::ManifestTarget, traits::ManifestTargets};

pub struct HelmManifests {}

impl ManifestTargets for HelmManifests {
    /// Nothing reaches up to the workspace root: a chart carries only its
    /// own version, so an umbrella chart above this package has no field
    /// for us to write.
    fn manifest_targets(
        _pkg_name: &str,
        _workspace_path: &Path,
        pkg_path: &Path,
    ) -> Vec<ManifestTarget> {
        vec![ManifestTarget {
            path: pkg_path.join("Chart.yaml"),
            basename: "Chart.yaml".into(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn returns_chart_yaml_manifest() {
        let workspace_path = Path::new("").to_path_buf();
        let pkg_path = workspace_path.clone();

        let targets = HelmManifests::manifest_targets(
            "tstpkg",
            &workspace_path,
            &pkg_path,
        );

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].basename, "Chart.yaml");
        assert_eq!(targets[0].path.to_string_lossy(), "Chart.yaml");
    }

    #[test]
    fn generates_correct_path_for_nested_package() {
        let workspace_path = Path::new("").to_path_buf();
        let pkg_path = Path::new("charts/my-chart").to_path_buf();

        let targets = HelmManifests::manifest_targets(
            "tstpkg",
            &workspace_path,
            &pkg_path,
        );

        assert_eq!(
            targets[0].path.to_string_lossy(),
            "charts/my-chart/Chart.yaml"
        );
    }
}
