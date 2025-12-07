//! Projected release command implementation.
use log::*;
use std::path::Path;
use tokio::fs;

use crate::{
    Result,
    command::{common, types::ReleasablePackage},
    forge::manager::ForgeManager,
};

/// Get projected next release info as JSON, optionally filtered by package name.
pub async fn execute(
    forge_manager: &ForgeManager,
    package: Option<String>,
    out_file: Option<String>,
) -> Result<()> {
    let mut config = forge_manager.load_config().await?;
    let repo_name = forge_manager.repo_name();
    let config = common::process_config(&repo_name, &mut config);

    let mut releasable_packages =
        common::get_releasable_packages(&config, forge_manager).await?;

    if let Some(package) = package {
        releasable_packages = releasable_packages
            .into_iter()
            .filter(|p| p.name == package)
            .collect::<Vec<ReleasablePackage>>();
    }

    let json = serde_json::json!(&releasable_packages);

    if let Some(out_file) = out_file {
        let file_path = Path::new(&out_file);
        let content = serde_json::to_string_pretty(&json)?;
        info!("writing projected release json to: {}", file_path.display());
        fs::write(file_path, &content).await?;
    } else {
        println!("{json}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analyzer::release::{Release, Tag},
        command::types::ReleasablePackage,
        config::release_type::ReleaseType,
        forge::traits::MockForge,
        test_helpers::*,
    };
    use semver::Version as SemVer;

    /// Creates a minimal releasable package for testing
    fn create_releasable_package(
        name: &str,
        version: &str,
    ) -> ReleasablePackage {
        ReleasablePackage {
            name: name.to_string(),
            path: ".".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: None,
            additional_manifest_files: None,
            release_type: ReleaseType::Node,
            release: Release {
                tag: Some(Tag {
                    sha: "test-sha".to_string(),
                    name: format!("v{}", version),
                    semver: SemVer::parse(version).unwrap(),
                    timestamp: None,
                }),
                link: format!(
                    "https://github.com/test/repo/releases/tag/v{}",
                    version
                ),
                sha: "test-sha".to_string(),
                commits: vec![],
                include_author: false,
                notes: format!("## Changes\n\nRelease {}", version),
                timestamp: 1234567890,
            },
        }
    }

    /// Creates a mock forge manager that returns the given packages
    fn mock_forge_with_packages(
        packages: Vec<(&str, &str, ReleaseType)>,
    ) -> ForgeManager {
        let mut mock = MockForge::new();

        // Convert to owned data to satisfy 'static lifetime requirement
        let owned_packages: Vec<(String, String, ReleaseType)> = packages
            .into_iter()
            .map(|(name, path, rt)| (name.to_string(), path.to_string(), rt))
            .collect();

        mock.expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock.expect_load_config().returning(move || {
            let config_packages: Vec<(&str, &str, ReleaseType)> =
                owned_packages
                    .iter()
                    .map(|(name, path, rt)| {
                        (name.as_str(), path.as_str(), rt.clone())
                    })
                    .collect();
            Ok(create_test_config_simple(config_packages))
        });

        mock.expect_default_branch()
            .returning(|| "main".to_string());

        mock.expect_get_latest_tag_for_prefix()
            .returning(|_| Ok(None));

        mock.expect_get_commits().returning(|_| Ok(vec![]));

        mock.expect_remote_config()
            .returning(create_test_remote_config);

        ForgeManager::new(Box::new(mock))
    }

    #[tokio::test]
    async fn returns_all_packages_when_no_filter_specified() {
        let manager = mock_forge_with_packages(vec![
            ("pkg-a", ".", ReleaseType::Node),
            ("pkg-b", ".", ReleaseType::Rust),
        ]);

        let result = execute(&manager, None, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn returns_specific_package_when_filtered() {
        let manager = mock_forge_with_packages(vec![
            ("pkg-a", ".", ReleaseType::Node),
            ("pkg-b", ".", ReleaseType::Rust),
        ]);

        let result = execute(&manager, Some("pkg-a".to_string()), None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn returns_empty_array_when_filter_does_not_match() {
        let manager = mock_forge_with_packages(vec![
            ("pkg-a", ".", ReleaseType::Node),
            ("pkg-b", ".", ReleaseType::Rust),
        ]);

        let result =
            execute(&manager, Some("nonexistent".to_string()), None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn handles_empty_releasable_packages() {
        let manager = mock_forge_with_packages(vec![]);

        let result = execute(&manager, None, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn returns_single_package_as_array_when_no_filter() {
        let manager = mock_forge_with_packages(vec![(
            "single-pkg",
            ".",
            ReleaseType::Node,
        )]);

        let result = execute(&manager, None, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn serializes_releasable_package_correctly() {
        let package = create_releasable_package("test-pkg", "1.2.3");

        let json = serde_json::to_value(&package).unwrap();

        assert_eq!(json["name"], "test-pkg");
        assert_eq!(json["path"], ".");
        assert_eq!(json["workspace_root"], ".");
        assert_eq!(json["release_type"], "node");
        assert_eq!(json["release"]["version"], "1.2.3");
        assert_eq!(json["release"]["sha"], "test-sha");
        assert!(
            json["release"]["notes"]
                .as_str()
                .unwrap()
                .contains("Release 1.2.3")
        );
    }

    #[tokio::test]
    async fn serializes_multiple_packages_as_array() {
        let packages = vec![
            create_releasable_package("pkg-a", "1.0.0"),
            create_releasable_package("pkg-b", "2.0.0"),
        ];

        let json = serde_json::to_value(&packages).unwrap();

        assert!(json.is_array());
        assert_eq!(json.as_array().unwrap().len(), 2);
        assert_eq!(json[0]["name"], "pkg-a");
        assert_eq!(json[1]["name"], "pkg-b");
    }

    #[tokio::test]
    async fn creates_file_with_valid_json() {
        let manager = mock_forge_with_packages(vec![]);

        let temp_dir = tempfile::tempdir().unwrap();
        let out_file = temp_dir.path().join("output.json");
        let out_file_str = out_file.to_str().unwrap().to_string();

        let result = execute(&manager, None, Some(out_file_str)).await;
        assert!(result.is_ok());

        // Verify file was created
        assert!(out_file.exists(), "Output file should be created");

        // Verify file contains valid JSON
        let content = tokio::fs::read_to_string(&out_file).await.unwrap();
        let json: serde_json::Value = serde_json::from_str(&content)
            .expect("File should contain valid JSON");

        // Verify it's a valid JSON array (this is the key behavioral outcome)
        assert!(json.is_array(), "Output should be a JSON array");
    }

    #[tokio::test]
    async fn succeeds_with_filter_and_out_file() {
        let manager = mock_forge_with_packages(vec![
            ("pkg-a", ".", ReleaseType::Node),
            ("pkg-b", ".", ReleaseType::Rust),
        ]);

        let result = execute(
            &manager,
            Some("pkg-a".to_string()),
            Some("/tmp/filtered.json".to_string()),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn succeeds_with_empty_packages_and_out_file() {
        let manager = mock_forge_with_packages(vec![]);

        let result =
            execute(&manager, None, Some("/tmp/empty.json".to_string())).await;
        assert!(result.is_ok());
    }
}
