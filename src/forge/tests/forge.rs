//! Integration tests for all forge implementations (GitHub, GitLab, Gitea).
//!
//! These tests run against live forge instances using real repositories.
//!
//! ## Running the Tests
//!
//! Set the appropriate environment variables with valid API tokens:
//!
//! ```bash
//! export GH_TEST_TOKEN="your-github-token"
//! export GL_TEST_TOKEN="your-gitlab-token"
//! export GT_TEST_TOKEN="your-gitea-token"
//! cargo test --lib --features _internal_e2e_tests forge::tests::forge
//! ```
//!
//! ## Test Repositories
//!
//! - GitHub: https://github.com/robgonnella/test-repo
//! - GitLab: https://gitlab.com/rgon/test-repo
//! - Gitea: https://gitea.com/rgon/test-repo

use crate::{
    forge::{
        config::RemoteConfig,
        gitea::Gitea,
        github::Github,
        gitlab::Gitlab,
        request::{
            CreateBranchRequest, CreatePrRequest, FileChange, FileUpdateType,
            GetPrRequest,
        },
        traits::Forge,
    },
    result::Result,
};
use secrecy::SecretString;
use std::env;

/// Forge type enum for test parameterization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ForgeType {
    GitHub,
    GitLab,
    Gitea,
}

impl ForgeType {
    fn name(&self) -> &str {
        match self {
            ForgeType::GitHub => "GitHub",
            ForgeType::GitLab => "GitLab",
            ForgeType::Gitea => "Gitea",
        }
    }

    fn token_env_var(&self) -> &str {
        match self {
            ForgeType::GitHub => "GH_TEST_TOKEN",
            ForgeType::GitLab => "GL_TEST_TOKEN",
            ForgeType::Gitea => "GT_TEST_TOKEN",
        }
    }

    fn host(&self) -> &str {
        match self {
            ForgeType::GitHub => "github.com",
            ForgeType::GitLab => "gitlab.com",
            ForgeType::Gitea => "gitea.com",
        }
    }

    fn owner(&self) -> &str {
        match self {
            ForgeType::GitHub => "robgonnella",
            ForgeType::GitLab => "rgon",
            ForgeType::Gitea => "rgon",
        }
    }

    fn repo(&self) -> &str {
        "test-repo"
    }

    fn path(&self) -> String {
        format!("{}/{}", self.owner(), self.repo())
    }
}

/// Creates a RemoteConfig for the specified forge type
fn create_remote_config(forge_type: ForgeType) -> RemoteConfig {
    let token = env::var(forge_type.token_env_var()).unwrap_or_else(|_| {
        panic!(
            "{} environment variable must be set to run {} integration tests",
            forge_type.token_env_var(),
            forge_type.name()
        )
    });

    let host = forge_type.host();
    let owner = forge_type.owner();
    let repo = forge_type.repo();
    let path = forge_type.path();

    RemoteConfig {
        host: host.to_string(),
        port: None,
        scheme: "https".to_string(),
        owner: owner.to_string(),
        repo: repo.to_string(),
        path: path.clone(),
        token: SecretString::from(token),
        commit_link_base_url: format!("https://{}/{}/commit", host, path),
        release_link_base_url: format!(
            "https://{}/{}/releases/tag",
            host, path
        ),
    }
}

/// Creates a forge instance for the specified type
async fn create_forge(
    forge_type: ForgeType,
) -> Result<Box<dyn Forge + Send + Sync>> {
    let config = create_remote_config(forge_type);

    match forge_type {
        ForgeType::GitHub => {
            let forge = Github::new(config)?;
            Ok(Box::new(forge) as Box<dyn Forge + Send + Sync>)
        }
        ForgeType::GitLab => {
            let forge = Gitlab::new(config).await?;
            Ok(Box::new(forge) as Box<dyn Forge + Send + Sync>)
        }
        ForgeType::Gitea => {
            let forge = Gitea::new(config)?;
            Ok(Box::new(forge) as Box<dyn Forge + Send + Sync>)
        }
    }
}

