//! Manager that wraps forge implementations
use log::*;

use crate::{
    Result,
    analyzer::release::Tag,
    config::Config,
    forge::{
        config::RemoteConfig,
        request::{
            Commit, CreateCommitRequest, CreatePrRequest,
            CreateReleaseBranchRequest, ForgeCommit, GetFileContentRequest,
            GetPrRequest, PrLabelsRequest, PullRequest, ReleaseByTagResponse,
            UpdatePrRequest,
        },
        traits::Forge,
    },
    updater::manager::{ManifestFile, ManifestTarget},
};

pub struct ForgeManager {
    forge: Box<dyn Forge>,
    remote_config: RemoteConfig,
}

impl ForgeManager {
    /// Create Gitea client with token authentication and API base URL
    /// configuration for self-hosted instances.
    pub fn new(forge: Box<dyn Forge>) -> Self {
        let remote_config = forge.remote_config();
        Self {
            forge,
            remote_config,
        }
    }

    pub fn repo_name(&self) -> String {
        self.forge.repo_name()
    }

    pub fn remote_config(&self) -> RemoteConfig {
        self.remote_config.clone()
    }

    pub fn default_branch(&self) -> String {
        self.forge.default_branch()
    }

    pub async fn get_file_content(
        &self,
        req: GetFileContentRequest,
    ) -> Result<Option<String>> {
        self.forge.get_file_content(req).await
    }

    pub async fn load_config(&self, branch: Option<String>) -> Result<Config> {
        self.forge.load_config(branch).await
    }

    pub async fn get_release_by_tag(
        &self,
        tag: &str,
    ) -> Result<ReleaseByTagResponse> {
        self.forge.get_release_by_tag(tag).await
    }

    pub async fn load_manifest_targets(
        &self,
        branch: Option<String>,
        targets: Vec<ManifestTarget>,
    ) -> Result<Option<Vec<ManifestFile>>> {
        if targets.is_empty() {
            return Ok(None);
        }

        let mut manifests = vec![];

        for target in targets.iter() {
            debug!("looking for manifest target: {}", target.path);
            if let Some(content) = self
                .get_file_content(GetFileContentRequest {
                    branch: branch.clone(),
                    path: target.path.to_string(),
                })
                .await?
            {
                info!("found manifest target: {}", target.path);
                manifests.push(ManifestFile {
                    is_workspace: target.is_workspace,
                    path: target.path.clone(),
                    basename: target.basename.clone(),
                    content,
                });
            } else {
                debug!("no file found for path: {}", target.path);
            }
        }

        if manifests.is_empty() {
            return Ok(None);
        }

        Ok(Some(manifests))
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
        self.forge.get_open_release_pr(req).await
    }

