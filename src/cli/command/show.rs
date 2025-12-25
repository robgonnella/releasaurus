//! Shows information about prior and upcoming releases
use log::*;
use std::path::Path;
use tokio::fs;

use crate::{
    Result,
    cli::{ShowCommand, common, types::ReleasablePackage},
    config::Config,
    forge::manager::ForgeManager,
};

/// Get projected next release info as JSON, optionally filtered by package name.
pub async fn execute(
    forge_manager: &ForgeManager,
    cmd: ShowCommand,
    config: Config,
) -> Result<()> {
    match cmd {
        ShowCommand::NextRelease {
            out_file, package, ..
        } => show_next_release(config, forge_manager, out_file, package).await,
        ShowCommand::Release { out_file, tag } => {
            show_release(forge_manager, out_file, tag).await
        }
    }
}

async fn show_release(
    forge_manager: &ForgeManager,
    out_file: Option<String>,
    tag: String,
) -> Result<()> {
    info!("retrieving release data for tag: {tag}");
    let data = forge_manager.get_release_by_tag(&tag).await?;
    let json = serde_json::json!(&data);

    if let Some(out_file) = out_file {
        let file_path = Path::new(&out_file);
        info!(
            "writing release data for tag {tag} to: {}",
            file_path.display()
        );

        if let Some(parent) = file_path.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(&json)?;
        fs::write(file_path, &content).await?;
    } else {
        println!("{json}");
    }
    Ok(())
}

