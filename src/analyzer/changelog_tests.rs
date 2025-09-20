use color_eyre::eyre::eyre;
use std::{fs, process::Command, thread, time::Duration};
use tempfile::TempDir;

use crate::forge::config::RemoteConfig;

use super::*;

#[derive(Clone)]
struct CommitArgs {
    file_name: &'static str,
    message: &'static str,
    footer: Option<&'static str>,
    tag: Option<&'static str>,
}

struct TestContext {
    tmp_dir: TempDir,
    starting_tag: Option<Tag>,
}

impl TestContext {
    fn new(tmp_dir: TempDir) -> Self {
        Self {
            tmp_dir,
            starting_tag: None,
        }
    }

    fn path(&self) -> &Path {
        self.tmp_dir.path()
    }

    fn starting_tag(&self) -> Option<Tag> {
        self.starting_tag.clone()
    }

    fn add_commit(&self, args: CommitArgs) -> Result<String> {
        // Create a file and commit it
        fs::write(self.tmp_dir.path().join(args.file_name), args.message)?;

        Command::new("git")
            .arg("add")
            .arg(args.file_name)
            .current_dir(self.tmp_dir.path())
            .status()?;

        let commit = |msg: &str| -> Result<()> {
            Command::new("git")
                .arg("commit")
                .arg("-m")
                .arg(msg)
                .current_dir(self.tmp_dir.path())
                .status()?;
            Ok(())
        };

        if let Some(footer) = args.footer {
            commit(format!("{}\n\n{footer}", args.message).as_str())?;
        } else {
            commit(args.message)?;
        }

        if let Some(tag) = args.tag {
            Command::new("git")
                .arg("tag")
                .arg("-m")
                .arg(tag)
                .arg(tag)
                .current_dir(self.tmp_dir.path())
                .status()?;
        }

        thread::sleep(Duration::from_millis(500));

        let output = Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(self.tmp_dir.path())
            .output()?;

        if !output.status.success() {
            return Err(eyre!("failed to get commit sha"));
        }

        let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

        Ok(sha)
    }

    fn setup_repo(&mut self) -> Result<()> {
        // Initialize Git repository
        Command::new("git")
            .arg("init")
            .current_dir(self.tmp_dir.path())
            .status()?;

        let mut args = CommitArgs {
            file_name: "init.txt",
            footer: None,
            message: "chore: init",
            tag: Some("v0.1.0"),
        };

        let result = self.add_commit(args.clone());
        assert!(result.is_ok());

        args.file_name = "feat.txt";
        args.footer = Some("I added a feature");
        args.message = "feat: new thing";
        args.tag = Some("v0.2.0");

        let result = self.add_commit(args.clone());
        assert!(result.is_ok());

        args.file_name = "fix.txt";
        args.footer = None;
        args.message = "fix: fixed a thing";
        args.tag = Some("v0.2.1");

        let result = self.add_commit(args.clone());
        assert!(result.is_ok());

        self.starting_tag = Some(Tag {
            name: "v0.2.1".into(),
            semver: semver::Version::parse("0.2.1").unwrap(),
            sha: result.unwrap(),
        });

        args.file_name = "break.txt";
        args.footer = Some("BREAKING CHANGE: It broke");
        args.message = "fix: fixed it by breaking it";
        args.tag = None;

        let result = self.add_commit(args.clone());
        assert!(result.is_ok());

        Ok(())
    }
}

#[test]
fn process_git_repository() {
    let tmp_dir = TempDir::new().unwrap();
    let tmp_dir_path_str = tmp_dir.path().display().to_string();
    let mut context = TestContext::new(tmp_dir);
    let result = context.setup_repo();
    let remote_config = RemoteConfig {
        host: "github.com".to_string(),
        scheme: "https".to_string(),
        owner: "test-owner".to_string(),
        repo: "test-repo".to_string(),
        path: "test-owner/test-repo".to_string(),
        commit_link_base_url: "https://github.com/test-owner/test-repo/commit"
            .to_string(),
        release_link_base_url:
            "https://github.com/test-owner/test-repo/releases/tag".to_string(),
        ..RemoteConfig::default()
    };

    assert!(result.is_ok(), "failed to setup test repo");

    let repo = Repository::from_local(
        context.path(),
        remote_config,
        "main".to_string(),
    )
    .unwrap();

    let config = AnalyzerConfig {
        repo_path: tmp_dir_path_str,
        tag_prefix: Some("v".to_string()),
        starting_tag: context.starting_tag(),
        ..AnalyzerConfig::default()
    };
    let result = Analyzer::new(config, &repo);
    assert!(result.is_ok(), "failed to create changelog instance");

    let analyzer = result.unwrap();

    let result = analyzer.write_changelog();
    assert!(result.is_ok(), "failed to write to file");

    let release = result.unwrap();

    assert!(release.is_some(), "there should be a release");

    let release = release.unwrap();

    assert!(!release.notes.is_empty(), "release should have notes");

    assert!(release.tag.is_some(), "release should have projected tag");
    let tag = release.tag.unwrap();

    assert_eq!(tag.name, "v1.0.0", "tag does not match");

    let file_path = format!("{}/CHANGELOG.md", context.path().display());

    // Assert that the file exists
    assert!(
        Path::new(&file_path).exists(),
        "File does not exist: {}",
        file_path,
    );

    // Assert that the file is not empty
    let result = fs::metadata(file_path);
    assert!(result.is_ok(), "failed to get file metadata");

    let metadata = result.unwrap();
    assert!(metadata.len() > 0, "file should not be empty");
}
