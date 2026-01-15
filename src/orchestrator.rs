use color_eyre::eyre::{OptionExt, eyre};
use derive_builder::Builder;
use regex::Regex;
use serde::Serialize;
use std::{path::Path, rc::Rc, sync::LazyLock};
use tokio::fs;

use crate::{
    ReleasaurusError, ResolvedPackage, Result,
    forge::{
        config::{DEFAULT_PR_BRANCH_PREFIX, PENDING_LABEL, TAGGED_LABEL},
        manager::ForgeManager,
        request::{
            CreateCommitRequest, GetPrRequest, PrLabelsRequest, PullRequest,
            ReleaseByTagResponse, UpdatePrRequest,
        },
    },
    orchestrator::{
        config::OrchestratorConfig,
        core::{Core, PRMetadata},
        package::{
            releasable::SerializableReleasablePackage,
            resolved::ResolvedPackageHash,
        },
    },
};

pub mod commits;
pub mod config;
pub mod core;
pub mod package;

static METADATA_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?ms)^<!--(?<metadata>.*?)-->\n*<details"#).unwrap()
});

/// Information about a package's current release
#[derive(Serialize)]
pub struct CurrentRelease {
    name: String,
    tag: String,
    sha: String,
    notes: String,
}

#[derive(Builder)]
#[builder(setter(into), build_fn(private, name = "_build"))]
pub struct OrchestratorParams {
    pub config: Rc<OrchestratorConfig>,
    pub package_configs: Rc<ResolvedPackageHash>,
    pub forge: Rc<ForgeManager>,
}

impl OrchestratorParamsBuilder {
    pub fn build(&self) -> Result<Orchestrator> {
        let params = self._build().map_err(|e| {
            ReleasaurusError::invalid_config(format!(
                "Failed to build release manager: {}",
                e
            ))
        })?;
        Orchestrator::new(params)
    }
}

pub struct Orchestrator {
    config: Rc<OrchestratorConfig>,
    package_configs: Rc<ResolvedPackageHash>,
    forge: Rc<ForgeManager>,
    core: Core,
}

impl Orchestrator {
    pub fn builder() -> OrchestratorParamsBuilder {
        OrchestratorParamsBuilder::default()
    }

    pub fn new(params: OrchestratorParams) -> Result<Self> {
        Ok(Self {
            config: Rc::clone(&params.config),
            package_configs: Rc::clone(&params.package_configs),
            forge: Rc::clone(&params.forge),
            core: Core::new(
                Rc::clone(&params.config),
                Rc::clone(&params.forge),
                Rc::clone(&params.package_configs),
            ),
        })
    }

    pub async fn recompile_notes_from_release_file(
        &self,
        file: &str,
    ) -> Result<Vec<SerializableReleasablePackage>> {
        let file_path = Path::new(&file);

        if !file_path.exists() {
            return Err(ReleasaurusError::Other(eyre!(format!(
                "file path does not exist: {}",
                file
            ))));
        }

        let content = fs::read_to_string(file_path).await?;

        let mut packages: Vec<SerializableReleasablePackage> =
            serde_json::from_str(&content)?;

        // Compile template once before loop to avoid O(n) template compilation
        let mut tera = tera::Tera::default();
        tera.add_raw_template("changelog", &self.config.changelog.body)?;

        for package in packages.iter_mut() {
            let context = tera::Context::from_serialize(&package.release)?;
            package.release.notes = tera.render("changelog", &context)?;
        }

        Ok(packages)
    }

    pub async fn create_release_prs(&self) -> Result<()> {
        let prepared = self.core.prepare_packages().await?;

        let analyzed = self.core.analyze_packages(prepared)?;

        let releasable = self.core.releasable_packages(analyzed).await?;

        log::info!("releasable packages: {:#?}", releasable);

        let pr_packages =
            self.core.release_pr_packages_by_branch(releasable)?;

        if pr_packages.is_empty() {
            return Ok(());
        }

        let requests = self.core.create_pr_branches(pr_packages).await?;

        for request in requests {
            let pr = if let Some(pr) = self
                .forge
                .get_open_release_pr(GetPrRequest {
                    head_branch: request.head_branch.clone(),
                    base_branch: request.base_branch.clone(),
                })
                .await?
            {
                self.forge
                    .update_pr(UpdatePrRequest {
                        pr_number: pr.number,
                        title: request.title,
                        body: request.body,
                    })
                    .await?;
                pr
            } else {
                self.forge.create_pr(request).await?
            };

            self.forge
                .replace_pr_labels(PrLabelsRequest {
                    pr_number: pr.number,
                    labels: vec![PENDING_LABEL.into()],
                })
                .await?;
        }

        Ok(())
    }