async fn show_next_release(
    config: Config,
    forge_manager: &ForgeManager,
    out_file: Option<String>,
    package: Option<String>,
) -> Result<()> {
    let base_branch = config.base_branch()?;

    let mut releasable_packages = common::get_releasable_packages(
        &config.packages,
        forge_manager,
        &base_branch,
    )
    .await?;

    if let Some(package) = package {
        releasable_packages = releasable_packages
            .into_iter()
            .filter(|p| p.name == package)
            .collect::<Vec<ReleasablePackage>>();
    }

    let json = serde_json::json!(&releasable_packages);

    if let Some(out_file) = out_file {
        let file_path = Path::new(&out_file);

        if let Some(parent) = file_path.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent).await?;
        }

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
        config::{Config, package::PackageConfig, release_type::ReleaseType},
        forge::{
            config::RemoteConfig, request::ReleaseByTagResponse,
            traits::MockForge,
        },
    };
    use semver::Version as SemVer;

    /// Creates a minimal releasable package for testing
    fn create_releasable_package(
        name: &str,
        version: &str,
    ) -> ReleasablePackage {
        ReleasablePackage {
            name: name.to_string(),
            workspace_root: ".".into(),
            path: ".".into(),
            release: Release {
                tag: Some(Tag {
                    sha: "test-sha".to_string(),
                    name: format!("v{}", version),
                    semver: SemVer::parse(version).unwrap(),
                    ..Tag::default()
                }),
                sha: "test-sha".to_string(),
                notes: format!("## Changes\n\nRelease {}", version),
                timestamp: 1234567890,
                ..Release::default()
            },
            ..ReleasablePackage::default()
        }
    }

    /// Creates a mock forge manager that returns the given packages
    fn mock_forge_with_packages(packages: Vec<PackageConfig>) -> ForgeManager {
        let mut mock = MockForge::new();

        mock.expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock.expect_load_config().returning(move |_| {
            Ok(Config {
                packages: packages.to_owned(),
                ..Config::default()
            })
        });

        mock.expect_get_latest_tag_for_prefix()
            .returning(|_| Ok(None));

        mock.expect_get_commits().returning(|_, _| Ok(vec![]));

        mock.expect_remote_config().returning(RemoteConfig::default);

        ForgeManager::new(Box::new(mock))
    }

    /// Creates a mock forge manager that returns release data
    fn mock_forge_with_release_data(
        tag: String,
        sha: String,
        notes: String,
    ) -> ForgeManager {
        let mut mock = MockForge::new();

        mock.expect_get_release_by_tag().returning(move |_| {
            Ok(ReleaseByTagResponse {
                tag: tag.clone(),
                sha: sha.clone(),
                notes: notes.clone(),
            })
        });

        mock.expect_remote_config().returning(RemoteConfig::default);

        ForgeManager::new(Box::new(mock))
    }

    // ===== NextRelease SubCommand Tests =====

    #[tokio::test]
    async fn next_release_returns_all_packages_when_no_filter() {
        let packages = vec![
            PackageConfig {
                name: "pkg-a".into(),
                release_type: Some(ReleaseType::Node),
                tag_prefix: Some("pkg-a-v".to_string()),
                ..PackageConfig::default()
            },
            PackageConfig {
                name: "pkg-b".into(),
                release_type: Some(ReleaseType::Rust),
                tag_prefix: Some("pkg-b-v".to_string()),
                ..PackageConfig::default()
            },
        ];

        let config = Config {
            base_branch: Some("main".into()),
            packages: packages.clone(),
            ..Config::default()
        };

        let manager = mock_forge_with_packages(packages);

        let cmd = ShowCommand::NextRelease {
            out_file: None,
            package: None,
            overrides: crate::cli::SharedCommandOverrides {
                package_overrides: vec![],
                prerelease_suffix: None,
                prerelease_strategy: None,
            },
        };

        execute(&manager, cmd, config).await.unwrap();
    }

    #[tokio::test]
    async fn next_release_filters_to_specific_package() {
        let packages = vec![
            PackageConfig {
                name: "pkg-a".into(),
                release_type: Some(ReleaseType::Node),
                tag_prefix: Some("pkg-a-v".to_string()),
                ..PackageConfig::default()
            },
            PackageConfig {
                name: "pkg-b".into(),
                release_type: Some(ReleaseType::Rust),
                tag_prefix: Some("pkg-b-v".to_string()),
                ..PackageConfig::default()
            },
        ];

        let config = Config {
            base_branch: Some("main".into()),
            packages: packages.clone(),
            ..Config::default()
        };

        let manager = mock_forge_with_packages(packages);

        let cmd = ShowCommand::NextRelease {
            out_file: None,
            package: Some("pkg-a".to_string()),
            overrides: crate::cli::SharedCommandOverrides {
                package_overrides: vec![],
                prerelease_suffix: None,
                prerelease_strategy: None,
            },
        };

        execute(&manager, cmd, config).await.unwrap();
    }

    #[tokio::test]
    async fn next_release_handles_empty_packages() {
        let config = Config {
            base_branch: Some("main".into()),
            packages: vec![],
            ..Config::default()
        };

        let manager = mock_forge_with_packages(vec![]);

        let cmd = ShowCommand::NextRelease {
            out_file: None,
            package: None,
            overrides: crate::cli::SharedCommandOverrides {
                package_overrides: vec![],
                prerelease_suffix: None,
                prerelease_strategy: None,
            },
        };

        execute(&manager, cmd, config).await.unwrap();
    }

    #[tokio::test]
    async fn next_release_creates_file_with_valid_json() {
        let config = Config {
            base_branch: Some("main".into()),
            packages: vec![],
            ..Config::default()
        };

        let manager = mock_forge_with_packages(vec![]);

        let temp_dir = tempfile::tempdir().unwrap();
        let out_file = temp_dir.path().join("output.json");
        let out_file_str = out_file.to_str().unwrap().to_string();

        let cmd = ShowCommand::NextRelease {
            out_file: Some(out_file_str),
            package: None,
            overrides: crate::cli::SharedCommandOverrides {
                package_overrides: vec![],
                prerelease_suffix: None,
                prerelease_strategy: None,
            },
        };

        execute(&manager, cmd, config).await.unwrap();

        // Verify file was created
        assert!(out_file.exists(), "Output file should be created");

        // Verify file contains valid JSON
        let content = tokio::fs::read_to_string(&out_file).await.unwrap();
        let json: serde_json::Value = serde_json::from_str(&content)
            .expect("File should contain valid JSON");

        // Verify it's a valid JSON array
        assert!(json.is_array(), "Output should be a JSON array");
    }

    #[tokio::test]
    async fn next_release_combines_filter_and_file_output() {
        let packages = vec![
            PackageConfig {
                name: "pkg-a".into(),
                release_type: Some(ReleaseType::Node),
                tag_prefix: Some("pkg-a-v".to_string()),
                ..PackageConfig::default()
            },
            PackageConfig {
                name: "pkg-b".into(),
                release_type: Some(ReleaseType::Rust),
                tag_prefix: Some("pkg-b-v".to_string()),
                ..PackageConfig::default()
            },
        ];

        let config = Config {
            base_branch: Some("main".into()),
            packages: packages.clone(),
            ..Config::default()
        };

        let manager = mock_forge_with_packages(packages);

        let cmd = ShowCommand::NextRelease {
            out_file: Some("/tmp/filtered.json".to_string()),
            package: Some("pkg-a".to_string()),
            overrides: crate::cli::SharedCommandOverrides {
                package_overrides: vec![],
                prerelease_suffix: None,
                prerelease_strategy: None,
            },
        };

        execute(&manager, cmd, config).await.unwrap();
    }

    #[tokio::test]
    async fn next_release_uses_branch_override() {
        let packages = vec![
            PackageConfig {
                name: "pkg-a".into(),
                release_type: Some(ReleaseType::Node),
                tag_prefix: Some("pkg-a-v".to_string()),
                ..PackageConfig::default()
            },
            PackageConfig {
                name: "pkg-b".into(),
                release_type: Some(ReleaseType::Rust),
                tag_prefix: Some("pkg-b-v".to_string()),
                ..PackageConfig::default()
            },
        ];

        let config = Config {
            base_branch: Some("develop".into()),
            packages: packages.clone(),
            ..Config::default()
        };

        let mut mock = MockForge::new();

        mock.expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock.expect_get_latest_tag_for_prefix()
            .returning(|_| Ok(None));

        mock.expect_get_commits().returning(|_, _| Ok(vec![]));

        mock.expect_remote_config().returning(RemoteConfig::default);

        let manager = ForgeManager::new(Box::new(mock));

        let cmd = ShowCommand::NextRelease {
            out_file: Some("/tmp/filtered.json".to_string()),
            package: Some("pkg-a".to_string()),
            overrides: crate::cli::SharedCommandOverrides {
                package_overrides: vec![],
                prerelease_suffix: None,
                prerelease_strategy: None,
            },
        };

        execute(&manager, cmd, config).await.unwrap();
    }

    // ===== Release SubCommand Tests =====

    #[tokio::test]
    async fn release_retrieves_data_to_stdout() {
        let notes = "## Release v1.0.0\n\n- Feature added".to_string();
        let manager = mock_forge_with_release_data(
            "v1.0.0".to_string(),
            "abc123".to_string(),
            notes,
        );

        let config = Config::default();

        let cmd = ShowCommand::Release {
            out_file: None,
            tag: "v1.0.0".to_string(),
        };

        execute(&manager, cmd, config).await.unwrap();
    }

    #[tokio::test]
    async fn release_writes_to_file() {
        let notes = "## Release v1.0.0\n\n- Feature added".to_string();
        let manager = mock_forge_with_release_data(
            "v1.0.0".to_string(),
            "abc123".to_string(),
            notes.clone(),
        );

        let config = Config::default();

        let temp_dir = tempfile::tempdir().unwrap();
        let out_file = temp_dir.path().join("release.json");
        let out_file_str = out_file.to_str().unwrap().to_string();

        let cmd = ShowCommand::Release {
            out_file: Some(out_file_str),
            tag: "v1.0.0".to_string(),
        };

        execute(&manager, cmd, config).await.unwrap();

        // Verify file was created
        assert!(out_file.exists(), "Output file should be created");

        // Verify file contains valid JSON with the release data
        let content = tokio::fs::read_to_string(&out_file).await.unwrap();
        let json: serde_json::Value = serde_json::from_str(&content)
            .expect("File should contain valid JSON");

        assert_eq!(json["tag"], "v1.0.0");
        assert_eq!(json["sha"], "abc123");
        assert_eq!(json["notes"], notes);
    }

    #[tokio::test]
    async fn release_handles_different_tags() {
        let notes = "## Release v2.1.3\n\n- Bug fix".to_string();
        let manager = mock_forge_with_release_data(
            "v2.1.3".to_string(),
            "def456".to_string(),
            notes,
        );

        let config = Config::default();

        let cmd = ShowCommand::Release {
            out_file: None,
            tag: "v2.1.3".to_string(),
        };

        execute(&manager, cmd, config).await.unwrap();
    }

    // ===== JSON Serialization Tests =====

    #[tokio::test]
    async fn serializes_releasable_package_correctly() {
        let package = create_releasable_package("test-pkg", "1.2.3");

        let json = serde_json::to_value(&package).unwrap();

        assert_eq!(json["name"], "test-pkg");
        assert_eq!(json["path"], ".");
        assert_eq!(json["workspace_root"], ".");
        assert_eq!(json["release_type"], "generic");
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
}
