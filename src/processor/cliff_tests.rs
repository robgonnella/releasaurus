use std::{env, fs, process::Command, thread, time::Duration};

use tempfile::TempDir;

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
}

impl TestContext {
    fn new(tmp_dir: TempDir) -> Self {
        Self { tmp_dir }
    }

    fn path(&self) -> &Path {
        self.tmp_dir.path()
    }

    fn add_commit(&self, args: CommitArgs) -> Result<()> {
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

        thread::sleep(Duration::from_millis(1000));

        Ok(())
    }

    fn setup_repo(&self) -> Result<()> {
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

        args.file_name = "break.txt";
        args.footer = Some("BREAKING CHANGE: It broke");
        args.message = "fix!: fixed it by breaking it";
        args.tag = None;

        let result = self.add_commit(args.clone());
        assert!(result.is_ok());

        let result = env::set_current_dir(self.tmp_dir.path());
        assert!(result.is_ok());

        Ok(())
    }
}

#[test]
fn process_git_repository() {
    let tmp_dir = TempDir::new().unwrap();
    let context = TestContext::new(tmp_dir);
    let result = context.setup_repo();
    assert!(result.is_ok(), "failed to setup test repo");

    let config = ChangelogConfig::default();
    let result = CliffProcessor::new(config);
    assert!(result.is_ok(), "failed to create changelog instance");

    let changelog = result.unwrap();
    let result = changelog.write_changelog();
    assert!(result.is_ok(), "failed to write to file");

    let output = result.unwrap();

    assert!(
        !output.changelog.is_empty(),
        "output.log should not be empty"
    );

    assert!(output.current_version.is_some());
    let current_version = output.current_version.unwrap();
    assert_eq!(current_version, "v0.2.1", "current version does not match");

    assert!(output.next_version.is_some());
    let next_version = output.next_version.unwrap();
    assert_eq!(next_version, "v1.0.0", "next version does not match");

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
