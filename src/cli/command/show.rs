//! Shows information about prior and upcoming releases
use log::*;
use serde::Serialize;
use std::path::Path;
use tokio::fs;

use crate::{
    Result,
    cli::{ShowCommand, common, types::ReleasablePackage},
    config::Config,
    forge::manager::ForgeManager,
};

/// Information about a package's current release
#[derive(Serialize)]
struct CurrentRelease {
    name: String,
    tag: String,
    sha: String,
    notes: String,
}

/// Get projected next release info as JSON, optionally filtered by package name
pub async fn execute(
    forge_manager: &ForgeManager,
    cmd: ShowCommand,
    config: Config,
) -> Result<()> {
    match cmd {
        ShowCommand::NextRelease {
            out_file, package, ..
        } => show_next_release(config, forge_manager, out_file, package).await,
        ShowCommand::CurrentRelease { out_file, package } => {
            show_current_release(config, forge_manager, out_file, package).await
        }
        ShowCommand::Release { out_file, tag } => {
            show_release(forge_manager, out_file, tag).await
        }
    }
}

/// Fetches the most recent release for each package
/// Packages without releases are omitted
async fn get_current_releases(
    config: &Config,
    forge_manager: &ForgeManager,
    target_package: Option<&str>,
) -> Result<Vec<CurrentRelease>> {
    let mut releases = vec![];

    for package in config.packages.iter() {
        if let Some(target) = target_package
            && package.name != target
        {
            continue;
        }

        let prefix = package.tag_prefix()?;
        let current = forge_manager.get_latest_tag_for_prefix(&prefix).await?;

        if let Some(tag) = current {
            let data = forge_manager.get_release_by_tag(&tag.name).await?;
            releases.push(CurrentRelease {
                name: package.name.clone(),
                tag: data.tag,
                sha: data.sha,
                notes: data.notes,
            });
        }
    }

    Ok(releases)
}

/// Shows the most recent release for each package
async fn show_current_release(
    config: Config,
    forge_manager: &ForgeManager,
    out_file: Option<String>,
    target_package: Option<String>,
) -> Result<()> {
    let releases =
        get_current_releases(&config, forge_manager, target_package.as_deref())
            .await?;

    let json = serde_json::json!(releases);
    print_json(json, out_file).await
}

async fn show_release(
    forge_manager: &ForgeManager,
    out_file: Option<String>,
    tag: String,
) -> Result<()> {
    info!("retrieving release data for tag: {tag}");
    let data = forge_manager.get_release_by_tag(&tag).await?;
    let json = serde_json::json!(&data);
    print_json(json, out_file).await
}

/// Fetches projected next release information
async fn get_next_releases(
    config: &Config,
    forge_manager: &ForgeManager,
    package: Option<&str>,
) -> Result<Vec<ReleasablePackage>> {
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

    Ok(releasable_packages)
}

async fn show_next_release(
    config: Config,
    forge_manager: &ForgeManager,
    out_file: Option<String>,
    package: Option<String>,
) -> Result<()> {
    let releasable_packages =
        get_next_releases(&config, forge_manager, package.as_deref()).await?;
    let json = serde_json::json!(&releasable_packages);
    print_json(json, out_file).await
}