/// Helper function to close a PR for the specified forge type
async fn close_pr(forge_type: ForgeType, pr_number: u64) -> Result<()> {
    use secrecy::ExposeSecret;
    let config = create_remote_config(forge_type);

    match forge_type {
        ForgeType::GitHub => {
            use octocrab::Octocrab;
            let octocrab = Octocrab::builder()
                .personal_token(config.token.expose_secret().to_string())
                .build()?;

            octocrab
                .pulls(&config.owner, &config.repo)
                .update(pr_number)
                .state(octocrab::params::pulls::State::Closed)
                .send()
                .await?;
            Ok(())
        }
        ForgeType::GitLab => {
            use gitlab::api::projects::merge_requests::MergeRequestStateEvent;
            use gitlab::{
                GitlabBuilder,
                api::{AsyncQuery, projects::merge_requests::EditMergeRequest},
            };

            let client =
                GitlabBuilder::new(&config.host, config.token.expose_secret())
                    .build_async()
                    .await?;
            let project_id = format!("{}/{}", config.owner, config.repo);

            let endpoint = EditMergeRequest::builder()
                .project(&project_id)
                .merge_request(pr_number)
                .state_event(MergeRequestStateEvent::Close)
                .build()?;

            let _result: serde_json::Value =
                endpoint.query_async(&client).await?;
            Ok(())
        }
        ForgeType::Gitea => {
            use reqwest::{
                Client,
                header::{HeaderMap, HeaderValue},
            };

            let token_value = HeaderValue::from_str(
                format!("token {}", config.token.expose_secret()).as_str(),
            )?;
            let mut headers = HeaderMap::new();
            headers.append("Authorization", token_value);

            let client = Client::builder().default_headers(headers).build()?;

            let url = format!(
                "https://{}/api/v1/repos/{}/{}/pulls/{}",
                config.host, config.owner, config.repo, pr_number
            );

            #[derive(serde::Serialize)]
            struct ClosePr {
                state: String,
            }

            client
                .patch(&url)
                .json(&ClosePr {
                    state: "closed".to_string(),
                })
                .send()
                .await?;
            Ok(())
        }
    }
}

/// Helper macro to run a test against all forge types
macro_rules! test_all_forges {
    ($test_name:ident, $test_fn:expr) => {
        paste::paste! {
            #[tokio::test]
            async fn [<$test_name _github>]() {
                if env::var("GH_TEST_TOKEN").is_ok() {
                    let forge = create_forge(ForgeType::GitHub).await.expect("Failed to create GitHub forge");
                    $test_fn(forge, ForgeType::GitHub).await;
                } else {
                    println!("Skipping GitHub test - GH_TEST_TOKEN not set");
                }
            }

            #[tokio::test]
            async fn [<$test_name _gitlab>]() {
                if env::var("GL_TEST_TOKEN").is_ok() {
                    let forge = create_forge(ForgeType::GitLab).await.expect("Failed to create GitLab forge");
                    $test_fn(forge, ForgeType::GitLab).await;
                } else {
                    println!("Skipping GitLab test - GL_TEST_TOKEN not set");
                }
            }

            #[tokio::test]
            async fn [<$test_name _gitea>]() {
                if env::var("GT_TEST_TOKEN").is_ok() {
                    let forge = create_forge(ForgeType::Gitea).await.expect("Failed to create Gitea forge");
                    $test_fn(forge, ForgeType::Gitea).await;
                } else {
                    println!("Skipping Gitea test - GT_TEST_TOKEN not set");
                }
            }
        }
    };
}

// Test: Forge instance creation and basic info
async fn test_forge_repo_name_impl(
    forge: Box<dyn Forge + Send + Sync>,
    forge_type: ForgeType,
) {
    let repo_name = forge.repo_name();
    assert_eq!(
        repo_name,
        forge_type.repo(),
        "[{}] repo_name() should return correct repository name",
        forge_type.name()
    );
}

test_all_forges!(test_forge_repo_name, test_forge_repo_name_impl);

// Test: Remote config
async fn test_forge_remote_config_impl(
    forge: Box<dyn Forge + Send + Sync>,
    forge_type: ForgeType,
) {
    let config = forge.remote_config();
    assert_eq!(
        config.host,
        forge_type.host(),
        "[{}] remote_config() host mismatch",
        forge_type.name()
    );
    assert_eq!(
        config.owner,
        forge_type.owner(),
        "[{}] remote_config() owner mismatch",
        forge_type.name()
    );
    assert_eq!(
        config.repo,
        forge_type.repo(),
        "[{}] remote_config() repo mismatch",
        forge_type.name()
    );
    assert_eq!(
        config.scheme,
        "https",
        "[{}] remote_config() scheme should be https",
        forge_type.name()
    );
}

test_all_forges!(test_forge_remote_config, test_forge_remote_config_impl);

