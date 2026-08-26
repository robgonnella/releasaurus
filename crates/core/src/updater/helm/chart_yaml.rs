use regex::Regex;
use std::sync::LazyLock;

use crate::{
    config::release_type::ReleaseType,
    forge::request::FileChange,
    packages::manifests::ManifestFile,
    result::Result,
    updater::{generic::updater::GenericUpdater, traits::FileUpdater},
};

/// Helm-specific version regex matching only the chart's own `version`
/// key. Anchoring to column 0 is what keeps it off the two other version
/// fields a Chart.yaml carries: `appVersion` does not start the line, and
/// a `dependencies[].version` is always indented under its list item.
/// GENERIC_VERSION_REGEX leads with `.*` and would rewrite all three to
/// the chart version.
static HELM_CHART_VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?m)^(?<start>version:[ \t]*['"]?)(?<version>\d+\.\d+\.\d+[^'"\s#]*)(?<end>.*)$"#,
    )
    .unwrap()
});

/// Helm-specific version regex matching both the chart's own `version` and
/// `appVersion` keys. Anchoring to column 0 is what keeps it off
/// `dependencies[].version`, which is always indented under its list item.
/// GENERIC_VERSION_REGEX leads with `.*` and would rewrite all three to
/// the chart version.
static HELM_CHART_COMBINED_VERSION_REGEX: LazyLock<Regex> = LazyLock::new(
    || {
        Regex::new(
          r#"(?m)^(?<start>(?:version|appVersion):[ \t]*['"]?)(?<version>\d+\.\d+\.\d+[^'"\s#]*)(?<end>.*)$"#,
    )
    .unwrap()
    },
);

/// Handles Chart.yaml file parsing and version updates for Helm charts.
pub struct ChartYaml {}

impl ChartYaml {
    /// Create Chart.yaml handler for chart version updates.
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ChartYaml {
    fn default() -> Self {
        ChartYaml::new()
    }
}

impl FileUpdater for ChartYaml {
    /// Process Chart.yaml files for all Helm packages.
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if manifest.basename != "Chart.yaml"
            || !matches!(manifest.release_type, ReleaseType::Helm(_))
        {
            return Ok(None);
        }

        let version_regex = if let ReleaseType::Helm(conf) =
            manifest.release_type
            && conf.update_app_version
        {
            &HELM_CHART_COMBINED_VERSION_REGEX
        } else {
            &HELM_CHART_VERSION_REGEX
        };

        Ok(GenericUpdater::update_manifest(manifest, version_regex))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use semver::Version;

    use crate::{
        config::release_type::{HelmConfig, ReleaseType},
        forge::request::{Tag, TagVersion},
        packages::manifests::{ManifestFile, ManifestPackage},
    };

    use super::*;

    /// A chart whose `version`, `appVersion` and dependency pin all share
    /// one value. Only the first is ours to write, so a pattern that is
    /// too loose rewrites all three and no assertion on `version` alone
    /// would catch it.
    const CHART: &str = r#"apiVersion: v2
name: my-chart
description: A Helm chart
type: application
version: 1.0.0
appVersion: 1.0.0
dependencies:
  - name: redis
    version: 1.0.0
    repository: https://charts.example.com
"#;

    fn manifest(basename: &str, content: &str, owned: bool) -> ManifestFile {
        let owner = ManifestPackage {
            name: "my-chart".into(),
            release_type: ReleaseType::Helm(HelmConfig::default()),
            tag: Tag {
                name: "v2.0.0".into(),
                version: TagVersion::Semantic(Version::new(2, 0, 0)),
                sha: "abc".into(),
                ..Default::default()
            },
        };

        ManifestFile {
            path: Path::new(basename).to_path_buf(),
            basename: basename.to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Helm(HelmConfig::default()),
            owner: owned.then_some(owner),
            releasing: vec![],
        }
    }

    fn update(content: &str) -> String {
        update_with(HelmConfig::default(), content)
    }

    fn update_with(config: HelmConfig, content: &str) -> String {
        let mut manifest = manifest("Chart.yaml", content, true);
        manifest.release_type = ReleaseType::Helm(config);

        ChartYaml::new()
            .update(&manifest)
            .unwrap()
            .expect("chart version was rewritten")
            .content
    }

    const WITH_APP_VERSION: HelmConfig = HelmConfig {
        update_app_version: true,
    };

    #[test]
    fn updates_chart_version() {
        assert!(update(CHART).contains("\nversion: 2.0.0\n"));
    }

    #[test]
    fn leaves_app_version_alone() {
        assert!(update(CHART).contains("\nappVersion: 1.0.0\n"));
    }

    #[test]
    fn leaves_dependency_pins_alone() {
        assert!(update(CHART).contains("\n    version: 1.0.0\n"));
    }

