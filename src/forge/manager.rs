//! Manager that wraps forge implementations
use async_trait::async_trait;
use log::*;
use std::sync::OnceLock;

use crate::{
    Result,
    analyzer::release::Tag,
    config::Config,
    file_loader::FileLoader,
    forge::{
        request::{
            Commit, CreateCommitRequest, CreatePrRequest,
            CreateReleaseBranchRequest, ForgeCommit, GetFileContentRequest,
            GetPrRequest, PrLabelsRequest, PullRequest, ReleaseByTagResponse,
            UpdatePrRequest,
        },
        traits::Forge,
    },
};

pub struct ForgeManager {
    forge: Box<dyn Forge>,
    repo_name: OnceLock<String>,
    default_branch: OnceLock<String>,
    release_link_base_url: OnceLock<String>,
}

impl ForgeManager {
    /// Create Gitea client with token authentication and API base URL
    /// configuration for self-hosted instances.
    pub fn new(forge: Box<dyn Forge>) -> Self {
        Self {
            forge,
            repo_name: OnceLock::new(),
            default_branch: OnceLock::new(),
            release_link_base_url: OnceLock::new(),
        }
    }

    pub fn repo_name(&self) -> &str {
        self.repo_name.get_or_init(|| self.forge.repo_name())
    }

    pub fn release_link_base_url(&self) -> &str {
        self.release_link_base_url
            .get_or_init(|| self.forge.release_link_base_url())
    }

    pub fn default_branch(&self) -> &str {
        self.default_branch
            .get_or_init(|| self.forge.default_branch())
    }

    pub async fn get_file_content(
        &self,
        req: GetFileContentRequest,
    ) -> Result<Option<String>> {
        debug!("Loading file: {} (branch: {:?})", req.path, req.branch);

        let result = self.forge.get_file_content(req).await;

        if let Err(e) = &result {
            error!("Failed to load file: {}", e);
        }

        result
    }

    pub async fn load_config(&self, branch: Option<String>) -> Result<Config> {
        info!("Loading configuration from forge (branch: {:?})", branch);

        let result = self.forge.load_config(branch).await;

        if let Err(e) = &result {
            error!("Failed to load configuration: {}", e);
        }

        result
    }

    pub async fn get_release_by_tag(
        &self,
        tag: &str,
    ) -> Result<ReleaseByTagResponse> {
        self.forge.get_release_by_tag(tag).await
    }

    pub async fn get_latest_tag_for_prefix(
        &self,
        prefix: &str,
    ) -> Result<Option<Tag>> {
        self.forge.get_latest_tag_for_prefix(prefix).await
    }

    pub async fn get_commits(
        &self,
        branch: Option<String>,
        sha: Option<String>,
    ) -> Result<Vec<ForgeCommit>> {
        debug!(
            "getting commits for branch [{:?}] starting from sha: {:?}",
            branch, sha
        );
        self.forge.get_commits(branch, sha).await
    }

    pub async fn get_open_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        info!(
            "Looking for open release PR: base={}, head={}",
            req.base_branch, req.head_branch
        );

        let result = self.forge.get_open_release_pr(req).await;

        match &result {
            Ok(Some(pr)) => info!("Found open PR #{}", pr.number),
            Ok(None) => debug!("No open PR found"),
            Err(e) => error!("Error searching for open PR: {}", e),
        }

