use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::{
    analyzer::Analyzer,
    config::resolved::ResolvedConfig,
    forge::{
        config::DEFAULT_PR_BRANCH_PREFIX,
        manager::ForgeManager,
        request::{
            CreatePrRequest, CreateReleaseBranchRequest, FileChange,
            FileUpdateType, ForgeCommit, GetPrRequest, PullRequest,
        },
    },
    orchestrator::{
        commit_fetcher::CommitFetcher,
        pr_body::{extract_preserved_header_footer, normalize_html_id},
    },
    packages::{
        analyzed::AnalyzedPackage,
        prepared::PreparedPackage,
        releasable::{
            ReleasablePackage, ReleasableSubPackage,
            SerializableReleasablePackage,
        },
        releasable_builder::ReleasablePackageBuilder,
        release_pr::{PRBundle, ReleasePRPackage},
        resolved::ResolvedPackage,
        resolved_hash::ResolvedPackageHash,
    },
    result::{ReleasaurusError, Result},
    updater::manager::UpdateManager,
};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PRMetadataFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_compare_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha_compare_link: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PRMetadata {
    pub metadata: PRMetadataFields,
}

/// Result of `create_pr_branches`: the forge request paired with the existing
/// open PR for that branch (if one was found during `release_pr_packages_by_branch`).
pub struct PrBranchResult {
    pub request: CreatePrRequest,
    pub existing_pr: Option<PullRequest>,
}

pub struct PackageProcessor {
    config: Rc<ResolvedConfig>,
    forge: Rc<ForgeManager>,
    package_configs: Rc<ResolvedPackageHash>,
    commit_fetcher: CommitFetcher,
}

impl PackageProcessor {
    pub fn new(
        config: Rc<ResolvedConfig>,
        forge: Rc<ForgeManager>,
        package_configs: Rc<ResolvedPackageHash>,
    ) -> Self {
        Self {
            config: Rc::clone(&config),
            commit_fetcher: CommitFetcher::new(
                config.base_branch.clone(),
                Rc::clone(&forge),
                Rc::clone(&package_configs),
            ),
            forge,
            package_configs,
        }
    }

    pub async fn generate_prepared_with_dummy_commit(
        &self,
        targets: Option<Vec<String>>,
    ) -> Result<Vec<PreparedPackage>> {
        let mut prepared = vec![];

        for (name, pkg) in self.package_configs.hash().iter() {
            // This is not added to changelog or tracked anywhere so we can just
            // use a fake dummy commit to trigger a patch version update
            let pkg_commit = ForgeCommit {
                id: "dummy".into(),
                short_id: "dummy".into(),
                message: "fix: dummy commit".into(),
                timestamp: Utc::now().timestamp(),
                files: vec![
                    pkg.normalized_full_path
                        .join("dummy.txt")
                        .to_string_lossy()
                        .to_string(),
                ],
                ..ForgeCommit::default()
            };

            let current_tag = self
                .forge
                .get_latest_tag_for_prefix(
                    &pkg.tag_prefix,
                    &self.config.base_branch,
                )
                .await?;

            if current_tag.is_none() {
                log::warn!(
                    "package {} has not been tagged yet: cannot start-next: skipping",
                    pkg.name
                );
                continue;
            }

            if let Some(list) = targets.as_ref()
                && list.contains(name)
            {
                prepared.push(PreparedPackage {
                    name: name.clone(),
                    current_tag,
                    commits: vec![pkg_commit],
                });
            } else if targets.is_none() {
                prepared.push(PreparedPackage {
                    name: name.clone(),
                    current_tag,
                    commits: vec![pkg_commit],
                });
            }
        }

        Ok(prepared)
    }

    pub async fn prepare_packages(
        &self,
        target: Option<&str>,
    ) -> Result<Vec<PreparedPackage>> {
        let mut prepared_packages = vec![];

        let (commits, tags) = self
            .commit_fetcher
            .get_commits_for_all_packages(target)
            .await?;

        let commit_hash_set: HashSet<_> = commits.iter().collect();

        for (name, package) in self.package_configs.hash().iter() {
            if let Some(target) = target
                && package.name != target
            {
                continue;
            }

            let tag_info = tags.get(name);
            let current_tag = tag_info.and_then(|i| i.tag.clone());
            let is_graduating_to_stable =
                tag_info.map(|i| i.graduating_to_stable).unwrap_or_default();

            let mut commits = self.commit_fetcher.filter_commits_for_package(
                package,
                current_tag.as_ref(),
                &commits,
            );

            if self.config.changelog.aggregate_prereleases
                && is_graduating_to_stable
            {
                let additional = self
                    .commit_fetcher
                    .fetch_additional_commits_for_prerelease_aggregation(
                        package,
                    )
                    .await?;
                commits.extend(
                    additional
                        .into_iter()
                        .filter(|c| !commit_hash_set.contains(c)),
                );
                commits.sort_by_key(|c| c.timestamp);
            }

            prepared_packages.push(PreparedPackage {
                name: name.clone(),
                current_tag,
                commits,
            })
        }

        Ok(prepared_packages)
    }

