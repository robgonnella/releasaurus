use color_eyre::eyre::eyre;
use derive_builder::Builder;
use serde::Serialize;
use std::{path::Path, rc::Rc};
use tokio::fs;

use crate::{
    config::resolved::ResolvedConfig,
    forge::{
        config::{DEFAULT_PR_BRANCH_PREFIX, PENDING_LABEL, TAGGED_LABEL},
        manager::ForgeManager,
        request::{
            CreateCommitRequest, GetPrRequest, PrLabelsRequest, PullRequest,
            ReleaseByTagResponse, UpdatePrRequest,
        },
    },
    orchestrator::{
        package_processor::{PackageProcessor, PrBranchResult},
        pr_body::{parse_legacy_pr_body, parse_pr_body},
    },
    packages::{
        releasable::SerializableReleasablePackage, resolved::ResolvedPackage,
        resolved_hash::ResolvedPackageHash,
    },
    result::{ReleasaurusError, Result},
};

pub mod commit_fetcher;
pub mod package_processor;
pub mod pr_body;

/// Information about a package's current release
#[derive(Serialize)]
pub struct CurrentRelease {
    name: String,
    tag: String,
    sha: String,
    notes: String,
}

/// Builder parameters for constructing an [`Orchestrator`].
/// Use [`Orchestrator::builder`] to create one.
#[derive(Builder)]
#[builder(setter(into), build_fn(private, name = "_build"))]
pub struct OrchestratorParams {
    pub config: Rc<ResolvedConfig>,
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

/// Entry point for all release pipeline operations.
///
/// `Orchestrator` coordinates the full release workflow: analyzing
/// commits, generating changelogs, creating release PRs, tagging
/// commits, and publishing releases. Construct it with
/// [`Orchestrator::builder`].
///
/// See the [crate-level quick start][crate] for a complete setup
/// example including [`ForgeManager`] and
/// [`ResolvedConfig`] construction.
pub struct Orchestrator {
    config: Rc<ResolvedConfig>,
    package_configs: Rc<ResolvedPackageHash>,
    forge: Rc<ForgeManager>,
    package_processor: PackageProcessor,
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
            package_processor: PackageProcessor::new(
                Rc::clone(&params.config),
                Rc::clone(&params.forge),
                Rc::clone(&params.package_configs),
            ),
        })
    }

    /// Re-render changelog notes from a saved release JSON file.
    ///
    /// Reads the file produced by `get next-release --out-file`,
    /// then re-applies the configured changelog template to each
    /// package's release data.
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

    /// Analyze commits and create or update release pull requests.
    ///
    /// If `target` is `Some`, only that package is processed.
    pub async fn create_release_prs(
        &self,
        target: Option<String>,
    ) -> Result<()> {
        if let Some(target_name) = target.as_ref()
            && !self.package_configs.hash().contains_key(target_name)
        {
            return Err(ReleasaurusError::InvalidArgs(format!(
                "unknown package: {target_name}"
            )));
        }

        let prepared = self
            .package_processor
            .prepare_packages(target.as_deref())
            .await?;

        let analyzed = self.package_processor.analyze_packages(prepared)?;

        let releasable =
            self.package_processor.releasable_packages(analyzed).await?;

        log::info!("releasable packages: {:#?}", releasable);

        let pr_packages = self
            .package_processor
            .release_pr_packages_by_branch(releasable)
            .await?;

        if pr_packages.is_empty() {
            return Ok(());
        }

        let results = self
            .package_processor
            .create_pr_branches(pr_packages)
            .await?;

        for PrBranchResult {
            request,
            existing_pr,
        } in results
        {
            let pr = if let Some(existing) = existing_pr {
                self.forge
                    .update_pr(UpdatePrRequest {
                        pr_number: existing.number,
                        title: request.title,
                        body: request.body,
                    })
                    .await?;
                existing
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

    /// Tag and publish releases for all packages with a merged
    /// release PR.
    ///
    /// If `target` is `Some`, only that package is processed. When
    /// `auto_start_next` is configured, a patch-bump commit is
    /// created after release.
    pub async fn create_releases(&self, target: Option<String>) -> Result<()> {
        let mut auto_start_packages: Vec<String> = vec![];
        let base_branch = self.config.base_branch.clone();

        if let Some(target_name) = target.as_ref()
            && !self.package_configs.hash().contains_key(target_name)
        {
            return Err(ReleasaurusError::InvalidArgs(format!(
                "unknown package: {target_name}"
            )));
        }

        for (name, package) in self.package_configs.hash().iter() {
            if let Some(target_name) = target.as_ref()
                && name != target_name
            {
                continue;
            }

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

    /// Bump manifest versions on the base branch without creating a
    /// PR.
    ///
    /// Used to advance patch versions after a release when
    /// `auto_start_next` is enabled.
    pub async fn start_next_release(
        &self,
        targets: Option<Vec<String>>,
    ) -> Result<()> {
        let prepared = self
            .package_processor
            .generate_prepared_with_dummy_commit(targets)
            .await?;

        let analyzed = self.package_processor.analyze_packages(prepared)?;

        let releasable =
            self.package_processor.releasable_packages(analyzed).await?;

        let pr_packages =
            self.package_processor.release_pr_packages(releasable)?;

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
                .get_latest_tag_for_prefix(
                    &package.tag_prefix,
                    &self.config.base_branch,
                )
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

    /// Analyze commits and return projected release data for each
    /// package without making any changes.
    pub async fn get_next_releases(
        &self,
        package: Option<&str>,
    ) -> Result<Vec<SerializableReleasablePackage>> {
        let prepared = self.package_processor.prepare_packages(package).await?;

        let analyzed = self.package_processor.analyze_packages(prepared)?;

        let mut releasable = self
            .package_processor
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

    /// Fetch release data for a specific tag from the forge.
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
        let (tag, notes) = if let Some((tag, notes)) = parse_legacy_pr_body(
            &package.name,
            merged_pr.number,
            &merged_pr.body,
        )? {
            (tag, notes)
        } else {
            parse_pr_body(&package.name, merged_pr.number, &merged_pr.body)?
        };

        log::info!("tagging commit: tag: {}, sha: {}", tag, merged_pr.sha);

        self.forge.tag_commit(&tag, &merged_pr.sha).await?;

        log::info!("creating release: tag: {}, sha: {}", tag, merged_pr.sha);

        self.forge
            .create_release(&tag, &merged_pr.sha, notes.trim())
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests;