        result
    }

    pub async fn get_merged_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        info!(
            "Looking for merged release PR: base={}, head={}",
            req.base_branch, req.head_branch
        );

        let result = self.forge.get_merged_release_pr(req).await;

        match &result {
            Ok(Some(pr)) => info!("Found merged PR #{}", pr.number),
            Ok(None) => warn!("No merged PR found"),
            Err(e) => error!("Error searching for merged PR: {}", e),
        }

        result
    }

    pub async fn create_release_branch(
        &self,
        req: CreateReleaseBranchRequest,
    ) -> Result<Commit> {
        if self.forge.dry_run() {
            warn!("dry_run: would create release branch: req: {:#?}", req);
            return Ok(Commit { sha: "fff".into() });
        }

        info!(
            "Creating release branch: {} from {}",
            req.release_branch, req.base_branch
        );

        let result = self.forge.create_release_branch(req).await;

        match &result {
            Ok(commit) => {
                info!("Created release branch with commit: {}", commit.sha)
            }
            Err(e) => error!("Failed to create release branch: {}", e),
        }

        result
    }

    pub async fn create_commit(
        &self,
        req: CreateCommitRequest,
    ) -> Result<Commit> {
        if self.forge.dry_run() {
            warn!("dry_run: would create commit: req: {:#?}", req);
            return Ok(Commit { sha: "fff".into() });
        }

        info!(
            "Creating commit on branch: {} ({} file changes)",
            req.target_branch,
            req.file_changes.len()
        );

        let result = self.forge.create_commit(req).await;

        match &result {
            Ok(commit) => info!("Created commit: {}", commit.sha),
            Err(e) => error!("Failed to create commit: {}", e),
        }

        result
    }

    pub async fn tag_commit(&self, tag_name: &str, sha: &str) -> Result<()> {
        if self.forge.dry_run() {
            warn!("dry_run: would tag commit: tag={}, sha={}", tag_name, sha);
            return Ok(());
        }

        info!("Tagging commit: tag={}, sha={}", tag_name, sha);

        let result = self.forge.tag_commit(tag_name, sha).await;

        match &result {
            Ok(_) => info!("Successfully created tag: {}", tag_name),
            Err(e) => error!("Failed to create tag {}: {}", tag_name, e),
        }

        result
    }

    pub async fn create_pr(&self, req: CreatePrRequest) -> Result<PullRequest> {
        if self.forge.dry_run() {
            warn!(
                "dry_run: would create PR: {} -> {}",
                req.head_branch, req.base_branch
            );
            return Ok(PullRequest {
                number: 0,
                sha: "fff".into(),
                body: req.body,
            });
        }

        info!(
            "Creating pull request: {} -> {}",
            req.head_branch, req.base_branch
        );

        let result = self.forge.create_pr(req).await;

        match &result {
            Ok(pr) => info!("Created pull request #{}", pr.number),
            Err(e) => error!("Failed to create pull request: {}", e),
        }

        result
    }

    pub async fn update_pr(&self, req: UpdatePrRequest) -> Result<()> {
        if self.forge.dry_run() {
            warn!("dry_run: would update PR: req: {:#?}", req);
            return Ok(());
        }

        info!("Updating pull request #{}", req.pr_number);

        let result = self.forge.update_pr(req).await;

        if let Err(e) = &result {
            error!("Failed to update PR: {}", e);
        }

        result
    }

    pub async fn replace_pr_labels(&self, req: PrLabelsRequest) -> Result<()> {
        if self.forge.dry_run() {
            warn!(
                "dry_run: would replace PR #{} labels with: {:?}",
                req.pr_number, req.labels
            );
            return Ok(());
        }

        info!(
            "Replacing labels on PR #{} with: {:?}",
            req.pr_number, req.labels
        );

        let result = self.forge.replace_pr_labels(req).await;

        if let Err(e) = &result {
            error!("Failed to update labels on PR: {}", e);
        }

        result
    }

    pub async fn create_release(
        &self,
        tag: &str,
        sha: &str,
        notes: &str,
    ) -> Result<()> {
        if self.forge.dry_run() {
            warn!(
                "dry_run: would create release: tag: {tag}, sha: {sha}, notes {notes}"
            );
            return Ok(());
        }

        info!("Creating release: tag={}, sha={}", tag, sha);

        let result = self.forge.create_release(tag, sha, notes).await;

        match &result {
            Ok(_) => info!("Successfully created release: {}", tag),
            Err(e) => error!("Failed to create release {}: {}", tag, e),
        }

        result
    }
}