    pub fn analyze_packages(
        &self,
        packages: Vec<PreparedPackage>,
    ) -> Result<Vec<AnalyzedPackage>> {
        let mut analyzed_packages = vec![];

        for pkg in packages.into_iter() {
            let config = self.package_configs.get(&pkg.name)?;
            let analyzer = Analyzer::new(&config.analyzer_config)?;
            let release = analyzer.analyze(pkg.commits, pkg.current_tag)?;
            let analyzed = AnalyzedPackage {
                name: pkg.name.clone(),
                release,
            };
            analyzed_packages.push(analyzed);
        }

        Ok(analyzed_packages)
    }

    pub async fn releasable_packages(
        &self,
        packages: Vec<AnalyzedPackage>,
    ) -> Result<Vec<ReleasablePackage>> {
        self.build_releasable_packages(packages).await
    }

    pub async fn full_serializable_releasable_packages(
        &self,
        packages: Vec<AnalyzedPackage>,
    ) -> Result<Vec<SerializableReleasablePackage>> {
        self.build_releasable_packages(packages).await
    }

    pub fn release_pr_packages(
        &self,
        packages: Vec<ReleasablePackage>,
    ) -> Result<Vec<ReleasePRPackage>> {
        let mut finalized = vec![];
        for target in packages.iter() {
            let target_config = self.package_configs.get(&target.name)?;

            let mut release_branch = format!(
                "{}-{}",
                DEFAULT_PR_BRANCH_PREFIX, self.config.base_branch
            );

            if self.config.separate_pull_requests {
                release_branch = format!("{release_branch}-{}", target.name);
            }

            let releasable_refs: Vec<&ReleasablePackage> =
                packages.iter().collect();

            // gather other packages related to target package that may be in
            // same workspace
            let workspace_packages =
                self.related_packages(target, target_config, &releasable_refs)?;

            log::info!(
                "Package: {}: Found {} other packages for workspace root: {}",
                target.name,
                workspace_packages.len(),
                target_config.normalized_workspace_root.to_string_lossy()
            );

            let mut file_changes =
                UpdateManager::get_package_manifest_file_changes(
                    target,
                    &releasable_refs,
                )?;

            file_changes
                .push(self.changelog_file_change(target, target_config));

            finalized.push(ReleasePRPackage {
                name: target.name.clone(),
                tag: target.tag.clone(),
                notes: target.notes.clone(),
                tag_compare_link: target.tag_compare_link.clone(),
                sha_compare_link: target.sha_compare_link.clone(),
                file_changes,
                release_branch,
            });
        }

        Ok(finalized)
    }

    pub async fn release_pr_packages_by_branch(
        &self,
        packages: Vec<ReleasablePackage>,
    ) -> Result<HashMap<String, PRBundle>> {
        let release_prs = self.release_pr_packages(packages)?;

        let mut map: HashMap<String, Vec<ReleasePRPackage>> = HashMap::new();

        for pkg in release_prs {
            let list = map.get_mut(&pkg.release_branch);

            if let Some(list) = list {
                list.push(pkg)
            } else {
                map.insert(pkg.release_branch.clone(), vec![pkg]);
            };
        }

        let mut bundles: HashMap<String, PRBundle> = HashMap::new();

        for (branch, packages) in map {
            let existing_pr = self
                .forge
                .get_open_release_pr(GetPrRequest {
                    head_branch: branch.clone(),
                    base_branch: self.config.base_branch.clone(),
                })
                .await?;

            bundles.insert(
                branch,
                PRBundle {
                    existing_pr,
                    packages,
                },
            );
        }

        Ok(bundles)
    }

    pub async fn create_pr_branches(
        &self,
        bundles: HashMap<String, PRBundle>,
    ) -> Result<Vec<PrBranchResult>> {
        let mut pr_results = vec![];

        for (release_branch, bundle) in bundles.into_iter() {
            if let Some(pending_release) = self
                .forge
                .get_merged_release_pr(GetPrRequest {
                    base_branch: self.config.base_branch.clone(),
                    head_branch: release_branch.clone(),
                })
                .await?
            {
                return Err(ReleasaurusError::pending_release(
                    release_branch.clone(),
                    pending_release.number,
                ));
            }

            let file_changes: Vec<FileChange> = bundle
                .packages
                .iter()
                .flat_map(|p| p.file_changes.clone())
                .collect();

            let message =
                self.release_message_for_pr_package_list(&bundle.packages);

            self.forge
                .create_release_branch(CreateReleaseBranchRequest {
                    base_branch: self.config.base_branch.clone(),
                    release_branch: release_branch.clone(),
                    message: message.clone(),
                    file_changes,
                })
                .await?;

            let existing_body =
                bundle.existing_pr.as_ref().map(|pr| pr.body.as_str());

            let request = CreatePrRequest {
                base_branch: self.config.base_branch.clone(),
                head_branch: release_branch.clone(),
                title: message,
                body: self.release_pr_body_for_pr_package_list(
                    &bundle.packages,
                    existing_body,
                )?,
            };

            pr_results.push(PrBranchResult {
                request,
                existing_pr: bundle.existing_pr,
            });
        }

        Ok(pr_results)
    }

