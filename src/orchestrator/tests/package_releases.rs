//! Tests for package release creation.
//!
//! Tests for:
//! - create_package_release method
//! - Metadata parsing from PR body
//! - Package matching by name
//! - Notes trimming
//! - Error handling for missing metadata

use super::common::*;
use crate::forge::{request::PullRequest, traits::MockForge};

#[tokio::test]
async fn create_package_release_parses_metadata_from_pr_body() {
    let pr_body = format!(
        r#"
<!--{{"metadata":{{"name":"{TEST_PKG_NAME}","tag":"v1.2.3","notes":"Release notes here"}}}}-->
<details><summary>v1.2.3</summary>

Release notes here
</details>
"#
    );

    let merged_pr = PullRequest {
        number: 123,
        sha: "abc123".to_string(),
        body: pr_body.to_string(),
    };

    // Set up mock forge expectations FIRST
    let mut mock_forge = MockForge::new();
    mock_forge.expect_dry_run().returning(|| false);
    mock_forge
        .expect_tag_commit()
        .times(1)
        .withf(|tag, sha| tag == "v1.2.3" && sha == "abc123")
        .returning(|_, _| Ok(()));

    mock_forge
        .expect_create_release()
        .times(1)
        .withf(|tag, sha, notes| {
            tag == "v1.2.3" && sha == "abc123" && notes == "Release notes here"
        })
        .returning(|_, _, _| Ok(()));

    // Then create orchestrator with the mock
    let orchestrator = create_test_orchestrator(mock_forge);

    let package = orchestrator.package_configs.get(TEST_PKG_NAME).unwrap();

    orchestrator
        .create_package_release(package, &merged_pr)
        .await
        .unwrap();
}

#[tokio::test]
async fn create_package_release_fails_when_metadata_missing() {
    let pr_body = r#"
<details><summary>v1.2.3</summary>

Release notes here
</details>
"#;

    let merged_pr = PullRequest {
        number: 123,
        sha: "abc123".to_string(),
        body: pr_body.to_string(),
    };

    // Mock forge not needed for this test since it should fail before calling forge
    let mock_forge = MockForge::new();
    let orchestrator = create_test_orchestrator(mock_forge);

    let package = orchestrator.package_configs.get(TEST_PKG_NAME).unwrap();
    let result = orchestrator
        .create_package_release(package, &merged_pr)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn create_package_release_matches_correct_package_by_name() {
    // PR body has metadata for different package - should match the correct one
    let pr_body = format!(
        r#"
<!--{{"metadata":{{"name":"other-pkg","tag":"v2.0.0","notes":"Other notes"}}}}-->
<details><summary>v2.0.0</summary>

Other notes
</details>

<!--{{"metadata":{{"name":"{TEST_PKG_NAME}","tag":"v1.2.3","notes":"Correct notes"}}}}-->
<details><summary>v1.2.3</summary>

Correct notes
</details>
"#
    );

    let merged_pr = PullRequest {
        number: 123,
        sha: "abc123".to_string(),
        body: pr_body.to_string(),
    };

    // Set up mock forge expectations
    let mut mock_forge = MockForge::new();
    mock_forge.expect_dry_run().returning(|| false);
    mock_forge
        .expect_tag_commit()
        .times(1)
        .withf(|tag, _| tag == "v1.2.3")
        .returning(|_, _| Ok(()));

    mock_forge
        .expect_create_release()
        .times(1)
        .withf(|_, _, notes| notes == "Correct notes")
        .returning(|_, _, _| Ok(()));

    let orchestrator = create_test_orchestrator(mock_forge);

    let package = orchestrator.package_configs.get(TEST_PKG_NAME).unwrap();

    orchestrator
        .create_package_release(package, &merged_pr)
        .await
        .unwrap();
}

#[tokio::test]
async fn create_package_release_trims_notes() {
    let pr_body = format!(
        r#"
<!--{{"metadata":{{"name":"{TEST_PKG_NAME}","tag":"v1.2.3","notes":"  \n Release notes \n  "}}}}-->
<details><summary>v1.2.3</summary>

Release notes
</details>
"#
    );

    let merged_pr = PullRequest {
        number: 123,
        sha: "abc123".to_string(),
        body: pr_body.to_string(),
    };

    // Set up mock forge expectations
    let mut mock_forge = MockForge::new();
    mock_forge.expect_dry_run().returning(|| false);
    mock_forge.expect_tag_commit().returning(|_, _| Ok(()));
    mock_forge
        .expect_create_release()
        .times(1)
        .withf(|_, _, notes| notes == "Release notes")
        .returning(|_, _, _| Ok(()));

    let orchestrator = create_test_orchestrator(mock_forge);

    let package = orchestrator.package_configs.get(TEST_PKG_NAME).unwrap();
    let result = orchestrator
        .create_package_release(package, &merged_pr)
        .await;
    result.unwrap();
}
