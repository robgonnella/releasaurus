use color_eyre::eyre::eyre;
use derive_builder::Builder;
use serde::Serialize;
use std::{
    collections::{HashMap, hash_map::Entry},
    path::Path,
    rc::Rc,
};
use tokio::fs;

use crate::{
    forge::{
        config::{PENDING_LABEL, TAGGED_LABEL},
        manager::ForgeManager,
        request::{
            CreateCommitRequest, CreateReleaseRequest, GetPrRequest,
            PrLabelsRequest, PullRequest, ReleaseByTagResponse,
            UpdatePrRequest,
        },
    },
    orchestrator::{
        package_processor::{PackageProcessor, PrBranchResult},
        pr_body::parse_pr_body,
    },
    packages::{
        releasable::ReleasablePackageGroups,
        release_pr::{PRBundle, ReleasePRPackage},
        resolved::ResolvedPackage,
    },
    resolver::ResolvedConfig,
    result::{ReleasaurusError, Result},
};

mod commit_fetcher;
mod package_processor;
mod pr_body;

pub use crate::{
    analyzer::{
        commit::{Commit, CommitPR},
        release::Release,
    },
    packages::releasable::{
        ReleasableSubPackage, SerializableReleasablePackage,
    },
    updater::manager::ManifestTarget,
};

/// Information about a package's current release
#[derive(Debug, Clone, Serialize)]
pub struct CurrentRelease {
    pub name: String,
    pub tag: String,
    pub sha: String,
    pub notes: String,
}