// Test: Get default branch
async fn test_forge_default_branch_impl(
    forge: Box<dyn Forge + Send + Sync>,
    forge_type: ForgeType,
) {
    let result = forge.default_branch().await;
    assert!(
        result.is_ok(),
        "[{}] default_branch() should succeed: {:?}",
        forge_type.name(),
        result.err()
    );

    let branch = result.unwrap();
    assert!(
        !branch.is_empty(),
        "[{}] default_branch() should not be empty",
        forge_type.name()
    );
    assert!(
        branch == "main" || branch == "master",
        "[{}] default_branch() should be 'main' or 'master', got: {}",
        forge_type.name(),
        branch
    );
}

test_all_forges!(test_forge_default_branch, test_forge_default_branch_impl);

// Test: Get commits
async fn test_forge_get_commits_impl(
    forge: Box<dyn Forge + Send + Sync>,
    forge_type: ForgeType,
) {
    let result = forge.get_commits(".", None).await;
    assert!(
        result.is_ok(),
        "[{}] get_commits() should succeed: {:?}",
        forge_type.name(),
        result.err()
    );

    let commits = result.unwrap();
    assert!(
        !commits.is_empty(),
        "[{}] Repository should have at least one commit",
        forge_type.name()
    );

    // Verify commit structure
    let first_commit = &commits[0];
    assert!(
        !first_commit.id.is_empty(),
        "[{}] Commit ID should not be empty",
        forge_type.name()
    );
    assert!(
        !first_commit.message.is_empty(),
        "[{}] Commit message should not be empty",
        forge_type.name()
    );
    assert!(
        !first_commit.author_name.is_empty(),
        "[{}] Author name should not be empty",
        forge_type.name()
    );
    assert!(
        !first_commit.link.is_empty(),
        "[{}] Commit link should not be empty",
        forge_type.name()
    );
    assert!(
        first_commit.timestamp > 0,
        "[{}] Timestamp should be positive",
        forge_type.name()
    );
}

test_all_forges!(test_forge_get_commits, test_forge_get_commits_impl);

// Test: Get commits with SHA filter
async fn test_forge_get_commits_with_sha_impl(
    forge: Box<dyn Forge + Send + Sync>,
    forge_type: ForgeType,
) {
    // First get all commits
    let all_commits = forge
        .get_commits(".", None)
        .await
        .expect("Failed to get all commits");

    if all_commits.len() >= 2 {
        // Get commits since the second commit
        let since_sha = all_commits[1].id.clone();
        let result = forge.get_commits(".", Some(since_sha)).await;

        assert!(
            result.is_ok(),
            "[{}] get_commits() with SHA should succeed: {:?}",
            forge_type.name(),
            result.err()
        );

        let filtered_commits = result.unwrap();
        assert!(
            filtered_commits.len() <= all_commits.len(),
            "[{}] Filtered commits should be <= total commits",
            forge_type.name()
        );
    } else {
        println!(
            "[{}] Skipping SHA filter test - need at least 2 commits",
            forge_type.name()
        );
    }
}

test_all_forges!(
    test_forge_get_commits_with_sha,
    test_forge_get_commits_with_sha_impl
);

// Test: Get latest tag for prefix
async fn test_forge_get_latest_tag_impl(
    forge: Box<dyn Forge + Send + Sync>,
    forge_type: ForgeType,
) {
    let result = forge.get_latest_tag_for_prefix("v").await;
    assert!(
        result.is_ok(),
        "[{}] get_latest_tag_for_prefix() should succeed: {:?}",
        forge_type.name(),
        result.err()
    );

    match result.unwrap() {
        Some(tag) => {
            assert!(
                tag.name.starts_with("v"),
                "[{}] Tag name should start with 'v'",
                forge_type.name()
            );
            assert!(
                !tag.sha.is_empty(),
                "[{}] Tag SHA should not be empty",
                forge_type.name()
            );
            println!(
                "[{}] Found tag: {} at {}",
                forge_type.name(),
                tag.name,
                tag.sha
            );
        }
        None => {
            println!(
                "[{}] No tags found with prefix 'v' (this is ok for a new repo)",
                forge_type.name()
            );
        }
    }
}

test_all_forges!(test_forge_get_latest_tag, test_forge_get_latest_tag_impl);

// Test: Get latest tag with non-existent prefix
async fn test_forge_get_latest_tag_nonexistent_impl(
    forge: Box<dyn Forge + Send + Sync>,
    forge_type: ForgeType,
) {
    let result = forge
        .get_latest_tag_for_prefix("nonexistent-prefix-xyz-12345")
        .await;
    assert!(
        result.is_ok(),
        "[{}] get_latest_tag_for_prefix() should succeed even for non-existent prefix: {:?}",
        forge_type.name(),
        result.err()
    );
    assert!(
        result.unwrap().is_none(),
        "[{}] Should return None for non-existent prefix",
        forge_type.name()
    );
}

