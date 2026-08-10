use chrono::Utc;
use color_eyre::eyre::eyre;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, rc::Rc};

use crate::{
    analyzer::Analyzer,
    forge::{
        manager::ForgeManager,
        request::{
            CreatePrRequest, CreateReleaseBranchRequest, FileChange,
            FileUpdateType, ForgeCommit, GetPrRequest, PullRequest, Tag,
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
            BranchName, ReleasablePackage, ReleasablePackageGroups,
            ReleasableSubPackage, SerializableReleasablePackage,
        },
        releasable_builder::ReleasablePackageBuilder,
        release_pr::{PRBundle, ReleasePRPackage},
        resolved::ResolvedPackage,
    },
    resolver::ResolvedConfig,
    result::{ReleasaurusError, Result},
    updater::{
        dispatch::Updater,
        manager::{ManifestTarget, UpdateManager},
    },
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
}

impl PackageProcessor {
    pub fn new(config: Rc<ResolvedConfig>, forge: Rc<ForgeManager>) -> Self {
        Self {
            config: Rc::clone(&config),
            forge,
        }
    }

    pub async fn generate_prepared_with_dummy_commit(
        &self,
        targets: Option<Vec<String>>,
    ) -> Result<Vec<PreparedPackage>> {
        let mut prepared = vec![];

        for (name, pkg) in self.config.package_configs.hash().iter() {
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

        prepared.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(prepared)
    }

    pub async fn prepare_packages(
        &self,
        target: Option<&str>,
    ) -> Result<Vec<PreparedPackage>> {
        let mut prepared_packages = vec![];

        let commit_fetcher =
            CommitFetcher::new(Rc::clone(&self.config), Rc::clone(&self.forge));

        let (commits, tags) =
            commit_fetcher.get_commits_for_all_packages(target).await?;

        let commit_hash_set: HashSet<_> = commits.iter().collect();

        for (name, package) in self.config.package_configs.hash().iter() {
            if let Some(target) = target
                && package.name != target
            {
                continue;
            }

            let tag_info = tags.get(name);
            let current_tag = tag_info.and_then(|i| i.tag.clone());
            let is_graduating_to_stable =
                tag_info.map(|i| i.graduating_to_stable).unwrap_or_default();

            let mut commits = commit_fetcher.filter_commits_for_package(
                package,
                current_tag.as_ref(),
                &commits,
            );

            if package.aggregate_prereleases && is_graduating_to_stable {
                let additional = commit_fetcher
                    .fetch_additional_commits_for_prerelease_aggregation(
                        package,
                    )
                    .await?;
                commits.extend(
                    additional
                        .into_iter()
                        .filter(|c| !commit_hash_set.contains(c)),
                );
                // restore the newest-first order guaranteed by
                // Forge::get_commits
                commits.sort_by_key(|c| std::cmp::Reverse(c.timestamp));
            }

            commit_fetcher
                .fetch_merged_commit_prs(package, &mut commits)
                .await;

            prepared_packages.push(PreparedPackage {
                name: name.clone(),
                current_tag,
                commits,
            })
        }

        // `package_configs` is a `HashMap`, so this order is otherwise
        // whatever the hasher produced this run. It reaches the emitted
        // file changes and the PR body sections, where a reshuffle looks
        // like a real diff.
        prepared_packages.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(prepared_packages)
    }

    pub fn analyze_packages(
        &self,
        packages: Vec<PreparedPackage>,
    ) -> Result<Vec<AnalyzedPackage>> {
        let mut analyzed_packages = vec![];

        for pkg in packages.into_iter() {
            let config = self.config.package_configs.get(&pkg.name)?;
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
        packages: &[ReleasablePackage],
    ) -> Result<Vec<ReleasePRPackage>> {
        let mut finalized = vec![];

        for target in packages.iter() {
            finalized.push(ReleasePRPackage {
                name: target.name.clone(),
                tag: target.tag.clone(),
                notes: target.notes.clone(),
                tag_compare_link: target.tag_compare_link.clone(),
                sha_compare_link: target.sha_compare_link.clone(),
            });
        }

        Ok(finalized)
    }

    /// Everything one release branch needs, with its file changes read
    /// off `base_branch` as it stands right now.
    ///
    /// Callers that commit several bundles to the *same* branch have to
    /// build each one immediately before its commit: the changes carry
    /// whole-file content, so a bundle built against a stale snapshot
    /// reverts whatever landed after it was read.
    pub async fn release_pr_bundle(
        &self,
        release_branch: BranchName,
        group: &[ReleasablePackage],
    ) -> Result<PRBundle> {
        let package_refs = group.iter().collect::<Vec<&ReleasablePackage>>();

        let mut file_changes = UpdateManager::get_file_changes_for_packages(
            &package_refs,
            self.forge.as_ref(),
            &self.config.base_branch,
        )
        .await?;

        for pkg in group.iter() {
            let conf = self.config.package_configs.get(&pkg.name)?;
            file_changes.push(self.changelog_file_change(pkg, conf))
        }

        Ok(PRBundle {
            release_branch,
            commit_message: self
                .release_commit_message_for_pr_package_list(group)?,
            pr_title: self.release_pr_title_for_pr_package_list(group)?,
            packages: self.release_pr_packages(group)?,
            file_changes,
        })
    }

    /// Every group's bundle, in release-branch order.
    ///
    /// Only safe for flows that commit each bundle to its own branch —
    /// see [`PackageProcessor::release_pr_bundle`].
    pub async fn release_pr_bundles(
        &self,
        groups: ReleasablePackageGroups,
    ) -> Result<Vec<PRBundle>> {
        let mut bundles = vec![];

        for (release_branch, group) in groups {
            bundles.push(self.release_pr_bundle(release_branch, &group).await?);
        }

        Ok(bundles)
    }

    pub async fn create_pr_branches(
        &self,
        bundles: Vec<PRBundle>,
    ) -> Result<Vec<PrBranchResult>> {
        let mut pr_results = vec![];

        for bundle in bundles.into_iter() {
            let release_branch = bundle.release_branch;

            let created = self
                .forge
                .create_release_branch(CreateReleaseBranchRequest {
                    base_branch: self.config.base_branch.clone(),
                    release_branch: release_branch.clone(),
                    message: bundle.commit_message,
                    file_changes: bundle.file_changes,
                })
                .await?;

            // No commit means no release branch to open a PR from.
            if created.is_none() {
                log::warn!(
                    "no file changes to commit: skipping release PR for branch: {release_branch}"
                );
                continue;
            }

            // Looked up here rather than while building the bundle: only
            // this flow reads it, and only once there is a branch to open
            // a PR from.
            let existing_pr = self
                .forge
                .get_open_release_pr(GetPrRequest {
                    head_branch: release_branch.clone(),
                    base_branch: self.config.base_branch.clone(),
                })
                .await?;

            let existing_body = existing_pr.as_ref().map(|pr| pr.body.as_str());

            let request = CreatePrRequest {
                base_branch: self.config.base_branch.clone(),
                head_branch: release_branch.clone(),
                title: bundle.pr_title,
                body: self.release_pr_body_for_pr_package_list(
                    &bundle.packages,
                    existing_body,
                )?,
            };

            pr_results.push(PrBranchResult {
                request,
                existing_pr,
            });
        }

        Ok(pr_results)
    }

    /// Gathers related packages together base on configuration and determined
    /// release branch
    pub fn group_releasable_packages(
        &self,
        releasable: Vec<ReleasablePackage>,
    ) -> ReleasablePackageGroups {
        let mut groups = ReleasablePackageGroups::new();

        for p in releasable {
            groups
                .entry(self.config.release_branch_for(&p.name))
                .or_default()
                .push(p);
        }

        groups
    }

    ////////////////////////////////////////////////////////////////////////////
    // Private
    ////////////////////////////////////////////////////////////////////////////

    fn release_commit_message_for_pr_package_list(
        &self,
        packages: &[ReleasablePackage],
    ) -> Result<String> {
        self.render_template_for_package_list(
            packages,
            "commit message",
            |pkg| &pkg.commit_message_template,
            &self.config.monorepo_commit_message_template,
        )
    }

    fn release_pr_title_for_pr_package_list(
        &self,
        packages: &[ReleasablePackage],
    ) -> Result<String> {
        self.render_template_for_package_list(
            packages,
            "PR title",
            |pkg| &pkg.pr_title_template,
            &self.config.monorepo_pr_title_template,
        )
    }

    /// Picks the package whose template applies to a bundle and renders
    /// it via [`PackageProcessor::render_release_template`].
    fn render_template_for_package_list(
        &self,
        packages: &[ReleasablePackage],
        what: &str,
        package_template: impl Fn(&ResolvedPackage) -> &str,
        monorepo_template: &str,
    ) -> Result<String> {
        let Some(package) = packages.first() else {
            return Err(ReleasaurusError::Other(eyre!(
                "Cannot generate {what} for empty package list"
            )));
        };

        let resolved_pkg = self.config.package_configs.get(&package.name)?;

        self.render_release_template(
            &package.name,
            &package.tag,
            package_template(resolved_pkg),
            monorepo_template,
        )
    }

    /// Renders one of the release templates.
    ///
    /// Which template applies is decided by config alone, not by what is
    /// being released: a repo with one configured package, or with
    /// `separate_pull_requests` on, can only ever release one package at
    /// a time, so those use the package's own template. Everything else
    /// is a combined release that may span several packages and uses the
    /// monorepo template — including on runs where only one package
    /// happens to have changes. Deciding this from config keeps the
    /// format stable from one release to the next.
    ///
    /// Both templates were validated during config resolution, so a
    /// failure here means the real values tripped something the probe
    /// values did not.
    fn render_release_template(
        &self,
        package_name: &str,
        tag: &Tag,
        package_template: &str,
        monorepo_template: &str,
    ) -> Result<String> {
        let mut context = tera::Context::new();
        context.insert("repo_name", &self.config.repo_name);
        context.insert("branch", &self.config.base_branch);

        let template = if self.config.separate_pull_requests
            || self.config.package_configs.hash().len() == 1
        {
            context.insert("package_name", package_name);
            context.insert("tag", &tag.name);
            context.insert("semver", &tag.semver.to_string());
            package_template
        } else {
            monorepo_template
        };

        Ok(tera::Tera::one_off(template, &context, false)?)
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
            let block = self.forge.encode_pr_metadata(&json);

            let div_attr = if block.div_attribute.is_empty() {
                String::new()
            } else {
                format!(" {}", block.div_attribute)
            };
            let inline_section = if block.inline_content.is_empty() {
                "\n".to_string()
            } else {
                format!("{}\n\n", block.inline_content)
            };

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
<div id="{html_id}" data-tag="{}"{div_attr}>
{inline_section}{notes}
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
                let pkg_config = self.config.package_configs.get(&pkg.name)?;

                let updater = Updater::new(pkg_config.release_type);

                let manifest_targets = updater.manifest_targets(
                    &pkg_config.name,
                    &pkg_config.normalized_workspace_root,
                    &pkg_config.normalized_full_path,
                );

                let mut additional_manifest_targets = vec![];

                for additional in pkg_config.additional_manifests.iter() {
                    let target = ManifestTarget {
                        basename: additional.basename.clone(),
                        path: additional.full_path.clone(),
                    };
                    additional_manifest_targets
                        .push((target, additional.version_regex.clone()))
                }

                let mut sub_packages = vec![];

                for sub in pkg_config.sub_packages.iter() {
                    let updater = Updater::new(sub.release_type);

                    let sub_targets = updater.manifest_targets(
                        &sub.name,
                        &sub.normalized_workspace_root,
                        &sub.normalized_full_path,
                    );

                    sub_packages.push(ReleasableSubPackage {
                        name: sub.name.clone(),
                        path: sub.normalized_full_path.clone(),
                        release_type: sub.release_type,
                        manifest_targets: sub_targets,
                    })
                }

                releasable.push(T::build(
                    pkg.name.clone(),
                    release,
                    pkg_config,
                    manifest_targets,
                    additional_manifest_targets,
                    sub_packages,
                ));
            }
        }

        Ok(releasable)
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
            content: format!("{}\n", target.notes),
            update_type: FileUpdateType::Prepend,
        }
    }
}

#[cfg(test)]
mod tests;
