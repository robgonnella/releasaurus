//! Tests for show notes functionality.
//!
//! Tests for:
//! - get_notes_from_file method
//! - Reading and parsing JSON release files
//! - Template rendering with package release data
//! - Error handling for invalid files and malformed JSON

use super::common::*;
use crate::{
    analyzer::release::{Release, Tag},
    config::Config,
    forge::traits::MockForge,
    orchestrator::package::releasable::SerializableReleasablePackage,
};
use std::{io::Write, path::PathBuf};
use tempfile::NamedTempFile;

// Helper function to create a test release with minimal required fields
fn create_test_release(version: &str, notes: &str) -> Release {
    Release {
        tag: Tag {
            name: format!("v{}", version),
            semver: Version::parse(version).unwrap(),
            sha: "abc123".to_string(),
            ..Default::default()
        },
        notes: notes.to_string(),
        ..Default::default()
    }
}

#[tokio::test]
async fn get_notes_from_file_returns_rendered_notes() {
    let mut mock_forge = MockForge::new();
    mock_forge.expect_dry_run().returning(|| false);

    let orchestrator = create_test_orchestrator(mock_forge);

    // Create temporary file with valid JSON
    let mut temp_file = NamedTempFile::new().unwrap();
    let package = SerializableReleasablePackage {
        name: "test-package".to_string(),
        path: PathBuf::from("."),
        release: create_test_release("1.0.0", "Existing notes"),
        ..Default::default()
    };

    let json = serde_json::to_string(&vec![package]).unwrap();
    temp_file.write_all(json.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let result = orchestrator
        .get_notes_from_file(&temp_file.path().to_string_lossy())
        .await
        .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "test-package");
    assert!(!result[0].notes.is_empty());
}

#[tokio::test]
async fn get_notes_from_file_handles_multiple_packages() {
    let mut mock_forge = MockForge::new();
    mock_forge.expect_dry_run().returning(|| false);

    let orchestrator = create_test_orchestrator(mock_forge);

    let mut temp_file = NamedTempFile::new().unwrap();
    let packages = vec![
        SerializableReleasablePackage {
            name: "package-one".to_string(),
            path: PathBuf::from("packages/one"),
            release: create_test_release("1.0.0", "Pkg1 Existing notes"),
            ..Default::default()
        },
        SerializableReleasablePackage {
            name: "package-two".to_string(),
            path: PathBuf::from("packages/two"),
            release: create_test_release("2.0.0", "Pkg2 Existing notes"),
            ..Default::default()
        },
    ];

    let json = serde_json::to_string(&packages).unwrap();
    temp_file.write_all(json.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let result = orchestrator
        .get_notes_from_file(&temp_file.path().to_string_lossy())
        .await
        .unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "package-one");
    assert_eq!(result[1].name, "package-two");
    assert!(!result[0].notes.is_empty());
    assert!(!result[1].notes.is_empty());
}

#[tokio::test]
async fn get_notes_from_file_renders_with_custom_template() {
    let mut mock_forge = MockForge::new();
    mock_forge.expect_dry_run().returning(|| false);

    // Create custom config with specific template
    let mut config = Config::default();
    config.changelog.body = "Version: {{ version }}".to_string();

    let orchestrator =
        create_test_orchestrator_with_config(mock_forge, vec![], Some(config));

    let mut temp_file = NamedTempFile::new().unwrap();
    let package = SerializableReleasablePackage {
        name: "test-pkg".to_string(),
        path: PathBuf::from("."),
        release: create_test_release("3.2.1", "Release notes"),
        ..Default::default()
    };

    let json = serde_json::to_string(&vec![package]).unwrap();
    temp_file.write_all(json.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let result = orchestrator
        .get_notes_from_file(&temp_file.path().to_string_lossy())
        .await
        .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].notes, "Version: 3.2.1");
}

#[tokio::test]
async fn get_notes_from_file_fails_with_nonexistent_file() {
    let mut mock_forge = MockForge::new();
    mock_forge.expect_dry_run().returning(|| false);

    let orchestrator = create_test_orchestrator(mock_forge);

    let result = orchestrator
        .get_notes_from_file("/nonexistent/path/file.json")
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("does not exist"));
}

#[tokio::test]
async fn get_notes_from_file_fails_with_invalid_json() {
    let mut mock_forge = MockForge::new();
    mock_forge.expect_dry_run().returning(|| false);

    let orchestrator = create_test_orchestrator(mock_forge);

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"not valid json").unwrap();
    temp_file.flush().unwrap();

    let result = orchestrator
        .get_notes_from_file(&temp_file.path().to_string_lossy())
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn get_notes_from_file_handles_empty_array() {
    let mut mock_forge = MockForge::new();
    mock_forge.expect_dry_run().returning(|| false);

    let orchestrator = create_test_orchestrator(mock_forge);

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"[]").unwrap();
    temp_file.flush().unwrap();

    let result = orchestrator
        .get_notes_from_file(&temp_file.path().to_string_lossy())
        .await
        .unwrap();

    assert_eq!(result.len(), 0);
}

#[tokio::test]
async fn get_notes_from_file_fails_with_invalid_template() {
    let mut mock_forge = MockForge::new();
    mock_forge.expect_dry_run().returning(|| false);

    let mut config = Config::default();
    // Invalid template syntax
    config.changelog.body = "{{ unclosed_variable".to_string();

    let orchestrator =
        create_test_orchestrator_with_config(mock_forge, vec![], Some(config));

    let mut temp_file = NamedTempFile::new().unwrap();
    let package = SerializableReleasablePackage {
        name: "test-pkg".to_string(),
        path: PathBuf::from("."),
        release: create_test_release("1.0.0", "Release notes"),
        ..Default::default()
    };

    let json = serde_json::to_string(&vec![package]).unwrap();
    temp_file.write_all(json.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let result = orchestrator
        .get_notes_from_file(&temp_file.path().to_string_lossy())
        .await;

    assert!(result.is_err());
}