    pub async fn create_releases(&self) -> Result<()> {
        let mut auto_start_packages: Vec<String> = vec![];
        let base_branch = self.config.base_branch.clone();

        for (name, package) in self.package_configs.hash().iter() {
            let mut release_branch =
                format!("{DEFAULT_PR_BRANCH_PREFIX}-{base_branch}");

            if self.config.separate_pull_requests {
                release_branch = format!(
                    "{DEFAULT_PR_BRANCH_PREFIX}-{base_branch}-{}",
                    package.name
                );
            }

            let req = GetPrRequest {
                base_branch: base_branch.clone(),
                head_branch: release_branch.to_string(),
            };

            if let Some(merged_pr) =
                self.forge.get_merged_release_pr(req).await?
            {
                self.create_package_release(package, &merged_pr).await?;

                let req = PrLabelsRequest {
                    pr_number: merged_pr.number,
                    labels: vec![TAGGED_LABEL.into()],
                };

                self.forge.replace_pr_labels(req).await?;

                if package.auto_start_next {
                    auto_start_packages.push(name.clone());
                };
            }
        }

        if !auto_start_packages.is_empty() {
            self.start_next_release(Some(auto_start_packages)).await?;
        }

        Ok(())
    }

    pub async fn start_next_release(
        &self,
        targets: Option<Vec<String>>,
    ) -> Result<()> {
        let prepared = self
            .core
            .generate_prepared_with_dummy_commit(targets)
            .await?;

        let analyzed = self.core.analyze_packages(prepared)?;

        let releasable = self.core.releasable_packages(analyzed).await?;

        let pr_packages = self.core.release_pr_packages(releasable)?;

        for pkg in pr_packages {
            log::info!("updating manifest files for package: {}", pkg.name);

            let req = CreateCommitRequest {
                target_branch: self.config.base_branch.to_string(),
                file_changes: pkg.file_changes,
                message: format!(
                    "chore({}): bump patch version {} - {}",
                    self.config.base_branch, pkg.name, pkg.tag.semver
                ),
            };

            let commit = self.forge.create_commit(req).await?;

            log::info!("created commit: {}", commit.sha);
        }

        Ok(())
    }

    /// Fetches the most recent release for each package
    /// Packages without releases are omitted
    pub async fn get_current_releases(
        &self,
        target_package: Option<String>,
    ) -> Result<Vec<CurrentRelease>> {
        let mut releases = vec![];

        for (name, package) in self.package_configs.hash().iter() {
            if let Some(target) = target_package.as_ref()
                && name != target
            {
                continue;
            }

            let current = self
                .forge
                .get_latest_tag_for_prefix(&package.tag_prefix)
                .await?;

            if let Some(tag) = current {
                let data = self.forge.get_release_by_tag(&tag.name).await?;
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

    /// Fetches projected next release information
    pub async fn get_next_releases(
        &self,
        package: Option<String>,
    ) -> Result<Vec<SerializableReleasablePackage>> {
        let prepared = self.core.prepare_packages().await?;

        let analyzed = self.core.analyze_packages(prepared)?;

        let mut releasable = self
            .core
            .full_serializable_releasable_packages(analyzed)
            .await?;

        if let Some(package) = package {
            releasable = releasable
                .into_iter()
                .filter(|p| p.name == package)
                .collect::<Vec<SerializableReleasablePackage>>();
        }

        Ok(releasable)
    }

    pub async fn get_release_by_tag(
        &self,
        tag: &str,
    ) -> Result<ReleaseByTagResponse> {
        self.forge.get_release_by_tag(tag).await
    }

    ////////////////////////////////////////////////////////////////////////////
    //// private
    ////////////////////////////////////////////////////////////////////////////

    /// Creates release for a targeted package and merged PR
    async fn create_package_release(
        &self,
        package: &ResolvedPackage,
        merged_pr: &PullRequest,
    ) -> Result<()> {
        let meta_caps = METADATA_REGEX.captures_iter(&merged_pr.body);

        let mut metadata = None;

        for cap in meta_caps {
            let metadata_str = cap
                .name("metadata")
                .ok_or_eyre("failed to parse metadata from PR body")?
                .as_str();

            log::debug!("parsing metadata string: {:#?}", metadata_str);

            let json: PRMetadata = serde_json::from_str(metadata_str)?;
            let pkg_meta = json.metadata;

            if pkg_meta.name == package.name {
                metadata = Some(pkg_meta);
                break;
            }
        }

        let metadata_err = format!(
            "failed to find metadata for package {} in pr {}",
            package.name, merged_pr.number,
        );

        let metadata = metadata.ok_or_eyre(metadata_err)?;

        log::debug!(
            "found package metadata from pr {}: {:#?}",
            merged_pr.number,
            metadata
        );

        log::info!(
            "tagging commit: tag: {}, sha: {}",
            metadata.tag,
            merged_pr.sha
        );

        self.forge.tag_commit(&metadata.tag, &merged_pr.sha).await?;

        log::info!(
            "creating release: tag: {}, sha: {}",
            metadata.tag,
            merged_pr.sha
        );

        self.forge
            .create_release(
                &metadata.tag,
                &merged_pr.sha,
                metadata.notes.trim(),
            )
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests;
