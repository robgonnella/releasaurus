//! Commit analysis, version detection, and changelog generation.
//!
//! Parses conventional commits, determines semantic version bumps,
//! and generates formatted changelogs using Tera templates.

use log::*;
use next_version::VersionUpdater;
use semver::Version;

use crate::{
    Result,
    analyzer::release::{Release, Tag},
    config::prerelease::{PrereleaseConfig, PrereleaseStrategy},
    forge::request::ForgeCommit,
};

mod commit;
pub mod config;
mod group;
mod helpers;
pub mod release;

/// Analyzes commits using conventional commit patterns to determine version
/// bumps and generate changelogs.
pub struct Analyzer {
    config: config::AnalyzerConfig,
    group_parser: group::GroupParser,
}

impl Analyzer {
    /// Create analyzer with changelog template configuration and tag prefix
    /// settings.
    pub fn new(config: config::AnalyzerConfig) -> Result<Self> {
        Ok(Self {
            config,
            group_parser: group::GroupParser::new(),
        })
    }

    /// Analyze commits to calculate the next semantic version and generate
    /// formatted release notes using Tera templates.
    pub fn analyze(
        &self,
        commits: Vec<ForgeCommit>,
        current_tag: Option<Tag>,
    ) -> Result<Option<Release>> {
        let mut release = self.process_commits(commits)?;

        if release.commits.is_empty() {
            return Ok(None);
        }

        // calculate next release
        if let Some(current) = current_tag.clone() {
            self.process_release_from_current_tag(&mut release, &current)?;
        } else {
            self.process_first_release(&mut release)?;
        }

        Ok(Some(release))
    }

    fn process_first_release(&self, release: &mut Release) -> Result<()> {
        // this is the first release
        let mut semver = semver::Version::parse("0.1.0")?;

        // Handle prerelease for first release
        if let Some(prerelease) = &self.config.prerelease {
            semver = helpers::add_prerelease(
                semver,
                prerelease.suffix()?,
                prerelease.strategy,
            )?;
        }

        let mut tag_name = semver.to_string();
        if let Some(prefix) = self.config.tag_prefix.clone() {
            tag_name = format!("{prefix}{tag_name}");
        }
        let next_tag = release::Tag {
            sha: release.sha.clone(),
            name: tag_name,
            semver,
            timestamp: None,
        };

        release.link =
            format!("{}/{}", self.config.release_link_base_url, next_tag.name);

        release.tag = Some(next_tag.clone());

        let context = tera::Context::from_serialize(&release)?;

        let notes = tera::Tera::one_off(&self.config.body, &context, false)?;

        release.notes = helpers::strip_extra_lines(notes.trim());

        Ok(())
    }

    fn process_release_from_current_tag(
        &self,
        release: &mut Release,
        current: &Tag,
    ) -> Result<()> {
        let commits = release
            .commits
            .iter()
            .map(|c| c.raw_message.to_string())
            .collect::<Vec<String>>();

        let mut version_updater = VersionUpdater::new()
            .with_breaking_always_increment_major(
                self.config.breaking_always_increment_major,
            )
            .with_features_always_increment_minor(
                self.config.features_always_increment_minor,
            );

        if let Some(value) = self.config.custom_major_increment_regex.clone() {
            version_updater =
                version_updater.with_custom_major_increment_regex(&value)?;
        }

        if let Some(value) = self.config.custom_minor_increment_regex.clone() {
            version_updater =
                version_updater.with_custom_minor_increment_regex(&value)?;
        }

        // Handle prerelease transitions
        let next = if let Some(prerelease) = &self.config.prerelease {
            info!(
                "calculating next prerelease version: suffix: {:?}, strategy: {:?}",
                prerelease.suffix, prerelease.strategy
            );

            // User wants a prerelease
            self.calculate_next_prerelease_version(
                current,
                &commits,
                prerelease,
                version_updater,
            )?
        } else {
            // No prerelease requested
            if current.semver.pre.is_empty() {
                // Normal stable version bump
                debug!(
                    "no prerelease configured - preforming standard version update"
                );
                version_updater.increment(&current.semver, commits)
            } else {
                info!(
                    "graduating current prerelease, {}, to stable version",
                    current.semver
                );
                // Graduate from prerelease to stable
                // Just remove prerelease suffix without bumping version
                helpers::graduate_prerelease(&current.semver)
            }
        };

        let mut next_tag_name = next.to_string();

        if let Some(prefix) = self.config.tag_prefix.clone() {
            next_tag_name = format!("{prefix}{}", next);
        }

        let next_tag = release::Tag {
            name: next_tag_name.clone(),
            semver: next,
            // we won't know timestamp or sha until release-pr is created
            timestamp: None,
            sha: "".into(),
        };

        release.link =
            format!("{}/{}", self.config.release_link_base_url, next_tag.name);

        release.tag = Some(next_tag.clone());

        let context = tera::Context::from_serialize(&release)?;
        let notes = tera::Tera::one_off(&self.config.body, &context, false)?;
        release.notes = helpers::strip_extra_lines(notes.trim());
        Ok(())
    }