test_all_forges!(
    test_forge_get_latest_tag_nonexistent,
    test_forge_get_latest_tag_nonexistent_impl
);

// Test: Load config
async fn test_forge_load_config_impl(
    forge: Box<dyn Forge + Send + Sync>,
    forge_type: ForgeType,
) {
    let result = forge.load_config().await;

    // Config may or may not exist - both are valid
    match result {
        Ok(config) => {
            assert!(
                !config.packages.is_empty(),
                "[{}] Config should have at least one package",
                forge_type.name()
            );
            println!(
                "[{}] Successfully loaded config with {} packages",
                forge_type.name(),
                config.packages.len()
            );
        }
        Err(e) => {
            println!(
                "[{}] Config file not found (this is ok): {}",
                forge_type.name(),
                e
            );
        }
    }
}

test_all_forges!(test_forge_load_config, test_forge_load_config_impl);

// Test: Get open release PR
async fn test_forge_get_open_release_pr_impl(
    forge: Box<dyn Forge + Send + Sync>,
    forge_type: ForgeType,
) {
    let default_branch = forge
        .default_branch()
        .await
        .expect("Failed to get default branch");

    let req = GetPrRequest {
        head_branch: "releasaurus-release".to_string(),
        base_branch: default_branch,
    };

    let result = forge.get_open_release_pr(req).await;
    assert!(
        result.is_ok(),
        "[{}] get_open_release_pr() should succeed: {:?}",
        forge_type.name(),
        result.err()
    );

    match result.unwrap() {
        Some(pr) => {
            assert!(
                pr.number > 0,
                "[{}] PR number should be positive",
                forge_type.name()
            );
            assert!(
                !pr.sha.is_empty(),
                "[{}] PR SHA should not be empty",
                forge_type.name()
            );
            println!(
                "[{}] Found open release PR #{}",
                forge_type.name(),
                pr.number
            );
        }
        None => {
            println!(
                "[{}] No open release PR found (this is normal)",
                forge_type.name()
            );
        }
    }
}

test_all_forges!(
    test_forge_get_open_release_pr,
    test_forge_get_open_release_pr_impl
);

// Test: Get merged release PR
async fn test_forge_get_merged_release_pr_impl(
    forge: Box<dyn Forge + Send + Sync>,
    forge_type: ForgeType,
) {
    let default_branch = forge.default_branch().await.unwrap();

    let req = GetPrRequest {
        base_branch: default_branch,
        head_branch: "releasaurus-release".into(),
    };
    let result = forge.get_merged_release_pr(req).await;
    assert!(
        result.is_ok(),
        "[{}] get_merged_release_pr() should succeed: {:?}",
        forge_type.name(),
        result.err()
    );

    match result.unwrap() {
        Some(pr) => {
            assert!(
                pr.number > 0,
                "[{}] PR number should be positive",
                forge_type.name()
            );
            assert!(
                !pr.sha.is_empty(),
                "[{}] PR SHA should not be empty",
                forge_type.name()
            );
            println!(
                "[{}] Found merged release PR #{}",
                forge_type.name(),
                pr.number
            );
        }
        None => {
            println!(
                "[{}] No merged release PR found (this is normal)",
                forge_type.name()
            );
        }
    }
}

test_all_forges!(
    test_forge_get_merged_release_pr,
    test_forge_get_merged_release_pr_impl
);

