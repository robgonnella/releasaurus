//! Strftime `version_type` scenarios. Regex-based updaters find the value
//! they replace by its shape, so a calendar format whose values are not
//! `X.Y.Z` must still be found and rewritten rather than silently skipped.

use chrono::Utc;

use super::common::*;

/// The book's own example: a `generic` package versioned `%y.%m` with a
/// default-regex `additional_manifest_files` entry.
#[tokio::test]
async fn additional_manifest_takes_a_two_segment_calver_version() {
    let scenario = Scenario::new(&[
        (
            "releasaurus.toml",
            r#"
[[package]]
path = "."
release_type = "generic"
versioning = { version_type = "%y.%m" }
additional_manifest_files = ["VERSION"]
"#,
        ),
        ("VERSION", "version = \"25.01\"\n"),
    ])
    .await
    .unwrap();

    let changes = scenario.file_changes().await.unwrap();
    let updated = &change(&changes, "VERSION").content;
    let expected = Utc::now().format("%y.%m").to_string();

    assert_eq!(updated, &format!("version = \"{expected}\"\n"));
}

#[tokio::test]
async fn language_updater_takes_a_single_segment_calver_version() {
    let scenario = Scenario::new(&[
        (
            "releasaurus.toml",
            r#"
[[package]]
path = "."
release_type = "python"
versioning = { version_type = "%Y%m%d" }
"#,
        ),
        ("setup.cfg", "[metadata]\nname = pkg\nversion = 20250101\n"),
    ])
    .await
    .unwrap();

    let changes = scenario.file_changes().await.unwrap();
    let updated = &change(&changes, "setup.cfg").content;
    let expected = Utc::now().format("%Y%m%d").to_string();

    assert_eq!(
        updated,
        &format!("[metadata]\nname = pkg\nversion = {expected}\n")
    );
}