/// Builder parameters for constructing an [`Orchestrator`].
/// Use [`Orchestrator::builder`] to create one.
#[derive(Builder)]
#[builder(setter(into), build_fn(private, name = "_build"))]
pub struct OrchestratorParams {
    pub config: Rc<ResolvedConfig>,
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
            forge: Rc::clone(&params.forge),
            package_processor: PackageProcessor::new(
                params.config,
                Rc::clone(&params.forge),
            ),
        })
    }

    /// Analyzes and releases directly to the base branch, with no PR
    ///
    /// Analyzes commits and, if package is releasable, bumps version in
    /// appropriate manifest files, creates release commit, tags release
    /// commit, and creates release for target forge
    ///
    /// This is a deliberately separate, out-of-band flow: it neither
    /// creates nor consults release PRs, and is not meant to be combined
    /// with the `release-pr` / `release` workflow.
    pub async fn release_direct(&self, target: Option<String>) -> Result<()> {
        self.validate_target(target.as_deref())?;

        let prepared = self
            .package_processor
            .prepare_packages(target.as_deref())
            .await?;

        let analyzed = self.package_processor.analyze_packages(prepared)?;

        let releasable =
            self.package_processor.releasable_packages(analyzed).await?;

        log::debug!("releasable packages: {:#?}", releasable);

        if releasable.is_empty() {
            log::info!("no releasable packages: nothing to release");
            return Ok(());
        }

        let groups =
            self.package_processor.group_releasable_packages(releasable);

        self.reject_pending_release_pr(&groups).await?;

        for (release_branch, group) in groups {
            // Built here rather than up front for all groups: every one
            // commits to the same base branch, and a bundle read before
            // the previous commit landed would revert it.
            let bundle = self
                .package_processor
                .release_pr_bundle(release_branch, &group)
                .await?;

            self.direct_release_bundle(bundle).await?;
        }

        Ok(())
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

        // Bodies are per-package but usually identical across packages, so
        // compile once per distinct body rather than once per package.
        let mut compiled: HashMap<&str, tera::Tera> = HashMap::new();

        for package in packages.iter_mut() {
            let body = self
                .config
                .package_configs
                .get(&package.name)?
                .analyzer_config
                .body
                .as_str();

            let tera = match compiled.entry(body) {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => {
                    let mut tera = tera::Tera::default();
                    tera.add_raw_template("changelog", body)?;
                    entry.insert(tera)
                }
            };

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
        self.validate_target(target.as_deref())?;

        let prepared = self
            .package_processor
            .prepare_packages(target.as_deref())
            .await?;

        let analyzed = self.package_processor.analyze_packages(prepared)?;

        let releasable =
            self.package_processor.releasable_packages(analyzed).await?;

        log::debug!("releasable packages: {:#?}", releasable);

        if releasable.is_empty() {
            log::info!("no releasable packages: nothing to release");
            return Ok(());
        }

        let groups =
            self.package_processor.group_releasable_packages(releasable);

        self.reject_pending_release_pr(&groups).await?;

        // Safe to build every bundle up front here: each one lands on its
        // own release branch cut from the base branch.
        let pr_bundles =
            self.package_processor.release_pr_bundles(groups).await?;

        let results = self
            .package_processor
            .create_pr_branches(pr_bundles)
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

        self.validate_target(target.as_deref())?;

        for (name, package) in self.config.package_configs.hash().iter() {
            if let Some(target_name) = target.as_ref()
                && name != target_name
            {
                continue;
            }

            let req = GetPrRequest {
                base_branch: base_branch.clone(),
                head_branch: self.config.release_branch_for(&package.name),
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

                if package
                    .versioning_config
                    .auto_start_next
                    .unwrap_or_default()
                {
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

        let groups =
            self.package_processor.group_releasable_packages(releasable);

        for (release_branch, group) in groups {
            log::info!(
                "updating manifest package bundle on branch: {release_branch}"
            );

            // Built per group inside the loop: every group commits to the
            // same base branch, so a bundle read before the previous
            // commit landed would revert it.
            let bundle = self
                .package_processor
                .release_pr_bundle(release_branch.clone(), &group)
                .await?;

            // One commit per branch group now, so the message names every
            // package it bumps rather than a single one.
            let bumped = bundle
                .packages
                .iter()
                .map(|pkg| format!("{} {}", pkg.name, pkg.tag.semver))
                .collect::<Vec<_>>()
                .join(", ");

            let req = CreateCommitRequest {
                target_branch: self.config.base_branch.to_string(),
                file_changes: bundle.file_changes,
                message: format!(
                    "chore({}): bump patch version {bumped}",
                    self.config.base_branch
                ),
            };

            if let Some(commit) = self.forge.create_commit(req).await? {
                log::info!("created commit: {}", commit.sha);
            } else {
                log::warn!(
                    "manifest files already up to date for packages on branch: {release_branch}",
                );
            }
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

        for (name, package) in self.config.package_configs.hash().iter() {
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
    // private
    ////////////////////////////////////////////////////////////////////////////

    /// Rejects a `--package` value that does not name a configured
    /// package.
    fn validate_target(&self, target: Option<&str>) -> Result<()> {
        if let Some(target_name) = target
            && !self.config.package_configs.hash().contains_key(target_name)
        {
            return Err(ReleasaurusError::InvalidArgs(format!(
                "unknown package: {target_name}"
            )));
        }

        Ok(())
    }

    /// Wraps a failure that happens once the direct release commit is
    /// already on the base branch. Unlike the PR flow there is no label
    /// or PR body recording what landed, so the error has to carry it.
    fn partial_direct_release(
        &self,
        sha: &str,
        tagged: &[&str],
        failed_tag: &str,
        cause: ReleasaurusError,
    ) -> ReleasaurusError {
        ReleasaurusError::partial_direct_release(
            sha,
            self.config.base_branch.as_str(),
            tagged,
            failed_tag,
            &cause,
        )
    }

    /// Refuses to release a package that still has a merged release PR
    /// waiting to be tagged, which would release that version twice.
    async fn reject_pending_release_pr(
        &self,
        groups: &ReleasablePackageGroups,
    ) -> Result<()> {
        for release_branch in groups.keys() {
            if let Some(pending) = self
                .forge
                .get_merged_release_pr(GetPrRequest {
                    base_branch: self.config.base_branch.clone(),
                    head_branch: release_branch.clone(),
                })
                .await?
            {
                return Err(ReleasaurusError::pending_release(
                    release_branch,
                    pending.number,
                ));
            }
        }

        Ok(())
    }

    /// Creates release for a targeted package and merged PR
    async fn create_package_release(
        &self,
        package: &ResolvedPackage,
        merged_pr: &PullRequest,
    ) -> Result<()> {
        let (tag, notes) =
            parse_pr_body(&package.name, merged_pr.number, &merged_pr.body)
                .inspect_err(|_e| {
                    log::debug!(
                        "parse_pr_body failed for PR #{} ({} chars body): {:?}",
                        merged_pr.number,
                        merged_pr.body.len(),
                        merged_pr.body
                    );
                })?;

        if !self.forge.tag_exists_for_sha(&tag, &merged_pr.sha).await? {
            log::info!("tagging commit: tag: {}, sha: {}", tag, merged_pr.sha);
            self.forge.tag_commit(&tag, &merged_pr.sha).await?;
        }

        log::info!("creating release: tag: {}, sha: {}", tag, merged_pr.sha);

        // The tag is on the commit by now, so a publish failure leaves the
        // same split state `release-direct` reports — except here the PR
        // keeps its pending label and re-running picks it back up.

        // Only the tag name survives in the merged PR body, so the
        // prerelease flag is recovered from it. An unparseable tag falls
        // back to a normal release rather than guessing.
        let prerelease = tag
            .strip_prefix(&package.tag_prefix)
            .and_then(|v| semver::Version::parse(v).ok())
            .is_some_and(|v| !v.pre.is_empty());

        self.forge
            .create_release(CreateReleaseRequest {
                tag: tag.clone(),
                sha: merged_pr.sha.clone(),
                notes: notes.trim().to_string(),
                prerelease,
            })
            .await
            .map_err(|e| {
                ReleasaurusError::release_not_published(
                    &tag,
                    &merged_pr.sha,
                    &e,
                )
            })?;

        Ok(())
    }

    async fn direct_release_bundle(&self, bundle: PRBundle) -> Result<()> {
        let created = self
            .forge
            .create_commit(CreateCommitRequest {
                target_branch: self.config.base_branch.clone(),
                message: bundle.commit_message,
                file_changes: bundle.file_changes,
            })
            .await?;

        let Some(commit) = created else {
            log::warn!(
                "release commit produced no file changes: skipping tag \
                 and release for: {}: the version bump may already be \
                 on '{}'",
                bundle
                    .packages
                    .iter()
                    .map(|pkg| pkg.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                self.config.base_branch,
            );

            return Ok(());
        };

        self.tag_and_release_all_packages_in_bundle(
            &bundle.packages,
            &commit.sha,
        )
        .await?;

        Ok(())
    }

    async fn tag_and_release_all_packages_in_bundle(
        &self,
        packages: &[ReleasePRPackage],
        commit_sha: &str,
    ) -> Result<()> {
        // Tag the whole group before publishing any of it. A failure
        // part way through then leaves every package tagged rather
        // than a mix, and nothing published.
        let mut tagged: Vec<&str> = vec![];

        for pkg in packages.iter() {
            log::info!(
                "tagging commit: tag: {}, sha: {}",
                pkg.tag.name,
                commit_sha
            );

            self.forge
                .tag_commit(&pkg.tag.name, commit_sha)
                .await
                .map_err(|e| {
                    self.partial_direct_release(
                        commit_sha,
                        &tagged,
                        &pkg.tag.name,
                        e,
                    )
                })?;

            tagged.push(&pkg.tag.name);
        }

        for pkg in packages.iter() {
            log::info!(
                "creating release: tag: {}, sha: {}",
                pkg.tag.name,
                commit_sha
            );

            self.forge
                .create_release(CreateReleaseRequest {
                    tag: pkg.tag.name.clone(),
                    sha: commit_sha.to_string(),
                    notes: pkg.notes.clone(),
                    prerelease: !pkg.tag.semver.pre.is_empty(),
                })
                .await
                .map_err(|e| {
                    self.partial_direct_release(
                        commit_sha,
                        &tagged,
                        &pkg.tag.name,
                        e,
                    )
                })?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