    ////////////////////////////////////////////////////////////////////////////
    //// Private
    ////////////////////////////////////////////////////////////////////////////
    fn release_message_for_pr_package_list(
        &self,
        pr_packages: &[ReleasePRPackage],
    ) -> String {
        let mut message =
            format!("chore({}): release", self.config.base_branch);

        if pr_packages.len() == 1 {
            message = format!(
                "{message} {} {}",
                pr_packages[0].name, pr_packages[0].tag.name
            );
        }

        message
    }

    fn release_pr_body_for_pr_package_list(
        &self,
        pr_packages: &[ReleasePRPackage],
        existing_body: Option<&str>,
    ) -> Result<String> {
        let mut body = String::new();

        for pkg in pr_packages.iter() {
            let start_details = if pr_packages.len() == 1 {
                // auto-open dropdown if there's only one package
                "<details open>"
            } else {
                "<details>"
            };

            let metadata = PRMetadata {
                metadata: PRMetadataFields {
                    sha_compare_link: Some(pkg.sha_compare_link.clone()),
                    tag_compare_link: Some(pkg.tag_compare_link.clone()),
                    ..Default::default()
                },
            };

            let json = serde_json::to_string(&metadata)?;
            let metadata_str = format!(r#"<!--{json}-->"#);

            // in the PR body link to the comparison with sha instead
            // of tag since the tag doesn't exist yet
            let notes = pkg
                .notes
                .replace(&pkg.tag_compare_link, &pkg.sha_compare_link);

            let html_id = normalize_html_id(&pkg.name);

            let (header, footer) = existing_body
                .map(|b| extract_preserved_header_footer(b, &html_id))
                .unwrap_or_default();

            // create the drop down
            let package_body = format!(
                r#"{start_details}
<summary>{}</summary>
<div id="{html_id}-header">{header}</div>
<div id="{html_id}" data-tag="{}">
{metadata_str}

{notes}
</div>
<div id="{html_id}-footer">{footer}</div>
</details>"#,
                pkg.tag.name, pkg.tag.name
            );

            if body.is_empty() {
                body = package_body;
            } else {
                body = format!("{body}\n{package_body}");
            }
        }

        Ok(body)
    }

    /// Generic method for building releasable packages with different output
    /// types. Uses the ReleasablePackageBuilder trait to construct the
    /// appropriate type.
    async fn build_releasable_packages<T: ReleasablePackageBuilder>(
        &self,
        packages: Vec<AnalyzedPackage>,
    ) -> Result<Vec<T>> {
        let mut releasable = vec![];

        for pkg in packages.into_iter() {
            if let Some(release) = pkg.release {
                let pkg_config = self.package_configs.get(&pkg.name)?;

                let manifest_files = UpdateManager::load_manifests_for_package(
                    pkg_config,
                    self.forge.as_ref(),
                    &self.config.base_branch,
                )
                .await?;

                let additional_manifest_files =
                    UpdateManager::load_additional_manifests_for_package(
                        pkg_config,
                        self.forge.as_ref(),
                        &self.config.base_branch,
                    )
                    .await?;

                let mut sub_packages = vec![];

                for sub in pkg_config.sub_packages.iter() {
                    let manifest_files =
                        UpdateManager::load_manifests_for_package(
                            sub,
                            self.forge.as_ref(),
                            &self.config.base_branch,
                        )
                        .await?;

                    sub_packages.push(ReleasableSubPackage {
                        name: sub.name.clone(),
                        release_type: sub.release_type,
                        manifest_files,
                    })
                }

                releasable.push(T::build(
                    pkg.name.clone(),
                    release,
                    pkg_config,
                    manifest_files,
                    additional_manifest_files,
                    sub_packages,
                ));
            }
        }

        Ok(releasable)
    }

    fn related_packages<'a>(
        &self,
        target: &ReleasablePackage,
        target_config: &ResolvedPackage,
        others: &'a [&'a ReleasablePackage],
    ) -> Result<Vec<&'a &'a ReleasablePackage>> {
        let mut workspace_packages = vec![];

        for p in others.iter() {
            let p_config = self.package_configs.get(&p.name)?;
            if p.name != target.name
                && p_config.normalized_workspace_root
                    == target_config.normalized_workspace_root
                && p_config.release_type == target_config.release_type
            {
                workspace_packages.push(p);
            }
        }

        Ok(workspace_packages)
    }

    fn changelog_file_change(
        &self,
        target: &ReleasablePackage,
        target_config: &ResolvedPackage,
    ) -> FileChange {
        FileChange {
            path: target_config
                .normalized_full_path
                .join("CHANGELOG.md")
                .to_string_lossy()
                .to_string(),
            content: format!("{}\n\n", target.notes),
            update_type: FileUpdateType::Prepend,
        }
    }
}

#[cfg(test)]
mod tests;