// Test: Integration workflow - chaining multiple operations
async fn test_forge_integration_workflow_impl(
    forge: Box<dyn Forge + Send + Sync>,
    forge_type: ForgeType,
) {
    println!("[{}] Starting integration workflow test", forge_type.name());

    // 1. Get repository info
    let repo_name = forge.repo_name();
    assert_eq!(repo_name, forge_type.repo());
    println!("[{}] ✓ Repository name verified", forge_type.name());

    // 2. Get default branch
    let default_branch = forge
        .default_branch()
        .await
        .expect("Failed to get default branch");
    assert!(!default_branch.is_empty());
    println!(
        "[{}] ✓ Default branch: {}",
        forge_type.name(),
        default_branch
    );

    // 3. Get commits
    let commits = forge
        .get_commits(".", None)
        .await
        .expect("Failed to get commits");
    assert!(!commits.is_empty());
    println!(
        "[{}] ✓ Retrieved {} commits",
        forge_type.name(),
        commits.len()
    );

    // 4. Try to load a file
    let readme_result = forge.get_file_content("README.md").await;
    println!(
        "[{}] ✓ README.md query: {}",
        forge_type.name(),
        if readme_result.is_ok() {
            "success"
        } else {
            "failed"
        }
    );

    // 5. Check for tags
    let tags = forge
        .get_latest_tag_for_prefix("v")
        .await
        .expect("Failed to query tags");
    println!(
        "[{}] ✓ Tags with 'v' prefix: {}",
        forge_type.name(),
        if tags.is_some() { "found" } else { "none" }
    );

    // 6. Check for open PRs
    let pr_req = GetPrRequest {
        head_branch: "releasaurus-release".to_string(),
        base_branch: default_branch.clone(),
    };
    let open_pr = forge
        .get_open_release_pr(pr_req.clone())
        .await
        .expect("Failed to query open PRs");
    println!(
        "[{}] ✓ Open release PR: {}",
        forge_type.name(),
        if open_pr.is_some() { "found" } else { "none" }
    );

    // 7. Check for merged PRs
    let merged_pr = forge
        .get_merged_release_pr(pr_req)
        .await
        .expect("Failed to query merged PRs");
    println!(
        "[{}] ✓ Merged release PR: {}",
        forge_type.name(),
        if merged_pr.is_some() { "found" } else { "none" }
    );

    println!(
        "[{}] Integration workflow test completed successfully",
        forge_type.name()
    );
}

test_all_forges!(
    test_forge_integration_workflow,
    test_forge_integration_workflow_impl
);

// Destructive tests - protected by _internal_e2e_tests feature flag

#[tokio::test]
async fn test_forge_create_pr_github() {
    if env::var("GH_TEST_TOKEN").is_err() {
        println!("Skipping - GH_TEST_TOKEN not set");
        return;
    }
    test_forge_create_pr_impl(ForgeType::GitHub).await;
}

#[tokio::test]
async fn test_forge_create_pr_gitlab() {
    if env::var("GL_TEST_TOKEN").is_err() {
        println!("Skipping - GL_TEST_TOKEN not set");
        return;
    }
    test_forge_create_pr_impl(ForgeType::GitLab).await;
}

#[tokio::test]
async fn test_forge_create_pr_gitea() {
    if env::var("GT_TEST_TOKEN").is_err() {
        println!("Skipping - GT_TEST_TOKEN not set");
        return;
    }
    test_forge_create_pr_impl(ForgeType::Gitea).await;
}

async fn test_forge_create_pr_impl(forge_type: ForgeType) {
    let forge = create_forge(forge_type)
        .await
        .expect("Failed to create forge");
    let default_branch = forge
        .default_branch()
        .await
        .expect("Failed to get default branch");

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let test_branch = format!("test-pr-branch-{}", timestamp);

    // Create a test branch
    let branch_req = CreateBranchRequest {
        branch: test_branch.clone(),
        message: format!("test: create test branch for PR at {}", timestamp),
        file_changes: vec![FileChange {
            path: format!("test-file-{}.txt", timestamp),
            content: format!("Test content for PR created at {}", timestamp),
            update_type: FileUpdateType::Replace,
        }],
    };

    forge
        .create_release_branch(branch_req)
        .await
        .expect("Failed to create test branch");

    // Create the PR
    let pr_req = CreatePrRequest {
        head_branch: test_branch,
        base_branch: default_branch,
        title: format!("[TEST] Test PR {}", timestamp),
        body: format!(
            "# Test PR\n\nCreated by integration tests at {}.\n\nThis can be safely closed.",
            timestamp
        ),
    };

    let result = forge.create_pr(pr_req).await;
    assert!(
        result.is_ok(),
        "[{}] Failed to create PR: {:?}",
        forge_type.name(),
        result.err()
    );

    let pr = result.unwrap();
    assert!(pr.number > 0);
    println!(
        "[{}] Successfully created test PR #{}",
        forge_type.name(),
        pr.number
    );

    // Close the PR to clean up
    match close_pr(forge_type, pr.number).await {
        Ok(_) => {
            println!(
                "[{}] Successfully closed test PR #{}",
                forge_type.name(),
                pr.number
            );
        }
        Err(e) => {
            eprintln!(
                "[{}] Warning: Failed to close PR #{}: {:?}",
                forge_type.name(),
                pr.number,
                e
            );
        }
    }
}
