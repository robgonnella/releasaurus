//! Commit analysis, version detection, and changelog generation.
//!
//! Parses conventional commits, determines semantic version bumps,
//! and generates formatted changelogs using Tera templates.

use next_version::VersionUpdater;

use crate::{forge::request::ForgeCommit, result::Result};
mod commit;
pub mod config;
mod group;
mod helpers;
pub mod release;

/// Repository analyzer for commit analysis and changelog generation.
pub struct Analyzer {
    config: config::AnalyzerConfig,
    group_parser: group::GroupParser,
}

impl Analyzer {
    /// Create analyzer with configuration and repository.
    pub fn new(config: config::AnalyzerConfig) -> Result<Self> {
        Ok(Self {
            config,
            group_parser: group::GroupParser::new(),
        })
    }

    /// Analyze commits and generate release information.
    pub fn analyze(
        &self,
        commits: Vec<ForgeCommit>,
        current_tag: Option<release::Tag>,
    ) -> Result<Option<release::Release>> {
        let mut release = self.process_commits(commits)?;

        if release.commits.is_empty() {
            return Ok(None);
        }

        // calculate next release
        if let Some(current) = current_tag.clone() {
            let commits = release
                .commits
                .iter()
                .map(|c| c.raw_message.to_string())
                .collect::<Vec<String>>();

            let version_updater = VersionUpdater::new()
                .with_breaking_always_increment_major(true)
                .with_features_always_increment_minor(true);

            let next = version_updater.increment(&current.semver, commits);

            let mut next_tag_name = next.to_string();

            if let Some(prefix) = self.config.tag_prefix.clone() {
                next_tag_name = format!("{prefix}{}", next);
            }

            let next_tag = release::Tag {
                sha: release.sha.clone(),
                name: next_tag_name.clone(),
                semver: next,
            };

            release.link = format!(
                "{}/{}",
                self.config.release_link_base_url, next_tag.name
            );

            release.tag = Some(next_tag.clone());

            let context = tera::Context::from_serialize(&release)?;
            let notes =
                tera::Tera::one_off(&self.config.body, &context, false)?;
            release.notes = helpers::strip_extra_lines(notes.trim());
        } else {
            // this is the first release
            let mut tag_name = "0.1.0".to_string();
            let semver = semver::Version::parse(&tag_name).unwrap();
            if let Some(prefix) = self.config.tag_prefix.clone() {
                tag_name = format!("{prefix}{tag_name}");
            }
            let next_tag = release::Tag {
                sha: release.sha.clone(),
                name: tag_name,
                semver,
            };

            release.link = format!(
                "{}/{}",
                self.config.release_link_base_url, next_tag.name
            );

            release.tag = Some(next_tag.clone());

            let context = tera::Context::from_serialize(&release)?;

            let notes =
                tera::Tera::one_off(&self.config.body, &context, false)?;

            release.notes = helpers::strip_extra_lines(notes.trim());
        }

        Ok(Some(release))
    }

    /// Process commits and build release information.
    fn process_commits(
        &self,
        commits: Vec<ForgeCommit>,
    ) -> Result<release::Release> {
        // fill out and append to list of releases as we process commits
        let mut release = release::Release::default();

        // loop commits in reverse oldest -> newest
        for forge_commit in commits.iter() {
            // add commit details to release
            helpers::update_release_with_commit(
                &self.group_parser,
                &mut release,
                forge_commit,
            );
        }

        Ok(release)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::config::AnalyzerConfig;
    use semver::Version as SemVer;

    fn create_test_config() -> AnalyzerConfig {
        AnalyzerConfig {
            tag_prefix: None,
            body: "Release version {{ version }}".to_string(),
            release_link_base_url: "https://github.com/test/repo/releases/tag"
                .to_string(),
        }
    }

    fn create_forge_commit(
        id: &str,
        message: &str,
        timestamp: i64,
    ) -> ForgeCommit {
        ForgeCommit {
            id: id.to_string(),
            link: format!("https://github.com/test/repo/commit/{}", id),
            author_name: "Test Author".to_string(),
            author_email: "test@example.com".to_string(),
            merge_commit: false,
            message: message.to_string(),
            timestamp,
        }
    }

    #[test]
    fn test_analyzer_new() {
        let config = create_test_config();
        let analyzer = Analyzer::new(config.clone());

        assert!(analyzer.is_ok());
        let analyzer = analyzer.unwrap();
        assert_eq!(analyzer.config.tag_prefix, config.tag_prefix);
    }

    #[test]
    fn test_analyzer_new_with_tag_prefix() {
        let mut config = create_test_config();
        config.tag_prefix = Some("v".to_string());

        let analyzer = Analyzer::new(config);
        assert!(analyzer.is_ok());
    }

    #[test]
    fn test_analyze_empty_commits() {
        let config = create_test_config();
        let analyzer = Analyzer::new(config).unwrap();

        let result = analyzer.analyze(vec![], None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_analyze_first_release_no_tag() {
        let config = create_test_config();
        let analyzer = Analyzer::new(config).unwrap();

        let commits = vec![
            create_forge_commit("abc123", "feat: add new feature", 1000),
            create_forge_commit("def456", "fix: fix bug", 2000),
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
        let config = create_test_config();
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
        };

        let commits =
            vec![create_forge_commit("abc123", "fix: fix critical bug", 1000)];

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
        let config = create_test_config();
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
        };

        let commits =
            vec![create_forge_commit("abc123", "feat: add new feature", 1000)];

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
        let config = create_test_config();
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
        };

        let commits = vec![create_forge_commit(
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
        let mut config = create_test_config();
        config.tag_prefix = Some("v".to_string());
        let analyzer = Analyzer::new(config).unwrap();

        let commits =
            vec![create_forge_commit("abc123", "feat: add new feature", 1000)];

        let result = analyzer.analyze(commits, None).unwrap();

        assert!(result.is_some());
        let release = result.unwrap();
        assert!(release.tag.is_some());
        assert_eq!(release.tag.as_ref().unwrap().name, "v0.1.0");
    }

    #[test]
    fn test_analyze_generates_release_link() {
        let config = create_test_config();
        let analyzer = Analyzer::new(config).unwrap();

        let commits =
            vec![create_forge_commit("abc123", "feat: add feature", 1000)];

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
        let config = create_test_config();
        let analyzer = Analyzer::new(config).unwrap();

        let current_tag = release::Tag {
            sha: "old123".to_string(),
            name: "1.0.0".to_string(),
            semver: SemVer::parse("1.0.0").unwrap(),
        };

        let commits = vec![
            create_forge_commit("abc123", "feat: feature one", 1000),
            create_forge_commit("def456", "feat: feature two", 2000),
            create_forge_commit("ghi789", "fix: bug fix", 3000),
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
}