async fn print_json(
    json: serde_json::Value,
    out_file: Option<String>,
) -> Result<()> {
    if let Some(out_file) = out_file {
        let file_path = Path::new(&out_file);

        if let Some(parent) = file_path.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(&json)?;
        info!("writing json to: {}", file_path.display());
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
        cli::CommitModifiers,
        config::{
            Config,
            package::{PackageConfig, PackageConfigBuilder},
            release_type::ReleaseType,
        },
        forge::{
            request::{ForgeCommitBuilder, ReleaseByTagResponse},
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
                tag: Tag {
                    sha: "test-sha".to_string(),
                    name: format!("v{}", version),
                    semver: SemVer::parse(version).unwrap(),
                    ..Tag::default()
                },
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

        mock.expect_dry_run().returning(|| false);

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

        mock.expect_dry_run().returning(|| false);

        ForgeManager::new(Box::new(mock))
    }

    // ===== NextRelease SubCommand Tests =====

    #[tokio::test]
    async fn next_release_returns_all_packages_when_no_filter() {
        let packages = vec![
            PackageConfigBuilder::default()
                .name("pkg-a")
                .release_type(ReleaseType::Node)
                .tag_prefix("pkg-a-v")
                .build()
                .unwrap(),
            PackageConfigBuilder::default()
                .name("pkg-b")
                .release_type(ReleaseType::Rust)
                .tag_prefix("pkg-b-v")
                .build()
                .unwrap(),
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
            commit_modifiers: CommitModifiers::default(),
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
            PackageConfigBuilder::default()
                .name("pkg-a")
                .tag_prefix("pkg-a-v")
                .build()
                .unwrap(),
            PackageConfigBuilder::default()
                .name("pkg-b")
                .tag_prefix("pkg-b-v")
                .build()
                .unwrap(),
        ];

        let config = Config {
            base_branch: Some("main".into()),
            packages: packages.clone(),
            ..Config::default()
        };

        let mut mock = MockForge::new();

        mock.expect_repo_name().returning(|| "test-repo".into());
        mock.expect_dry_run().returning(|| false);
        mock.expect_get_latest_tag_for_prefix()
            .returning(|_| Ok(None));
        mock.expect_get_commits().returning(|_, _| {
            Ok(vec![
                ForgeCommitBuilder::default()
                    .message("feat: test feature")
                    .files(vec!["main.rs".into()])
                    .build()
                    .unwrap(),
            ])
        });

        let forge_manager = ForgeManager::new(Box::new(mock));

        let next_releases =
            get_next_releases(&config, &forge_manager, "pkg-a".into())
                .await
                .unwrap();

        assert_eq!(next_releases.len(), 1);
        assert_eq!(next_releases[0].name, "pkg-a");
        assert_eq!(next_releases[0].release.tag.semver.to_string(), "0.1.0");
        assert!(next_releases[0].release.notes.contains("test feature"));
    }

    #[tokio::test]
    async fn next_release_command_filters_and_writes_correct_output() {
        let packages = vec![
            PackageConfigBuilder::default()
                .name("pkg-a")
                .tag_prefix("pkg-a-v")
                .build()
                .unwrap(),
            PackageConfigBuilder::default()
                .name("pkg-b")
                .tag_prefix("pkg-b-v")
                .build()
                .unwrap(),
        ];

        let config = Config {
            base_branch: Some("main".into()),
            packages: packages.clone(),
            ..Config::default()
        };

        let mut mock = MockForge::new();

        mock.expect_repo_name().returning(|| "test-repo".into());
        mock.expect_dry_run().returning(|| false);
        mock.expect_get_latest_tag_for_prefix()
            .returning(|_| Ok(None));
        mock.expect_get_commits().returning(|_, _| {
            Ok(vec![
                ForgeCommitBuilder::default()
                    .message("feat: test feature")
                    .files(vec!["main.rs".into()])
                    .build()
                    .unwrap(),
            ])
        });

        let forge_manager = ForgeManager::new(Box::new(mock));

        let temp_dir = tempfile::tempdir().unwrap();
        let out_file = temp_dir.path().join("next-release.json");
        let out_file_str = out_file.to_str().unwrap().to_string();

        let cmd = ShowCommand::NextRelease {
            out_file: Some(out_file_str),
            package: Some("pkg-a".to_string()),
            commit_modifiers: CommitModifiers::default(),
            overrides: crate::cli::SharedCommandOverrides {
                package_overrides: vec![],
                prerelease_suffix: None,
                prerelease_strategy: None,
            },
        };

        execute(&forge_manager, cmd, config).await.unwrap();

        assert!(out_file.exists());

        let content = tokio::fs::read_to_string(&out_file).await.unwrap();
        let json: serde_json::Value =
            serde_json::from_str(&content).expect("Valid JSON");

        assert!(json.is_array());
        let releases = json.as_array().unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0]["name"], "pkg-a");
        assert_eq!(releases[0]["release"]["version"], "0.1.0");
        assert!(
            releases[0]["release"]["notes"]
                .as_str()
                .unwrap()
                .contains("test feature")
        );
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
            commit_modifiers: CommitModifiers::default(),
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
            commit_modifiers: CommitModifiers::default(),
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
            PackageConfigBuilder::default()
                .name("pkg-a")
                .release_type(ReleaseType::Node)
                .tag_prefix("pkg-a-v")
                .build()
                .unwrap(),
            PackageConfigBuilder::default()
                .name("pkg-b")
                .release_type(ReleaseType::Rust)
                .tag_prefix("pkg-b-v")
                .build()
                .unwrap(),
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
            commit_modifiers: CommitModifiers::default(),
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
            PackageConfigBuilder::default()
                .name("pkg-a")
                .release_type(ReleaseType::Node)
                .tag_prefix("pkg-a-v")
                .build()
                .unwrap(),
            PackageConfigBuilder::default()
                .name("pkg-b")
                .release_type(ReleaseType::Rust)
                .tag_prefix("pkg-b-v")
                .build()
                .unwrap(),
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

        mock.expect_dry_run().returning(|| false);

        let manager = ForgeManager::new(Box::new(mock));

        let cmd = ShowCommand::NextRelease {
            out_file: Some("/tmp/filtered.json".to_string()),
            package: Some("pkg-a".to_string()),
            commit_modifiers: CommitModifiers::default(),
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

    // ===== CurrentRelease SubCommand Tests =====

    #[derive(Clone, Copy)]
    struct MockTag {
        prefix: &'static str,
        semver: &'static str,
        sha: &'static str,
    }

    fn mock_forge_with_tags(tags: Vec<MockTag>) -> ForgeManager {
        let mut mock = MockForge::new();

        mock.expect_get_latest_tag_for_prefix()
            .returning(move |prefix| {
                for mock_tag in &tags {
                    if prefix.contains(mock_tag.prefix) {
                        let tag_name =
                            format!("{}{}", mock_tag.prefix, mock_tag.semver);
                        return Ok(Some(Tag {
                            sha: mock_tag.sha.to_string(),
                            name: tag_name,
                            semver: SemVer::parse(mock_tag.semver).unwrap(),
                            ..Tag::default()
                        }));
                    }
                }
                Ok(None)
            });

        mock.expect_get_release_by_tag().returning(|tag| {
            Ok(ReleaseByTagResponse {
                tag: tag.to_string(),
                sha: format!("sha-{}", tag),
                notes: format!("Release notes for {}", tag),
            })
        });

        mock.expect_dry_run().returning(|| false);

        ForgeManager::new(Box::new(mock))
    }

    #[tokio::test]
    async fn current_release_returns_all_packages_with_releases() {
        let packages = vec![
            PackageConfigBuilder::default()
                .name("pkg-a")
                .release_type(ReleaseType::Node)
                .tag_prefix("pkg-a-v")
                .build()
                .unwrap(),
            PackageConfigBuilder::default()
                .name("pkg-b")
                .release_type(ReleaseType::Rust)
                .tag_prefix("pkg-b-v")
                .build()
                .unwrap(),
        ];

        let config = Config {
            packages,
            ..Config::default()
        };

        let manager = mock_forge_with_tags(vec![
            MockTag {
                prefix: "pkg-a-v",
                semver: "1.0.0",
                sha: "sha-a",
            },
            MockTag {
                prefix: "pkg-b-v",
                semver: "2.0.0",
                sha: "sha-b",
            },
        ]);

        let releases =
            get_current_releases(&config, &manager, None).await.unwrap();

        assert_eq!(releases.len(), 2);
        assert_eq!(releases[0].name, "pkg-a");
        assert_eq!(releases[0].tag, "pkg-a-v1.0.0");
        assert_eq!(releases[0].sha, "sha-pkg-a-v1.0.0");
        assert_eq!(releases[1].name, "pkg-b");
        assert_eq!(releases[1].tag, "pkg-b-v2.0.0");
    }

    #[tokio::test]
    async fn current_release_filters_to_specific_package() {
        let packages = vec![
            PackageConfigBuilder::default()
                .name("pkg-a")
                .release_type(ReleaseType::Node)
                .tag_prefix("pkg-a-v")
                .build()
                .unwrap(),
            PackageConfigBuilder::default()
                .name("pkg-b")
                .release_type(ReleaseType::Rust)
                .tag_prefix("pkg-b-v")
                .build()
                .unwrap(),
        ];

        let config = Config {
            packages,
            ..Config::default()
        };

        let manager = mock_forge_with_tags(vec![
            MockTag {
                prefix: "pkg-a-v",
                semver: "1.0.0",
                sha: "sha-a",
            },
            MockTag {
                prefix: "pkg-b-v",
                semver: "2.0.0",
                sha: "sha-b",
            },
        ]);

        let releases = get_current_releases(&config, &manager, Some("pkg-a"))
            .await
            .unwrap();

        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].name, "pkg-a");
        assert_eq!(releases[0].tag, "pkg-a-v1.0.0");
    }

    #[tokio::test]
    async fn current_release_omits_packages_without_releases() {
        let packages = vec![
            PackageConfigBuilder::default()
                .name("pkg-a")
                .release_type(ReleaseType::Node)
                .tag_prefix("pkg-a-v")
                .build()
                .unwrap(),
            PackageConfigBuilder::default()
                .name("never-released")
                .release_type(ReleaseType::Rust)
                .tag_prefix("never-v")
                .build()
                .unwrap(),
        ];

        let config = Config {
            packages,
            ..Config::default()
        };

        let manager = mock_forge_with_tags(vec![MockTag {
            prefix: "pkg-a-v",
            semver: "1.0.0",
            sha: "sha-a",
        }]);

        let releases =
            get_current_releases(&config, &manager, None).await.unwrap();

        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].name, "pkg-a");
    }

    #[tokio::test]
    async fn current_release_writes_to_file() {
        let packages = vec![
            PackageConfigBuilder::default()
                .name("pkg-a")
                .release_type(ReleaseType::Node)
                .tag_prefix("pkg-a-v")
                .build()
                .unwrap(),
        ];

        let config = Config {
            packages,
            ..Config::default()
        };

        let manager = mock_forge_with_tags(vec![MockTag {
            prefix: "pkg-a-v",
            semver: "1.0.0",
            sha: "sha-a",
        }]);

        let temp_dir = tempfile::tempdir().unwrap();
        let out_file = temp_dir.path().join("current.json");
        let out_file_str = out_file.to_str().unwrap().to_string();

        let cmd = ShowCommand::CurrentRelease {
            out_file: Some(out_file_str),
            package: None,
        };

        execute(&manager, cmd, config).await.unwrap();

        assert!(out_file.exists());

        let content = tokio::fs::read_to_string(&out_file).await.unwrap();
        let json: serde_json::Value = serde_json::from_str(&content)
            .expect("File should contain valid JSON");

        assert!(json.is_array());
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