    fn calculate_next_prerelease_version(
        &self,
        current: &Tag,
        commits: &[String],
        prerelease: &PrereleaseConfig,
        version_updater: VersionUpdater,
    ) -> Result<Version> {
        // User wants a prerelease
        let prerelease_id = prerelease.suffix()?;
        if current.semver.pre.is_empty() {
            info!(
                "current version, {}, is not a prerelease: starting new prerelease",
                current.semver
            );

            // Currently stable, starting a prerelease
            let next_stable =
                version_updater.increment(&current.semver, commits);

            let version = helpers::add_prerelease(
                next_stable,
                prerelease_id,
                prerelease.strategy,
            )?;
            Ok(version)
        } else {
            // Currently in a prerelease
            let current_pre_id =
                current.semver.pre.as_str().split('.').next().unwrap_or("");
            if current_pre_id == prerelease_id {
                info!(
                    "current version, {}, is prerelease that matches config",
                    current.semver
                );
                match prerelease.strategy {
                    PrereleaseStrategy::Static => {
                        info!("preforming static prerelease increment");
                        // first graduate to remove suffix
                        let mut version =
                            helpers::graduate_prerelease(&current.semver);
                        // then increment version as normal
                        version = version_updater.increment(&version, commits);
                        // finally re-add the static prerelease suffix
                        Ok(helpers::add_prerelease(
                            version,
                            prerelease_id,
                            prerelease.strategy,
                        )?)
                    }
                    PrereleaseStrategy::Versioned => {
                        // Same prerelease identifier - increment it
                        info!("preforming versioned prerelease increment");
                        Ok(version_updater.increment(&current.semver, commits))
                    }
                }
            } else {
                info!(
                    "current tag has prerelease that does not match config - graduating prerelease and adding new prerelease suffix"
                );
                // Different prerelease identifier - switch to new one
                // Graduate to stable, calculate next version, then add new prerelease
                let stable_current =
                    helpers::graduate_prerelease(&current.semver);
                let stable_next =
                    version_updater.increment(&stable_current, commits);
                let version = helpers::add_prerelease(
                    stable_next,
                    prerelease_id,
                    prerelease.strategy,
                )?;
                Ok(version)
            }
        }
    }

    /// Parse commits into structured format with conventional commit
    /// categorization and grouping.
    fn process_commits(
        &self,
        commits: Vec<ForgeCommit>,
    ) -> Result<release::Release> {
        // fill out and append to list of releases as we process commits
        let mut release = release::Release::default();

        if self.config.include_author {
            release.include_author = true;
        }

        // loop commits in reverse oldest -> newest
        for forge_commit in commits.iter() {
            // add commit details to release
            helpers::update_release_with_commit(
                &self.group_parser,
                &mut release,
                forge_commit,
                &self.config,
            );
        }

        Ok(release)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analyzer::config::AnalyzerConfig,
        config::prerelease::{PrereleaseConfig, PrereleaseStrategy},
    };
    use semver::Version as SemVer;

    fn create_prerelease_config(
        name: &str,
        strategy: PrereleaseStrategy,
    ) -> PrereleaseConfig {
        PrereleaseConfig {
            suffix: Some(name.to_string()),
            strategy,
        }
    }

    #[test]
    fn test_analyzer_new() {
        let config = AnalyzerConfig::default();
        let analyzer = Analyzer::new(config.clone()).unwrap();
        assert_eq!(analyzer.config.tag_prefix, config.tag_prefix);
    }

