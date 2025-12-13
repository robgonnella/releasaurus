//! Commit analysis, version detection, and changelog generation.
//!
//! Parses conventional commits, determines semantic version bumps,
//! and generates formatted changelogs using Tera templates.

use next_version::VersionUpdater;
use semver::Version;

use crate::{
    Result,
    analyzer::release::{Release, Tag},
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
        if let Some(ref prerelease_id) = self.config.prerelease {
            semver = helpers::add_prerelease(
                semver,
                prerelease_id,
                self.config.prerelease_version,
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
        let next = if let Some(ref prerelease_id) = self.config.prerelease {
            // User wants a prerelease
            self.calculate_next_prerelease_version(
                current,
                &commits,
                prerelease_id,
                version_updater,
            )?
        } else {
            // No prerelease requested
            if current.semver.pre.is_empty() {
                // Normal stable version bump
                version_updater.increment(&current.semver, commits)
            } else {
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
        prerelease_id: &str,
        version_updater: VersionUpdater,
    ) -> Result<Version> {
        // User wants a prerelease
        if current.semver.pre.is_empty() {
            // Currently stable, starting a prerelease
            let next_stable =
                version_updater.increment(&current.semver, commits);
            let version = helpers::add_prerelease(
                next_stable,
                prerelease_id,
                self.config.prerelease_version,
            )?;
            Ok(version)
        } else {
            // Currently in a prerelease
            let current_pre_id =
                current.semver.pre.as_str().split('.').next().unwrap_or("");
            if current_pre_id == prerelease_id {
                // Same prerelease identifier - increment it
                Ok(version_updater.increment(&current.semver, commits))
            } else {
                // Different prerelease identifier - switch to new one
                // Graduate to stable, calculate next version, then add new prerelease
                let stable_current =
                    helpers::graduate_prerelease(&current.semver);
                let stable_next =
                    version_updater.increment(&stable_current, commits);
                let version = helpers::add_prerelease(
                    stable_next,
                    prerelease_id,
                    self.config.prerelease_version,
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
    use crate::test_helpers::*;
    use semver::Version as SemVer;

    #[test]
    fn test_analyzer_new() {
        let config = create_test_analyzer_config(None);
        let analyzer = Analyzer::new(config.clone());

        assert!(analyzer.is_ok());
        let analyzer = analyzer.unwrap();
        assert_eq!(analyzer.config.tag_prefix, config.tag_prefix);
    }

    #[test]
    fn test_analyzer_new_with_tag_prefix() {
        let mut config = create_test_analyzer_config(None);
        config.tag_prefix = Some("v".to_string());

        let analyzer = Analyzer::new(config);
        assert!(analyzer.is_ok());
    }

    #[test]
    fn test_analyze_empty_commits() {
        let config = create_test_analyzer_config(None);
        let analyzer = Analyzer::new(config).unwrap();

        let result = analyzer.analyze(vec![], None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_analyze_first_release_no_tag() {
        let config = create_test_analyzer_config(None);
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![
            create_test_forge_commit("abc123", "feat: add new feature", 1000),
            create_test_forge_commit("def456", "fix: fix bug", 2000),
        ];

        let result = analyzer.analyze(commits, None).unwrap();

        assert!(result.is_some());
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
        let config = create_test_analyzer_config(None);
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            timestamp: None,
        };

        let commits = vec![create_test_forge_commit(
            "abc123",
            "fix: fix critical bug",
            1000,
        )];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        assert!(release.tag.is_some());
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("1.0.1").unwrap()
        );
    }

    #[test]
    fn test_analyze_with_current_tag_minor_bump() {
        let config = create_test_analyzer_config(None);
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            timestamp: None,
        };

        let commits = vec![create_test_forge_commit(
            "abc123",
            "feat: add new feature",
            1000,
        )];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        assert!(release.tag.is_some());
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("1.1.0").unwrap()
        );
    }

    #[test]
    fn test_analyze_with_current_tag_major_bump() {
        let config = create_test_analyzer_config(None);
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            timestamp: None,
        };

        let commits = vec![create_test_forge_commit(
            "abc123",
            "feat!: breaking change",
            1000,
        )];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        assert!(release.tag.is_some());
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("2.0.0").unwrap()
        );
    }

    #[test]
    fn test_analyze_with_tag_prefix() {
        let mut config = create_test_analyzer_config(None);
        config.tag_prefix = Some("v".to_string());
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![create_test_forge_commit(
            "abc123",
            "feat: new feature",
            1000,
        )];

        let result = analyzer.analyze(commits, None).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        assert!(release.tag.is_some());
        assert_eq!(release.tag.as_ref().unwrap().name, "v0.1.0");
    }

    #[test]
    fn test_analyze_generates_release_link() {
        let config = create_test_analyzer_config(None);
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![create_test_forge_commit(
            "abc123",
            "feat!: breaking change",
            1000,
        )];

        let result = analyzer.analyze(commits, None).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        assert!(
            release
                .link
                .starts_with("https://github.com/test/repo/releases/tag")
        );
    }

    #[test]
    fn test_analyze_multiple_commits() {
        let config = create_test_analyzer_config(None);
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            timestamp: None,
        };

        let commits = vec![
            create_test_forge_commit("abc123", "feat: feature one", 1000),
            create_test_forge_commit("def456", "feat: feature two", 2000),
            create_test_forge_commit("ghi789", "fix: bug fix", 3000),
        ];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        assert!(result.is_some());
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
        let mut config = create_test_analyzer_config(None);
        config.skip_ci = true;
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            timestamp: None,
        };

        let commits = vec![
            create_test_forge_commit("abc123", "feat: add new feature", 1000),
            create_test_forge_commit("def456", "ci: update workflow", 2000),
            create_test_forge_commit("ghi789", "ci: fix pipeline", 3000),
            create_test_forge_commit("jkl012", "fix: bug fix", 4000),
        ];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        // Should only have 2 commits (feat and fix), ci commits filtered out
        assert_eq!(release.commits.len(), 2);
        assert!(release.commits.iter().all(|c| c.group != group::Group::Ci));
    }

    #[test]
    fn test_skip_ci_false_includes_ci_commits() {
        let mut config = create_test_analyzer_config(None);
        config.skip_ci = false; // Explicitly set to false
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![
            create_test_forge_commit("abc123", "feat: add feature", 1000),
            create_test_forge_commit("def456", "ci: update workflow", 2000),
        ];

        let result = analyzer.analyze(commits, None).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        // Should have both commits
        assert_eq!(release.commits.len(), 2);
    }

    #[test]
    fn test_skip_chore_filters_chore_commits() {
        let mut config = create_test_analyzer_config(None);
        config.skip_chore = true;
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            timestamp: None,
        };

        let commits = vec![
            create_test_forge_commit("abc123", "feat: new feature", 1000),
            create_test_forge_commit(
                "def456",
                "chore: update dependencies",
                2000,
            ),
            create_test_forge_commit("ghi789", "chore: cleanup code", 3000),
            create_test_forge_commit("jkl012", "fix: fix bug", 4000),
        ];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        assert!(result.is_some());
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
        let mut config = create_test_analyzer_config(None);
        config.skip_chore = false; // Explicitly set to false
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![
            create_test_forge_commit("abc123", "feat: add feature", 1000),
            create_test_forge_commit(
                "def456",
                "chore: update dependencies",
                2000,
            ),
        ];

        let result = analyzer.analyze(commits, None).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        // Should have both commits
        assert_eq!(release.commits.len(), 2);
    }

    #[test]
    fn test_skip_miscellaneous_filters_non_conventional_commits() {
        let mut config = create_test_analyzer_config(None);
        config.skip_miscellaneous = true;
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            timestamp: None,
        };

        let commits = vec![
            create_test_forge_commit("abc123", "feat: new feature", 1000),
            create_test_forge_commit(
                "def456",
                "random commit without type",
                2000,
            ),
            create_test_forge_commit("ghi789", "another random commit", 3000),
            create_test_forge_commit("jkl012", "fix: fix bug", 4000),
        ];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        assert!(result.is_some());
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
        let mut config = create_test_analyzer_config(None);
        config.skip_miscellaneous = false; // Explicitly set to false
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![
            create_test_forge_commit("abc123", "feat: add feature", 1000),
            create_test_forge_commit("def456", "random commit message", 2000),
        ];

        let result = analyzer.analyze(commits, None).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        // Should have both commits
        assert_eq!(release.commits.len(), 2);
    }

    #[test]
    fn test_skip_multiple_types_combined() {
        let mut config = create_test_analyzer_config(None);
        config.skip_ci = true;
        config.skip_chore = true;
        config.skip_miscellaneous = true;
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            timestamp: None,
        };

        let commits = vec![
            create_test_forge_commit("abc123", "feat: new feature", 1000),
            create_test_forge_commit("def456", "ci: update workflow", 2000),
            create_test_forge_commit("ghi789", "chore: cleanup", 3000),
            create_test_forge_commit("jkl012", "random commit", 4000),
            create_test_forge_commit("mno345", "fix: fix bug", 5000),
            create_test_forge_commit("pqr678", "docs: update readme", 6000),
        ];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        assert!(result.is_some());
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
        let mut config = create_test_analyzer_config(None);
        config.include_author = true;
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![create_test_forge_commit(
            "abc123",
            "feat: new feature",
            1000,
        )];

        let result = analyzer.analyze(commits, None).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        // Should have include_author set to true
        assert!(release.include_author);
    }

    #[test]
    fn test_include_author_false_by_default() {
        let config = create_test_analyzer_config(None);
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![create_test_forge_commit(
            "abc123",
            "feat: new feature",
            1000,
        )];

        let result = analyzer.analyze(commits, None).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        // Should have include_author set to false by default
        assert!(!release.include_author);
    }

    #[test]
    fn test_skip_ci_with_no_ci_commits() {
        let mut config = create_test_analyzer_config(None);
        config.skip_ci = true;
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![
            create_test_forge_commit("abc123", "feat: new feature", 1000),
            create_test_forge_commit("def456", "fix: fix bug", 2000),
        ];

        let result = analyzer.analyze(commits, None).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        // Should have all commits since none are ci
        assert_eq!(release.commits.len(), 2);
    }

    #[test]
    fn test_skip_all_types_results_in_no_release() {
        let mut config = create_test_analyzer_config(None);
        config.skip_ci = true;
        config.skip_chore = true;
        config.skip_miscellaneous = true;
        let analyzer = Analyzer::new(config).unwrap();

        // Only commits that would be filtered out
        let commits = vec![
            create_test_forge_commit("abc123", "ci: update workflow", 1000),
            create_test_forge_commit("def456", "chore: cleanup", 2000),
            create_test_forge_commit("ghi789", "random commit", 3000),
        ];

        let result = analyzer.analyze(commits, None).unwrap();

        // Should return None since all commits are filtered out
        assert!(result.is_none());
    }

    #[test]
    fn test_include_author_with_skip_options() {
        let mut config = create_test_analyzer_config(None);
        config.skip_ci = true;
        config.include_author = true;
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![
            create_test_forge_commit("abc123", "feat: new feature", 1000),
            create_test_forge_commit("def456", "ci: update workflow", 2000),
        ];

        let result = analyzer.analyze(commits, None).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        // Should have only 1 commit (ci filtered out)
        assert_eq!(release.commits.len(), 1);
        // Should have include_author set to true
        assert!(release.include_author);
    }

    #[test]
    fn test_prerelease_start_from_stable() {
        let mut config = create_test_analyzer_config(None);
        config.prerelease = Some("alpha".to_string());
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            timestamp: None,
        };

        let commits = vec![create_test_forge_commit(
            "abc123",
            "feat: new feature",
            1000,
        )];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("1.1.0-alpha.1").unwrap()
        );
    }

    #[test]
    fn test_prerelease_continue_same_identifier() {
        let mut config = create_test_analyzer_config(None);
        config.prerelease = Some("alpha".to_string());
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.1.0-alpha.1".to_string(),
            semver: SemVer::parse("1.1.0-alpha.1").unwrap(),
            timestamp: None,
        };

        let commits =
            vec![create_test_forge_commit("abc123", "fix: bug fix", 1000)];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("1.1.0-alpha.2").unwrap()
        );
    }

    #[test]
    fn test_prerelease_graduate_to_stable() {
        let mut config = create_test_analyzer_config(None);
        config.prerelease = None; // No prerelease = graduate
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0-alpha.5".to_string(),
            semver: SemVer::parse("1.0.0-alpha.5").unwrap(),
            timestamp: None,
        };

        let commits =
            vec![create_test_forge_commit("abc123", "fix: final fix", 1000)];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("1.0.0").unwrap()
        );
    }

    #[test]
    fn test_prerelease_switch_identifier() {
        let mut config = create_test_analyzer_config(None);
        config.prerelease = Some("beta".to_string());
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0-alpha.3".to_string(),
            semver: SemVer::parse("1.0.0-alpha.3").unwrap(),
            timestamp: None,
        };

        let commits =
            vec![create_test_forge_commit("abc123", "feat: beta ready", 1000)];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        // Should switch to beta and calculate next version
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("1.1.0-beta.1").unwrap()
        );
    }

    #[test]
    fn test_prerelease_first_release() {
        let mut config = create_test_analyzer_config(None);
        config.prerelease = Some("alpha".to_string());
        let analyzer = Analyzer::new(config).unwrap();

        let commits =
            vec![create_test_forge_commit("abc123", "feat: initial", 1000)];

        let result = analyzer.analyze(commits, None).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("0.1.0-alpha.1").unwrap()
        );
    }

    #[test]
    fn test_prerelease_breaking_change() {
        let mut config = create_test_analyzer_config(None);
        config.prerelease = Some("alpha".to_string());
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            timestamp: None,
        };

        let commits = vec![create_test_forge_commit(
            "abc123",
            "feat!: breaking change",
            1000,
        )];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        // Breaking change should bump major version
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("2.0.0-alpha.1").unwrap()
        );
    }

    #[test]
    fn test_prerelease_with_tag_prefix() {
        let mut config = create_test_analyzer_config(None);
        config.prerelease = Some("rc".to_string());
        config.tag_prefix = Some("v".to_string());
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "v1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
            timestamp: None,
        };

        let commits = vec![create_test_forge_commit(
            "abc123",
            "feat: new feature",
            1000,
        )];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        let tag = release.tag.unwrap();
        assert_eq!(tag.semver, SemVer::parse("1.1.0-rc.1").unwrap());
        assert_eq!(tag.name, "v1.1.0-rc.1");
    }

    #[test]
    fn test_breaking_always_increment_major_disabled() {
        let mut config = create_test_analyzer_config(None);
        config.breaking_always_increment_major = false;

        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = create_test_tag("0.1.0", "0.1.0", "old123");

        let commits = vec![create_test_forge_commit(
            "abc123",
            "feat!: breaking change",
            1000,
        )];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
        assert!(result.is_some());

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
        let mut config = create_test_analyzer_config(None);
        config.custom_major_increment_regex = Some("MAJOR".to_string());

        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = create_test_tag("0.1.0", "0.1.0", "old123");

        // Conventional breaking syntax still works even with custom regex
        let commits = vec![create_test_forge_commit(
            "abc123",
            "feat!: breaking change",
            1000,
        )];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
        assert!(result.is_some());
        let release = result.unwrap();

        // Breaking syntax still triggers major bump (custom regex is additive)
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("1.0.0").unwrap()
        );
    }

    #[test]
    fn test_custom_major_increment_regex() {
        let mut config = create_test_analyzer_config(None);
        config.custom_major_increment_regex = Some("doc".to_string());

        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = create_test_tag("0.1.0", "0.1.0", "old123");

        let commits = vec![create_test_forge_commit(
            "abc123",
            "doc: this should bump major",
            1000,
        )];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
        assert!(result.is_some());

        let release = result.unwrap();

        // Custom regex matches "doc" in commit message, bumps major
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("1.0.0").unwrap()
        );
    }

    #[test]
    fn test_features_always_increment_minor_disabled() {
        let mut config = create_test_analyzer_config(None);
        config.features_always_increment_minor = false;

        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = create_test_tag("0.1.0", "0.1.0", "old123");

        let commits = vec![create_test_forge_commit(
            "abc123",
            "feat: new feature",
            1000,
        )];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
        assert!(result.is_some());

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
        let mut config = create_test_analyzer_config(None);
        config.custom_minor_increment_regex = Some("ci".to_string());

        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = create_test_tag("0.1.0", "0.1.0", "old123");

        let commits = vec![create_test_forge_commit(
            "abc123",
            "ci: this should bump minor",
            1000,
        )];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
        assert!(result.is_some());

        let release = result.unwrap();

        // Custom regex matches "ci" in commit message, bumps minor
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("0.2.0").unwrap()
        );
    }

    #[test]
    fn test_custom_minor_regex_works_with_feat_syntax() {
        let mut config = create_test_analyzer_config(None);
        config.custom_minor_increment_regex = Some("ci".to_string());

        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = create_test_tag("0.1.0", "0.1.0", "old123");

        // Conventional feat syntax still works even with custom regex
        let commits = vec![create_test_forge_commit(
            "abc123",
            "feat: new feature",
            1000,
        )];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
        assert!(result.is_some());

        let release = result.unwrap();

        // Feat syntax still triggers minor bump (custom regex is additive)
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("0.2.0").unwrap()
        );
    }

    #[test]
    fn test_both_boolean_flags_disabled_minor_bump() {
        let mut config = create_test_analyzer_config(None);
        config.breaking_always_increment_major = false;
        config.features_always_increment_minor = false;

        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = create_test_tag("0.1.0", "0.1.0", "old123");

        // With both flags disabled, only minor bump should occur
        let commits = vec![
            create_test_forge_commit("abc123", "feat!: breaking feature", 1000),
            create_test_forge_commit("def456", "feat: regular feature", 2000),
            create_test_forge_commit("ghi789", "fix: bug fix", 3000),
        ];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
        assert!(result.is_some());

        let release = result.unwrap();

        // With both flags disabled, only minor bump
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("0.2.0").unwrap()
        );
    }

    #[test]
    fn test_both_boolean_flags_disabled_path_bump() {
        let mut config = create_test_analyzer_config(None);
        config.breaking_always_increment_major = false;
        config.features_always_increment_minor = false;

        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = create_test_tag("0.1.0", "0.1.0", "old123");

        // With both flags disabled, only patch bump should occur
        let commits = vec![
            create_test_forge_commit("def456", "feat: regular feature", 1000),
            create_test_forge_commit("ghi789", "fix: bug fix", 2000),
        ];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
        assert!(result.is_some());

        let release = result.unwrap();

        // With both flags disabled, only patch bump
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("0.1.1").unwrap()
        );
    }

    #[test]
    fn test_custom_regex_matches_non_conventional_commit() {
        let mut config = create_test_analyzer_config(None);
        config.custom_major_increment_regex = Some("wow".to_string());

        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = create_test_tag("0.1.0", "0.1.0", "old123");

        // Non-conventional commit message that matches custom regex
        let commits = vec![create_test_forge_commit(
            "abc123",
            "wow: complete rewrite of core functionality",
            1000,
        )];

        let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
        assert!(result.is_some());

        let release = result.unwrap();

        // Custom regex matches "wow" and triggers major bump
        assert_eq!(
            release.tag.unwrap().semver,
            SemVer::parse("1.0.0").unwrap()
        );
    }
}
