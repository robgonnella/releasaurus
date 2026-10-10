use crate::{
    config::release_type::ReleaseType,
    forge::request::FileChange,
    packages::manifests::ManifestFile,
    result::Result,
    updater::{helm::chart_yaml::ChartYaml, traits::FileUpdater},
};

/// Helm chart updater for Chart.yaml files.
pub struct HelmUpdater {
    chart_yaml: ChartYaml,
}

impl HelmUpdater {
    /// Create Helm updater for Chart.yaml version updates.
    pub fn new() -> Self {
        Self {
            chart_yaml: ChartYaml::new(),
        }
    }
}

impl Default for HelmUpdater {
    fn default() -> Self {
        HelmUpdater::new()
    }
}

impl FileUpdater for HelmUpdater {
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if !matches!(manifest.release_type, ReleaseType::Helm) {
            return Ok(None);
        }
        self.chart_yaml.update(manifest)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use semver::Version;

    use crate::{
        config::release_type::ReleaseType,
        forge::request::Tag,
        packages::manifests::{ManifestFile, ManifestPackage},
    };

    use super::*;

    fn manifest(basename: &str, release_type: ReleaseType) -> ManifestFile {
        ManifestFile {
            path: Path::new(basename).to_path_buf(),
            basename: basename.to_string(),
            content: "version: 1.0.0\n".to_string(),
            release_type,
            owner: Some(ManifestPackage {
                name: "my-chart".into(),
                release_type,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        }
    }

    #[test]
    fn processes_helm_chart() {
        let updater = HelmUpdater::new();

        let result = updater
            .update(&manifest("Chart.yaml", ReleaseType::Helm))
            .unwrap();

        assert_eq!(result.unwrap().content, "version: 2.0.0\n");
    }

    #[test]
    fn returns_none_when_no_chart_files() {
        let updater = HelmUpdater::new();

        let result = updater
            .update(&manifest("values.yaml", ReleaseType::Helm))
            .unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn returns_none_for_another_release_type() {
        let updater = HelmUpdater::new();

        let result = updater
            .update(&manifest("Chart.yaml", ReleaseType::Node))
            .unwrap();

        assert!(result.is_none());
    }
}