    pub async fn get_merged_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        self.forge.get_merged_release_pr(req).await
    }

    pub async fn create_release_branch(
        &self,
        req: CreateReleaseBranchRequest,
    ) -> Result<Commit> {
        if self.remote_config.dry_run {
            warn!("dry_run: would create release branch: req: {:#?}", req);
            return Ok(Commit { sha: "fff".into() });
        }
        self.forge.create_release_branch(req).await
    }

    pub async fn create_commit(
        &self,
        req: CreateCommitRequest,
    ) -> Result<Commit> {
        if self.remote_config.dry_run {
            warn!("dry_run: would create commit: req: {:#?}", req);
            return Ok(Commit { sha: "fff".into() });
        }
        self.forge.create_commit(req).await
    }

    pub async fn tag_commit(&self, tag_name: &str, sha: &str) -> Result<()> {
        if self.remote_config.dry_run {
            warn!(
                "dry_run: would tag commit: tag_name: {tag_name}, sha: {sha}"
            );
            return Ok(());
        }

        self.forge.tag_commit(tag_name, sha).await
    }

    pub async fn create_pr(&self, req: CreatePrRequest) -> Result<PullRequest> {
        if self.remote_config.dry_run {
            warn!("dry_run: would create PR: req: {:#?}", req);
            return Ok(PullRequest {
                number: 0,
                sha: "fff".into(),
                body: req.body,
            });
        }

        self.forge.create_pr(req).await
    }

    pub async fn update_pr(&self, req: UpdatePrRequest) -> Result<()> {
        if self.remote_config.dry_run {
            warn!("dry_run: would update PR: req: {:#?}", req);
            return Ok(());
        }
        self.forge.update_pr(req).await
    }

    pub async fn replace_pr_labels(&self, req: PrLabelsRequest) -> Result<()> {
        if self.remote_config.dry_run {
            warn!("dry_run: would replace PR labels: req: {:#?}", req);
            return Ok(());
        }
        self.forge.replace_pr_labels(req).await
    }

    pub async fn create_release(
        &self,
        tag: &str,
        sha: &str,
        notes: &str,
    ) -> Result<()> {
        if self.remote_config.dry_run {
            warn!(
                "dry_run: would create release: tag: {tag}, sha: {sha}, notes {notes}"
            );
            return Ok(());
        }

        self.forge.create_release(tag, sha, notes).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::{request::GetFileContentRequest, traits::MockForge};

    #[tokio::test]
    async fn load_manifest_targets_returns_none_for_empty_targets() {
        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_remote_config()
            .returning(RemoteConfig::default);

        let manager = ForgeManager::new(Box::new(mock_forge));
        let result = manager.load_manifest_targets(None, vec![]).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn load_manifest_targets_returns_none_when_no_files_exist() {
        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_remote_config()
            .returning(RemoteConfig::default);
        mock_forge.expect_get_file_content().returning(|_| Ok(None));

        let manager = ForgeManager::new(Box::new(mock_forge));
        let targets = vec![ManifestTarget {
            is_workspace: false,
            path: "package.json".to_string(),
            basename: "package.json".to_string(),
        }];
        let result =
            manager.load_manifest_targets(None, targets).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn load_manifest_targets_returns_manifests_when_files_exist() {
        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_remote_config()
            .returning(RemoteConfig::default);
        mock_forge
            .expect_get_file_content()
            .with(mockall::predicate::eq(GetFileContentRequest {
                branch: None,
                path: "package.json".to_string(),
            }))
            .returning(|_| Ok(Some(r#"{"version":"1.0.0"}"#.to_string())));
        mock_forge
            .expect_get_file_content()
            .with(mockall::predicate::eq(GetFileContentRequest {
                branch: None,
                path: "Cargo.toml".to_string(),
            }))
            .returning(|_| Ok(None));

        let manager = ForgeManager::new(Box::new(mock_forge));
        let targets = vec![
            ManifestTarget {
                is_workspace: false,
                path: "package.json".to_string(),
                basename: "package.json".to_string(),
            },
            ManifestTarget {
                is_workspace: false,
                path: "Cargo.toml".to_string(),
                basename: "Cargo.toml".to_string(),
            },
        ];
        let result =
            manager.load_manifest_targets(None, targets).await.unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].basename, "package.json");
        assert!(manifests[0].content.contains("1.0.0"));
    }

    #[tokio::test]
    async fn dry_run_prevents_create_release_branch() {
        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_remote_config()
            .returning(|| RemoteConfig {
                dry_run: true,
                ..Default::default()
            });

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
        mock_forge
            .expect_remote_config()
            .returning(|| RemoteConfig {
                dry_run: true,
                ..Default::default()
            });

        let manager = ForgeManager::new(Box::new(mock_forge));
        let result = manager.tag_commit("v1.0.0", "abc123").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn dry_run_prevents_create_pr() {
        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_remote_config()
            .returning(|| RemoteConfig {
                dry_run: true,
                ..Default::default()
            });

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
        mock_forge
            .expect_remote_config()
            .returning(|| RemoteConfig {
                dry_run: true,
                ..Default::default()
            });

        let manager = ForgeManager::new(Box::new(mock_forge));
        let req = UpdatePrRequest {
            pr_number: 42,
            title: "Updated title".to_string(),
            body: "Updated body".to_string(),
        };
        let result = manager.update_pr(req).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn dry_run_prevents_replace_pr_labels() {
        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_remote_config()
            .returning(|| RemoteConfig {
                dry_run: true,
                ..Default::default()
            });

        let manager = ForgeManager::new(Box::new(mock_forge));
        let req = PrLabelsRequest {
            pr_number: 42,
            labels: vec!["release".to_string()],
        };
        let result = manager.replace_pr_labels(req).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn dry_run_prevents_create_release() {
        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_remote_config()
            .returning(|| RemoteConfig {
                dry_run: true,
                ..Default::default()
            });

        let manager = ForgeManager::new(Box::new(mock_forge));
        let result = manager
            .create_release("v1.0.0", "abc123", "Release notes")
            .await;

        assert!(result.is_ok());
    }
}
