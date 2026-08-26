//! Helm scenarios. A `Chart.yaml` carries three version-shaped fields and
//! only one of them is the chart's own, so these pin what is *not* written
//! as tightly as what is: `appVersion` names an application that commonly
//! lives in another repo, and a `dependencies` pin names a chart version
//! we do not resolve.

use super::common::*;

/// A chart whose three version fields all start equal. Any pattern loose
/// enough to reach `appVersion` or the dependency pin rewrites them to the
/// chart's new version, and the file still parses, so nothing but an
/// explicit assertion catches it.
const CHART: &str = r#"apiVersion: v2
name: my-chart
description: A Helm chart
type: application
version: 0.0.1
appVersion: 0.0.1
dependencies:
  - name: redis
    version: 0.0.1
    repository: https://charts.example.com
"#;

#[tokio::test]
async fn writes_the_chart_version_and_nothing_else() {
    let scenario = Scenario::new(&[
        (
            "releasaurus.toml",
            r#"
[[package]]
name = "my-chart"
path = "."
release_type = "helm"
"#,
        ),
        ("Chart.yaml", CHART),
    ])
    .await
    .unwrap();

    let changes = scenario.file_changes().await.unwrap();
    let updated = &change(&changes, "Chart.yaml").content;

    assert!(updated.contains("\nversion: 0.1.0\n"), "{updated}");
    assert!(updated.contains("\nappVersion: 0.0.1\n"), "{updated}");
    assert!(updated.contains("\n    version: 0.0.1\n"), "{updated}");
    assert_eq!(updated.matches("0.1.0").count(), 1, "{updated}");
}

/// Two charts on one release branch, each versioned from its own history.
/// Nothing reaches outside a package directory, so neither chart's bump can
/// land in the other's file.
#[tokio::test]
async fn each_chart_is_written_from_its_own_directory() {
    let scenario = Scenario::new(&[
        (
            "releasaurus.toml",
            r#"
[[package]]
name = "chart-a"
path = "charts/a"
release_type = "helm"

[[package]]
name = "chart-b"
path = "charts/b"
release_type = "helm"
"#,
        ),
        ("charts/a/Chart.yaml", CHART),
        ("charts/b/Chart.yaml", CHART),
    ])
    .await
    .unwrap();

    let changes = scenario.file_changes().await.unwrap();

    for path in ["charts/a/Chart.yaml", "charts/b/Chart.yaml"] {
        let updated = &change(&changes, path).content;

        assert!(updated.contains("\nversion: 0.1.0\n"), "{path}: {updated}");
        assert!(
            updated.contains("\nappVersion: 0.0.1\n"),
            "{path}: {updated}"
        );
    }
}

/// Comments and quoting are what a regex-based rewrite most easily loses,
/// and a chart is the manifest most likely to carry them.
#[tokio::test]
async fn preserves_comments_and_quoting() {
    let chart = r#"# The chart that does the thing.
apiVersion: v2
name: my-chart
version: "0.0.1" # bumped on release
appVersion: "0.0.1"
"#;

    let scenario = Scenario::new(&[
        (
            "releasaurus.toml",
            r#"
[[package]]
name = "my-chart"
path = "."
release_type = "helm"
"#,
        ),
        ("Chart.yaml", chart),
    ])
    .await
    .unwrap();

    let changes = scenario.file_changes().await.unwrap();
    let updated = &change(&changes, "Chart.yaml").content;

    assert_eq!(
        updated,
        r#"# The chart that does the thing.
apiVersion: v2
name: my-chart
version: "0.1.0" # bumped on release
appVersion: "0.0.1"
"#
    );
}

/// `Chart.lock` is deliberately not a target: its digest is a sha256 over
/// the resolved dependency set, and writing a stale one makes Helm reject
/// the chart.
#[tokio::test]
async fn leaves_chart_lock_alone() {
    let scenario = Scenario::new(&[
        (
            "releasaurus.toml",
            r#"
[[package]]
name = "my-chart"
path = "."
release_type = "helm"
"#,
        ),
        ("Chart.yaml", CHART),
        (
            "Chart.lock",
            r#"dependencies:
  - name: redis
    version: 0.0.1
digest: sha256:abc
"#,
        ),
    ])
    .await
    .unwrap();

    let changes = scenario.file_changes().await.unwrap();

    assert_untouched(&changes, "Chart.lock");
}