    #[test]
    fn test_analyze_empty_commits() {
        let config = AnalyzerConfig::default();
        let analyzer = Analyzer::new(config).unwrap();
        let result = analyzer.analyze(vec![], None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_analyze_first_release_no_tag() {
        let config = AnalyzerConfig::default();
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![
            ForgeCommit {
                id: "abc123".to_string(),
                message: "feat: add new feature".to_string(),
                timestamp: 1000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "def456".to_string(),
                message: "fix: fix bug".to_string(),
                timestamp: 2000,
                ..ForgeCommit::default()
            },
        ];

        let result = analyzer.analyze(commits, None).unwrap();

        let release = result.unwrap();
        assert!(release.tag.is_some());
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("0.1.0").unwrap()
        );
        assert_eq!(release.commits.len(), 2);
    }

    #[test]
    fn test_analyze_with_current_tag_patch_bump() {
        let config = AnalyzerConfig::default();
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            ..Tag::default()
        };

        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "fix: fix critical bug".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        let release = result.unwrap();
        assert!(release.tag.is_some());
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("1.0.1").unwrap()
        );
    }

    #[test]
    fn test_analyze_with_current_tag_minor_bump() {
        let config = AnalyzerConfig::default();
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            ..Tag::default()
        };

        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "feat: add new feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        let release = result.unwrap();
        assert!(release.tag.is_some());
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("1.1.0").unwrap()
        );
    }

    #[test]
    fn test_analyze_with_current_tag_major_bump() {
        let config = AnalyzerConfig::default();
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            ..Tag::default()
        };

        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "feat!: breaking change".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        let release = result.unwrap();
        assert!(release.tag.is_some());
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("2.0.0").unwrap()
        );
    }

    #[test]
    fn test_analyze_with_tag_prefix() {
        let config = AnalyzerConfig {
            tag_prefix: Some("v".to_string()),
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "feat: new feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, None).unwrap();

        let release = result.unwrap();
        assert_eq!(release.tag.as_ref().unwrap().name, "v0.1.0");
    }

    #[test]
    fn test_analyze_generates_release_link() {
        let config = AnalyzerConfig::default();
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "feat!: breaking change".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, None).unwrap();

        let release = result.unwrap();
        assert_eq!(release.commits.len(), 1);
    }

    #[test]
    fn test_analyze_multiple_commits() {
        let config = AnalyzerConfig::default();
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            ..Tag::default()
        };

        let commits = vec![
            ForgeCommit {
                id: "abc123".to_string(),
                message: "feat: feature one".to_string(),
                timestamp: 1000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "def456".to_string(),
                message: "feat: feature two".to_string(),
                timestamp: 2000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "ghi789".to_string(),
                message: "fix: bug fix".to_string(),
                timestamp: 3000,
                ..ForgeCommit::default()
            },
        ];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        let release = result.unwrap();
        assert_eq!(release.commits.len(), 3);
        // Should bump minor due to features
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("1.1.0").unwrap()
        );
    }

    #[test]
    fn test_skip_ci_filters_ci_commits() {
        let config = AnalyzerConfig {
            skip_ci: true,
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            ..Tag::default()
        };

        let commits = vec![
            ForgeCommit {
                id: "abc123".to_string(),
                message: "feat: add new feature".to_string(),
                timestamp: 1000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "def456".to_string(),
                message: "ci: update workflow".to_string(),
                timestamp: 2000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "ghi789".to_string(),
                message: "ci: fix pipeline".to_string(),
                timestamp: 3000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "jkl012".to_string(),
                message: "fix: bug fix".to_string(),
                timestamp: 4000,
                ..ForgeCommit::default()
            },
        ];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        let release = result.unwrap();
        // Should only have 2 commits (feat and fix), ci commits filtered out
        assert_eq!(release.commits.len(), 2);
        assert!(release.commits.iter().all(|c| c.group != group::Group::Ci));
    }

    #[test]
    fn test_skip_ci_false_includes_ci_commits() {
        let config = AnalyzerConfig {
            skip_ci: false,
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![
            ForgeCommit {
                id: "abc123".to_string(),
                message: "feat: add feature".to_string(),
                timestamp: 1000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "def456".to_string(),
                message: "ci: update workflow".to_string(),
                timestamp: 2000,
                ..ForgeCommit::default()
            },
        ];

        let result = analyzer.analyze(commits, None).unwrap();

        let release = result.unwrap();
        // Should have both commits
        assert_eq!(release.commits.len(), 2);
    }

    #[test]
    fn test_skip_chore_filters_chore_commits() {
        let config = AnalyzerConfig {
            skip_chore: true,
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            ..Tag::default()
        };

        let commits = vec![
            ForgeCommit {
                id: "abc123".to_string(),
                message: "feat: new feature".to_string(),
                timestamp: 1000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "def456".to_string(),
                message: "chore: update dependencies".to_string(),
                timestamp: 2000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "ghi789".to_string(),
                message: "chore: cleanup code".to_string(),
                timestamp: 3000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "jkl012".to_string(),
                message: "fix: fix bug".to_string(),
                timestamp: 4000,
                ..ForgeCommit::default()
            },
        ];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        let release = result.unwrap();
        // Should only have 2 commits (feat and fix), chore commits filtered out
        assert_eq!(release.commits.len(), 2);
        assert!(
            release
                .commits
                .iter()
                .all(|c| c.group != group::Group::Chore)
        );
    }

    #[test]
    fn test_skip_chore_false_includes_chore_commits() {
        let config = AnalyzerConfig {
            skip_chore: false,
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![
            ForgeCommit {
                id: "abc123".to_string(),
                message: "feat: add feature".to_string(),
                timestamp: 1000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "def456".to_string(),
                message: "chore: update dependencies".to_string(),
                timestamp: 2000,
                ..ForgeCommit::default()
            },
        ];

        let result = analyzer.analyze(commits, None).unwrap();

        let release = result.unwrap();
        // Should have both commits
        assert_eq!(release.commits.len(), 2);
    }

    #[test]
    fn test_skip_miscellaneous_filters_non_conventional_commits() {
        let config = AnalyzerConfig {
            skip_miscellaneous: true,
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            ..Tag::default()
        };

        let commits = vec![
            ForgeCommit {
                id: "abc123".to_string(),
                message: "feat: new feature".to_string(),
                timestamp: 1000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "def456".to_string(),
                message: "random commit without type".to_string(),
                timestamp: 2000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "ghi789".to_string(),
                message: "another random commit".to_string(),
                timestamp: 3000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "jkl012".to_string(),
                message: "fix: fix bug".to_string(),
                timestamp: 4000,
                ..ForgeCommit::default()
            },
        ];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        let release = result.unwrap();
        // Should only have 2 commits (feat and fix), miscellaneous filtered out
        assert_eq!(release.commits.len(), 2);
        assert!(
            release
                .commits
                .iter()
                .all(|c| c.group != group::Group::Miscellaneous)
        );
    }

    #[test]
    fn test_skip_miscellaneous_false_includes_non_conventional_commits() {
        let config = AnalyzerConfig {
            skip_miscellaneous: false,
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![
            ForgeCommit {
                id: "abc123".to_string(),
                message: "feat: add feature".to_string(),
                timestamp: 1000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "def456".to_string(),
                message: "random commit message".to_string(),
                timestamp: 2000,
                ..ForgeCommit::default()
            },
        ];

        let result = analyzer.analyze(commits, None).unwrap();

        let release = result.unwrap();
        // Should have both commits
        assert_eq!(release.commits.len(), 2);
    }

    #[test]
    fn test_skip_multiple_types_combined() {
        let config = AnalyzerConfig {
            skip_ci: true,
            skip_miscellaneous: true,
            skip_chore: true,
            skip_merge_commits: true,
            skip_release_commits: true,
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            ..Tag::default()
        };

        let commits = vec![
            ForgeCommit {
                id: "abc123".to_string(),
                message: "feat: new feature".to_string(),
                timestamp: 1000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "def456".to_string(),
                message: "ci: update workflow".to_string(),
                timestamp: 2000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "ghi789".to_string(),
                message: "chore: cleanup".to_string(),
                timestamp: 3000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "jkl012".to_string(),
                message: "random commit".to_string(),
                timestamp: 4000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "mno345".to_string(),
                message: "fix: fix bug".to_string(),
                timestamp: 5000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "pqr678".to_string(),
                message: "docs: update readme".to_string(),
                timestamp: 6000,
                ..ForgeCommit::default()
            },
        ];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        let release = result.unwrap();
        // Should only have 3 commits (feat, fix, docs)
        assert_eq!(release.commits.len(), 3);
        assert!(release.commits.iter().all(|c| c.group != group::Group::Ci));
        assert!(
            release
                .commits
                .iter()
                .all(|c| c.group != group::Group::Chore)
        );
        assert!(
            release
                .commits
                .iter()
                .all(|c| c.group != group::Group::Miscellaneous)
        );
    }

    #[test]
    fn test_include_author_sets_release_flag() {
        let config = AnalyzerConfig {
            include_author: true,
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "feat: new feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, None).unwrap();

        let release = result.unwrap();
        // Should have include_author set to true
        assert!(release.include_author);
    }

    #[test]
    fn test_include_author_false_by_default() {
        let config = AnalyzerConfig::default();
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "feat: new feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, None).unwrap();

        let release = result.unwrap();
        // Should have include_author set to false by default
        assert!(!release.include_author);
    }

    #[test]
    fn test_skip_ci_with_no_ci_commits() {
        let config = AnalyzerConfig {
            skip_ci: true,
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![
            ForgeCommit {
                id: "abc123".to_string(),
                message: "feat: new feature".to_string(),
                timestamp: 1000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "def456".to_string(),
                message: "fix: fix bug".to_string(),
                timestamp: 2000,
                ..ForgeCommit::default()
            },
        ];

        let result = analyzer.analyze(commits, None).unwrap();

        let release = result.unwrap();
        // Should have all commits since none are ci
        assert_eq!(release.commits.len(), 2);
    }

    #[test]
    fn test_skip_all_types_results_in_no_release() {
        let config = AnalyzerConfig {
            skip_ci: true,
            skip_chore: true,
            skip_miscellaneous: true,
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        // Only commits that would be filtered out
        let commits = vec![
            ForgeCommit {
                id: "abc123".to_string(),
                message: "ci: update workflow".to_string(),
                timestamp: 1000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "def456".to_string(),
                message: "chore: cleanup".to_string(),
                timestamp: 2000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "ghi789".to_string(),
                message: "random commit".to_string(),
                timestamp: 3000,
                ..ForgeCommit::default()
            },
        ];

        let result = analyzer.analyze(commits, None).unwrap();

        // Should return None since all commits are filtered out
        assert!(result.is_none());
    }

    #[test]
    fn test_prerelease_start_from_stable() {
        let config = AnalyzerConfig {
            prerelease: Some(create_prerelease_config(
                "alpha",
                PrereleaseStrategy::Versioned,
            )),
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            ..Tag::default()
        };

        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "feat: new feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        let release = result.unwrap();
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("1.1.0-alpha.1").unwrap()
        );
    }

    #[test]
    fn test_prerelease_continue_same_identifier() {
        let config = AnalyzerConfig {
            prerelease: Some(create_prerelease_config(
                "alpha",
                PrereleaseStrategy::Versioned,
            )),
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.1.0-alpha.1".to_string(),
            semver: SemVer::parse("1.1.0-alpha.1").unwrap(),
            ..Tag::default()
        };

        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "fix: bug fix".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        let release = result.unwrap();
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("1.1.0-alpha.2").unwrap()
        );
    }

    #[test]
    fn test_prerelease_graduate_to_stable() {
        let config = AnalyzerConfig {
            prerelease: None,
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0-alpha.5".to_string(),
            semver: SemVer::parse("1.0.0-alpha.5").unwrap(),
            ..Tag::default()
        };

        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "fix: final fix".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        let release = result.unwrap();
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("1.0.0").unwrap()
        );
    }

    #[test]
    fn test_prerelease_switch_identifier() {
        let config = AnalyzerConfig {
            prerelease: Some(create_prerelease_config(
                "beta",
                PrereleaseStrategy::Versioned,
            )),
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0-alpha.3".to_string(),
            semver: SemVer::parse("1.0.0-alpha.3").unwrap(),
            ..Tag::default()
        };

        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "feat: beta ready".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        let release = result.unwrap();
        // Should switch to beta and calculate next version
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("1.1.0-beta.1").unwrap()
        );
    }

    #[test]
    fn test_prerelease_first_release() {
        let config = AnalyzerConfig {
            prerelease: Some(create_prerelease_config(
                "alpha",
                PrereleaseStrategy::Versioned,
            )),
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "feat: initial".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, None).unwrap();

        let release = result.unwrap();
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("0.1.0-alpha.1").unwrap()
        );
    }

    #[test]
    fn test_prerelease_breaking_change() {
        let config = AnalyzerConfig {
            prerelease: Some(create_prerelease_config(
                "alpha",
                PrereleaseStrategy::Versioned,
            )),
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            ..Tag::default()
        };

        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "feat!: breaking change".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        let release = result.unwrap();
        // Breaking change should bump major version
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("2.0.0-alpha.1").unwrap()
        );
    }

    #[test]
    fn test_new_prerelease_with_static_strategy() {
        let config = AnalyzerConfig {
            prerelease: Some(create_prerelease_config(
                "snapshot",
                PrereleaseStrategy::Static,
            )),
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "abc123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            ..Tag::default()
        };

        let commits = vec![ForgeCommit {
            id: "def456".to_string(),
            message: "feat: new feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        let release = result.unwrap();
        let tag = release.tag.unwrap();
        assert_eq!(tag.semver, SemVer::parse("1.1.0-snapshot").unwrap());
        assert_eq!(tag.name, "1.1.0-snapshot");
    }

    #[test]
    fn test_continuing_prerelease_with_static_strategy() {
        let config = AnalyzerConfig {
            prerelease: Some(create_prerelease_config(
                "SNAPSHOT",
                PrereleaseStrategy::Static,
            )),
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "abc123".to_string(),
            name: "1.0.0-SNAPSHOT".to_string(),
            semver: SemVer::parse("1.0.0-SNAPSHOT").unwrap(),
            ..Tag::default()
        };

        let commits = vec![ForgeCommit {
            id: "def456".to_string(),
            message: "feat: new feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        let release = result.unwrap();
        let tag = release.tag.unwrap();
        assert_eq!(tag.semver, SemVer::parse("1.1.0-SNAPSHOT").unwrap());
        assert_eq!(tag.name, "1.1.0-SNAPSHOT");
    }

    #[test]
    fn test_prerelease_with_tag_prefix() {
        let config = AnalyzerConfig {
            tag_prefix: Some("v".into()),
            prerelease: Some(create_prerelease_config(
                "rc",
                PrereleaseStrategy::Versioned,
            )),
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "v1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            ..Tag::default()
        };

        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "feat: new feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        let release = result.unwrap();
        let tag = release.tag.unwrap();
        assert_eq!(tag.semver, SemVer::parse("1.1.0-rc.1").unwrap());
        assert_eq!(tag.name, "v1.1.0-rc.1");
    }

    #[test]
    fn test_breaking_always_increment_major_disabled() {
        let config = AnalyzerConfig {
            breaking_always_increment_major: false,
            ..AnalyzerConfig::default()
        };

        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "0.1.0".to_string(),
            semver: SemVer::parse("0.1.0").unwrap(),
            ..Tag::default()
        };

        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "feat!: breaking change".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
        let release = result.unwrap();

        // In 0.x versions with breaking_always_increment_major=false,
        // breaking changes bump minor instead of major
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("0.2.0").unwrap()
        );
    }

    #[test]
    fn test_custom_major_regex_works_with_breaking_syntax() {
        let config = AnalyzerConfig {
            custom_major_increment_regex: Some("MAJOR".to_string()),
            ..AnalyzerConfig::default()
        };

        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "0.1.0".to_string(),
            semver: SemVer::parse("0.1.0").unwrap(),
            ..Tag::default()
        };

        // Conventional breaking syntax still works even with custom regex
        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "feat!: breaking change".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
        let release = result.unwrap();

        // Breaking syntax still triggers major bump (custom regex is additive)
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("1.0.0").unwrap()
        );
    }

    #[test]
    fn test_custom_major_increment_regex() {
        let config = AnalyzerConfig {
            custom_major_increment_regex: Some("doc".to_string()),
            ..AnalyzerConfig::default()
        };

        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "0.1.0".to_string(),
            semver: SemVer::parse("0.1.0").unwrap(),
            ..Tag::default()
        };

        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "doc: this should bump major".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
        let release = result.unwrap();

        // Custom regex matches "doc" in commit message, bumps major
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("1.0.0").unwrap()
        );
    }

    #[test]
    fn test_features_always_increment_minor_disabled() {
        let config = AnalyzerConfig {
            features_always_increment_minor: false,
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "0.1.0".to_string(),
            semver: SemVer::parse("0.1.0").unwrap(),
            ..Tag::default()
        };

        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "feat: new feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
        let release = result.unwrap();

        // In 0.x versions with features_always_increment_minor=false,
        // features bump patch instead of minor
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("0.1.1").unwrap()
        );
    }

    #[test]
    fn test_custom_minor_increment_regex() {
        let config = AnalyzerConfig {
            custom_minor_increment_regex: Some("ci".to_string()),
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "0.1.0".to_string(),
            semver: SemVer::parse("0.1.0").unwrap(),
            ..Tag::default()
        };

        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "ci: this should bump minor".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
        let release = result.unwrap();

        // Custom regex matches "ci" in commit message, bumps minor
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("0.2.0").unwrap()
        );
    }

    #[test]
    fn test_custom_minor_regex_works_with_feat_syntax() {
        let config = AnalyzerConfig {
            custom_minor_increment_regex: Some("ci".to_string()),
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "0.1.0".to_string(),
            semver: SemVer::parse("0.1.0").unwrap(),
            ..Tag::default()
        };

        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "feat: new feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
        let release = result.unwrap();

        // Feat syntax still triggers minor bump (custom regex is additive)
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("0.2.0").unwrap()
        );
    }

    #[test]
    fn test_both_boolean_flags_disabled_minor_bump() {
        let config = AnalyzerConfig {
            features_always_increment_minor: false,
            breaking_always_increment_major: false,
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "0.1.0".to_string(),
            semver: SemVer::parse("0.1.0").unwrap(),
            ..Tag::default()
        };

        // With both flags disabled, only minor bump should occur
        let commits = vec![
            ForgeCommit {
                id: "abc123".to_string(),
                message: "feat!: breaking feature".to_string(),
                timestamp: 1000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "def456".to_string(),
                message: "feat: regular feature".to_string(),
                timestamp: 2000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "ghi789".to_string(),
                message: "fix: bug fix".to_string(),
                timestamp: 3000,
                ..ForgeCommit::default()
            },
        ];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
        let release = result.unwrap();

        // With both flags disabled, only minor bump
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("0.2.0").unwrap()
        );
    }

    #[test]
    fn test_both_boolean_flags_disabled_path_bump() {
        let config = AnalyzerConfig {
            features_always_increment_minor: false,
            breaking_always_increment_major: false,
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "0.1.0".to_string(),
            semver: SemVer::parse("0.1.0").unwrap(),
            ..Tag::default()
        };

        // With both flags disabled, only patch bump should occur
        let commits = vec![
            ForgeCommit {
                id: "def456".to_string(),
                message: "feat: regular feature".to_string(),
                timestamp: 1000,
                ..ForgeCommit::default()
            },
            ForgeCommit {
                id: "ghi789".to_string(),
                message: "fix: bug fix".to_string(),
                timestamp: 2000,
                ..ForgeCommit::default()
            },
        ];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
        let release = result.unwrap();

        // With both flags disabled, only patch bump
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("0.1.1").unwrap()
        );
    }

    #[test]
    fn test_custom_regex_matches_non_conventional_commit() {
        let config = AnalyzerConfig {
            custom_major_increment_regex: Some("wow".to_string()),
            ..AnalyzerConfig::default()
        };
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "0.1.0".to_string(),
            semver: SemVer::parse("0.1.0").unwrap(),
            ..Tag::default()
        };

        // Non-conventional commit message that matches custom regex
        let commits = vec![ForgeCommit {
            id: "abc123".to_string(),
            message: "wow: complete rewrite of core functionality".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        }];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
        let release = result.unwrap();

        // Custom regex matches "wow" and triggers major bump
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("1.0.0").unwrap()
        );
    }
}