#[async_trait]
impl FileLoader for ForgeManager {
    async fn load_file(
        &self,
        branch: Option<String>,
        path: String,
    ) -> Result<Option<String>> {
        self.get_file_content(GetFileContentRequest { branch, path })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        file_loader::FileLoader,
        forge::{request::GetFileContentRequest, traits::MockForge},
    };

    #[tokio::test]
    async fn file_loader_returns_file_content() {
        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_get_file_content()
            .with(mockall::predicate::eq(GetFileContentRequest {
                branch: Some("main".to_string()),
                path: "package.json".to_string(),
            }))
            .returning(|_| Ok(Some(r#"{"version":"1.0.0"}"#.to_string())));

        let manager = ForgeManager::new(Box::new(mock_forge));
        let result = manager
            .load_file(Some("main".to_string()), "package.json".to_string())
            .await
            .unwrap();

        assert!(result.is_some());
        assert!(result.unwrap().contains("1.0.0"));
    }

    #[tokio::test]
    async fn file_loader_returns_none_when_file_not_found() {
        let mut mock_forge = MockForge::new();
        mock_forge.expect_get_file_content().returning(|_| Ok(None));

        let manager = ForgeManager::new(Box::new(mock_forge));
        let result = manager
            .load_file(None, "missing.txt".to_string())
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn dry_run_prevents_create_release_branch() {
        let mut mock_forge = MockForge::new();
        mock_forge.expect_dry_run().returning(|| true);

        let manager = ForgeManager::new(Box::new(mock_forge));

        let req = CreateReleaseBranchRequest {
            base_branch: "main".into(),
            release_branch: "release-branch".into(),
            message: "chore: release".into(),
            file_changes: vec![],
        };
        let result = manager.create_release_branch(req).await.unwrap();

        assert_eq!(result.sha, "fff");
    }

    #[tokio::test]
    async fn dry_run_prevents_tag_commit() {
        let mut mock_forge = MockForge::new();
        mock_forge.expect_dry_run().returning(|| true);

        let manager = ForgeManager::new(Box::new(mock_forge));
        manager.tag_commit("v1.0.0", "abc123").await.unwrap();
    }

    #[tokio::test]
    async fn dry_run_prevents_create_pr() {
        let mut mock_forge = MockForge::new();
        mock_forge.expect_dry_run().returning(|| true);

        let manager = ForgeManager::new(Box::new(mock_forge));
        let req = CreatePrRequest {
            title: "test".to_string(),
            body: "test body".to_string(),
            head_branch: "branch".to_string(),
            base_branch: "main".to_string(),
        };
        let result = manager.create_pr(req).await.unwrap();

        assert_eq!(result.number, 0);
        assert_eq!(result.sha, "fff");
    }

    #[tokio::test]
    async fn dry_run_prevents_update_pr() {
        let mut mock_forge = MockForge::new();
        mock_forge.expect_dry_run().returning(|| true);

        let manager = ForgeManager::new(Box::new(mock_forge));
        let req = UpdatePrRequest {
            pr_number: 42,
            title: "Updated title".to_string(),
            body: "Updated body".to_string(),
        };
        manager.update_pr(req).await.unwrap();
    }

    #[tokio::test]
    async fn dry_run_prevents_replace_pr_labels() {
        let mut mock_forge = MockForge::new();
        mock_forge.expect_dry_run().returning(|| true);

        let manager = ForgeManager::new(Box::new(mock_forge));
        let req = PrLabelsRequest {
            pr_number: 42,
            labels: vec!["release".to_string()],
        };
        manager.replace_pr_labels(req).await.unwrap();
    }

    #[tokio::test]
    async fn dry_run_prevents_create_release() {
        let mut mock_forge = MockForge::new();
        mock_forge.expect_dry_run().returning(|| true);

        let manager = ForgeManager::new(Box::new(mock_forge));
        manager
            .create_release("v1.0.0", "abc123", "Release notes")
            .await
            .unwrap();
    }
}
