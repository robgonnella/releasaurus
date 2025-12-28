use log::info;
use semver::Version;
use tokio::time::{Duration, sleep};

use crate::{
    ReleasaurusError,
    config::Config,
    forge::{
        config::PENDING_LABEL,
        manager::ForgeManager,
        request::{
            CreateCommitRequest, CreatePrRequest, CreateReleaseBranchRequest,
            FileChange, FileUpdateType, GetFileContentRequest, GetPrRequest,
            PrLabelsRequest,
        },
        tests::common::traits::ForgeTestHelper,
    },
};

pub async fn run_forge_test(
    forge: &ForgeManager,
    helper: &dyn ForgeTestHelper,
    padding: Duration,
) {
    info!("resetting repository");
    helper.reset().await.unwrap();

    let default_branch = forge.default_branch();
    let release_branch = "release-branch";
    let test_file_path = "test.txt";
    let test_file_content = "test content";

    ////////////////////////////////////////////////////////////////////////////
    // get_file_content: expect -> None
    ////////////////////////////////////////////////////////////////////////////
    info!("looking for non-existent file content");
    let get_file_req = GetFileContentRequest {
        branch: Some(default_branch.to_string()),
        path: test_file_path.to_string(),
    };
    let file_content = forge.get_file_content(get_file_req).await.unwrap();
    assert!(file_content.is_none());

    ////////////////////////////////////////////////////////////////////////////
    // create_commit -> succeeds
    ////////////////////////////////////////////////////////////////////////////
    info!("creating commit with new file content");
    let create_commit_req = CreateCommitRequest {
        target_branch: default_branch.to_string(),
        message: "fix: test fix commit".into(),
        file_changes: vec![FileChange {
            content: test_file_content.to_string(),
            path: test_file_path.to_string(),
            update_type: FileUpdateType::Replace,
        }],
    };

    let created_commit = forge.create_commit(create_commit_req).await.unwrap();
    assert!(!created_commit.sha.is_empty());
    sleep(padding).await;

    ////////////////////////////////////////////////////////////////////////////
    // get_commits -> succeeds
    ////////////////////////////////////////////////////////////////////////////

    let commits = forge.get_commits(None, None).await.unwrap();
    assert_eq!(commits.len(), 2);

    ////////////////////////////////////////////////////////////////////////////
    // re-get file content: expect -> content
    ////////////////////////////////////////////////////////////////////////////
    info!("looking for existing file content");
    let get_file_req = GetFileContentRequest {
        branch: Some(default_branch.to_string()),
        path: test_file_path.to_string(),
    };
    let file_content = forge.get_file_content(get_file_req).await.unwrap();
    assert!(file_content.is_some());
    let file_content = file_content.unwrap();
    assert_eq!(file_content, test_file_content);

    ////////////////////////////////////////////////////////////////////////////
    // create_release_branch with changelog file change -> succeeds
    ////////////////////////////////////////////////////////////////////////////
    info!("creating release-branch");
    let create_release_branch_req = CreateReleaseBranchRequest {
        base_branch: default_branch.to_string(),
        message: "chore(main): release".into(),
        release_branch: release_branch.to_string(),
        file_changes: vec![FileChange {
            content: format!("# Changelog - {}", created_commit.sha),
            path: "CHANGELOG.md".into(),
            update_type: FileUpdateType::Prepend,
        }],
    };

    let release_commit = forge
        .create_release_branch(create_release_branch_req)
        .await
        .unwrap();
    assert!(!release_commit.sha.is_empty());
    sleep(padding).await;

    ////////////////////////////////////////////////////////////////////////////
    // get_open_release_pr: expect -> None
    ////////////////////////////////////////////////////////////////////////////
    info!("getting non-existent open PR");
    let get_pr_req = GetPrRequest {
        base_branch: default_branch.to_string(),
        head_branch: release_branch.to_string(),
    };

    let open_pr = forge.get_open_release_pr(get_pr_req).await.unwrap();
    assert!(open_pr.is_none());

    ////////////////////////////////////////////////////////////////////////////
    // create_pr from release_branch -> succeeds
    ////////////////////////////////////////////////////////////////////////////
    info!("creating PR");
    let create_pr_req = CreatePrRequest {
        base_branch: default_branch.to_string(),
        body: "Test PR".into(),
        head_branch: release_branch.to_string(),
        title: "Test PR".into(),
    };

    let release_pr = forge.create_pr(create_pr_req).await.unwrap();
    assert_ne!(release_pr.number, 0);
    assert!(!release_pr.body.is_empty());
    assert!(!release_pr.sha.is_empty());
    sleep(padding).await;

    ////////////////////////////////////////////////////////////////////////////
    // replace_pr_labels -> succeeds
    ////////////////////////////////////////////////////////////////////////////
    info!("replacing PR labels");
    let replace_labels_req = PrLabelsRequest {
        labels: vec![PENDING_LABEL.into()],
        pr_number: release_pr.number,
    };
    forge.replace_pr_labels(replace_labels_req).await.unwrap();
    sleep(padding).await;

    ////////////////////////////////////////////////////////////////////////////
    // get_open_release_pr -> Found
    ////////////////////////////////////////////////////////////////////////////
    info!("looking for newly created open PR");
    let get_pr_req = GetPrRequest {
        base_branch: default_branch.to_string(),
        head_branch: release_branch.to_string(),
    };
    let open_pr = forge.get_open_release_pr(get_pr_req).await.unwrap();
    assert!(open_pr.is_some());
    let open_pr = open_pr.unwrap();
    assert_ne!(open_pr.number, 0);
    assert!(!open_pr.body.is_empty());
    assert!(!open_pr.sha.is_empty());

    ////////////////////////////////////////////////////////////////////////////
    // get_merged_release_pr -> None
    ////////////////////////////////////////////////////////////////////////////
    info!("looking for non-existent merged PR");
    let get_pr_req = GetPrRequest {
        base_branch: default_branch.to_string(),
        head_branch: release_branch.to_string(),
    };
    let merged_pr = forge.get_merged_release_pr(get_pr_req).await.unwrap();
    assert!(merged_pr.is_none());

    ////////////////////////////////////////////////////////////////////////////
    // merge release PR -> helper succeeds
    ////////////////////////////////////////////////////////////////////////////
    info!("merging release PR via helper");
    helper.merge_pr(release_pr.number).await.unwrap();
    sleep(padding).await;

    ////////////////////////////////////////////////////////////////////////////
    // get_merged_release_pr -> Found
    ////////////////////////////////////////////////////////////////////////////
    info!("looking for newly merged release PR");
    let get_pr_req = GetPrRequest {
        base_branch: default_branch.to_string(),
        head_branch: release_branch.to_string(),
    };
    let merged_pr = forge.get_merged_release_pr(get_pr_req).await.unwrap();
    assert!(merged_pr.is_some());
    let merged_pr = merged_pr.unwrap();
    assert_ne!(merged_pr.number, 0);
    assert!(!merged_pr.sha.is_empty());
    assert!(!merged_pr.body.is_empty());

    ////////////////////////////////////////////////////////////////////////////
    // get_latest_tag_for_prefix -> None
    ////////////////////////////////////////////////////////////////////////////
    info!("looking for non-existent tag by prefix");
    let semver = "1.1.0";
    let tag = format!("v{}", semver);
    let current_tag = forge.get_latest_tag_for_prefix("v").await.unwrap();
    assert!(current_tag.is_none());

    ////////////////////////////////////////////////////////////////////////////
    // tag_commit -> succeeds
    ////////////////////////////////////////////////////////////////////////////
    info!("tagging commit");
    forge.tag_commit(&tag, &merged_pr.sha).await.unwrap();
    sleep(padding).await;

    ////////////////////////////////////////////////////////////////////////////
    // get_latest_tag_for_prefix -> Found
    ////////////////////////////////////////////////////////////////////////////
    info!("looking for newly tagged commit by prefix");
    let current_tag = forge.get_latest_tag_for_prefix("v").await.unwrap();
    assert!(current_tag.is_some());
    let current_tag = current_tag.unwrap();
    assert_eq!(current_tag.name, tag);
    assert_eq!(current_tag.semver, Version::parse(semver).unwrap());
    assert_eq!(current_tag.sha, merged_pr.sha);

    ////////////////////////////////////////////////////////////////////////////
    // get_release_by_tag -> Err Not Found
    ////////////////////////////////////////////////////////////////////////////
    info!("getting non-existent release by tag name");
    let err = forge
        .get_release_by_tag(&current_tag.name)
        .await
        .unwrap_err();
    assert!(matches!(err, ReleasaurusError::ForgeError(_)));

    ////////////////////////////////////////////////////////////////////////////
    // create_release -> succeeds
    ////////////////////////////////////////////////////////////////////////////
    info!("creating release for tag");
    forge
        .create_release(&current_tag.name, &current_tag.sha, "release notes")
        .await
        .unwrap();
    sleep(padding).await;

    ////////////////////////////////////////////////////////////////////////////
    // get_release_by_tag -> Found
    ////////////////////////////////////////////////////////////////////////////
    info!("getting newly created release by tag name");
    let release = forge.get_release_by_tag(&current_tag.name).await.unwrap();
    assert_eq!(release.tag, current_tag.name);

    ////////////////////////////////////////////////////////////////////////////
    // load_config -> Default::default()
    ////////////////////////////////////////////////////////////////////////////
    info!("loading non-existent config file");
    let config = forge.load_config(None).await.unwrap();
    assert_eq!(
        config.packages[0].workspace_root,
        Config::default().packages[0].workspace_root
    );
    assert_eq!(config.packages[0].path, Config::default().packages[0].path);

    ////////////////////////////////////////////////////////////////////////////
    // load_config -> Found config
    ////////////////////////////////////////////////////////////////////////////
    info!("creating commit to add releasaurus config file");
    let releasaurus_toml_content = r#"
    [[package]]
    name = "test-package"
    workspace_root = "packages"
    path = "test-package"
    "#;

    let create_commit_req = CreateCommitRequest {
        target_branch: default_branch.to_string(),
        message: "chore: adds releasaurus.toml".into(),
        file_changes: vec![FileChange {
            content: releasaurus_toml_content.to_string(),
            path: "releasaurus.toml".to_string(),
            update_type: FileUpdateType::Replace,
        }],
    };

    info!("loading newly created releasaurus config file");
    let created_commit = forge.create_commit(create_commit_req).await.unwrap();
    assert!(!created_commit.sha.is_empty());
    sleep(padding).await;

    let config = forge
        .load_config(Some(default_branch.to_string()))
        .await
        .unwrap();

    assert_eq!(config.packages[0].name, "test-package");
    assert_eq!(config.packages[0].workspace_root, "packages");
    assert_eq!(config.packages[0].path, "test-package");
}