    /// The three assertions above each pass on their own if the pattern
    /// matched some *other* line too, so pin the total: exactly one field
    /// in the file may carry the new version.
    #[test]
    fn writes_the_new_version_exactly_once() {
        assert_eq!(update(CHART).matches("2.0.0").count(), 1);
    }

    #[test]
    fn preserves_quotes_and_trailing_comments() {
        let content =
            "version: \"1.0.0\" # the chart version\nappVersion: 1.0.0\n";

        assert!(
            update(content).contains("version: \"2.0.0\" # the chart version")
        );
    }

    /// `replacen` rewrites the first occurrence of the version text inside
    /// the whole match, so a comment repeating the old version must not
    /// absorb the substitution.
    #[test]
    fn rewrites_the_field_not_a_comment_repeating_it() {
        let updated = update("version: 1.0.0 # was 1.0.0\n");

        assert_eq!(updated, "version: 2.0.0 # was 1.0.0\n");
    }

    #[test]
    fn updates_a_prerelease_version() {
        let updated = update("version: 1.0.0-rc.1+build.5\n");

        assert_eq!(updated, "version: 2.0.0\n");
    }

    #[test]
    fn preserves_surrounding_content_and_trailing_newline() {
        let updated = update(CHART);

        assert!(updated.starts_with("apiVersion: v2\n"));
        assert!(updated.contains("description: A Helm chart"));
        assert!(updated.ends_with("repository: https://charts.example.com\n"));
    }

    /// A Chart.yaml no releasing package owns has no version of ours to
    /// write.
    #[test]
    fn returns_none_when_unowned() {
        let chart_yaml = ChartYaml::new();
        let manifest = manifest("Chart.yaml", CHART, false);

        assert!(chart_yaml.update(&manifest).unwrap().is_none());
    }

    #[test]
    fn returns_none_for_another_basename() {
        let chart_yaml = ChartYaml::new();
        let manifest = manifest("values.yaml", "version: 1.0.0\n", true);

        assert!(chart_yaml.update(&manifest).unwrap().is_none());
    }

    #[test]
    fn returns_none_when_chart_has_no_version_field() {
        let chart_yaml = ChartYaml::new();
        let manifest =
            manifest("Chart.yaml", "apiVersion: v2\nname: my-chart\n", true);

        assert!(chart_yaml.update(&manifest).unwrap().is_none());
    }

    #[test]
    fn updates_app_version_when_enabled() {
        let updated = update_with(WITH_APP_VERSION, CHART);

        assert!(updated.contains("\nversion: 2.0.0\n"));
        assert!(updated.contains("\nappVersion: 2.0.0\n"));
        assert!(updated.contains("\n    version: 1.0.0\n"));
        assert_eq!(updated.matches("2.0.0").count(), 2);
    }

    /// `appVersion` receives the release version, not a bump of its own
    /// value.
    #[test]
    fn app_version_takes_the_release_version_not_its_own() {
        let updated = update_with(
            WITH_APP_VERSION,
            "version: 1.0.0\nappVersion: 4.5.6\n",
        );

        assert_eq!(updated, "version: 2.0.0\nappVersion: 2.0.0\n");
    }

    #[test]
    fn app_version_preserves_quotes_and_trailing_comments() {
        let updated = update_with(
            WITH_APP_VERSION,
            "version: 1.0.0\nappVersion: \"1.0.0\" # the app\n",
        );

        assert_eq!(
            updated,
            "version: 2.0.0\nappVersion: \"2.0.0\" # the app\n"
        );
    }

    /// Only a bare `X.Y.Z` value is rewritten: anything else is left as-is.
    #[test]
    fn leaves_non_semver_app_version_alone() {
        for app_version in [r#""v1.0.0""#, r#""1.16""#, "latest"] {
            let content =
                format!("version: 1.0.0\nappVersion: {app_version}\n");

            assert_eq!(
                update_with(WITH_APP_VERSION, &content),
                format!("version: 2.0.0\nappVersion: {app_version}\n")
            );
        }
    }

    /// Key names: only `version` and `appVersion`, exactly as spelled.
    #[test]
    fn leaves_other_version_keys_alone_when_enabled() {
        let content = "apiVersion: 1.0.0\nkubeVersion: 1.0.0\n\
                       Version: 1.0.0\nappversion: 1.0.0\n\
                       version: 1.0.0\nappVersion: 1.0.0\n";

        let updated = update_with(WITH_APP_VERSION, content);

        assert_eq!(
            updated,
            "apiVersion: 1.0.0\nkubeVersion: 1.0.0\n\
             Version: 1.0.0\nappversion: 1.0.0\n\
             version: 2.0.0\nappVersion: 2.0.0\n"
        );
    }

    #[test]
    fn returns_none_for_another_release_type() {
        let chart_yaml = ChartYaml::new();
        let mut manifest = manifest("Chart.yaml", CHART, true);
        manifest.release_type = ReleaseType::Generic;

        assert!(chart_yaml.update(&manifest).unwrap().is_none());
    }
}
