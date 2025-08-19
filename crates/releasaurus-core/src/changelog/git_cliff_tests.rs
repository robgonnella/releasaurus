use std::{env, fs, process::Command, thread, time::Duration};

use tempfile::TempDir;

use crate::config::Config;

use super::*;

#[derive(Clone)]
struct CommitArgs<'a> {
    path: &'a Path,
    file_name: &'static str,
    message: &'static str,
    footer: Option<&'static str>,
    tag: Option<&'static str>,
}

fn add_commit(args: CommitArgs) -> Result<()> {
    // Create a file and commit it
    fs::write(args.path.join(args.file_name), args.message)?;

    Command::new("git")
        .arg("add")
        .arg(args.file_name)
        .current_dir(args.path)
        .status()?;

    let commit = |msg: &str| -> Result<()> {
        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg(msg)
            .current_dir(args.path)
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
            .current_dir(args.path)
            .status()?;
    }

    thread::sleep(Duration::from_millis(1000));

    Ok(())
}

fn setup() -> Result<TempDir> {
    let tmp_dir = TempDir::new()?;

    // Initialize Git repository
    Command::new("git")
        .arg("init")
        .current_dir(tmp_dir.path())
        .status()?;

    let mut args = CommitArgs {
        file_name: "init.txt",
        footer: None,
        message: "chore: init",
        path: tmp_dir.path(),
        tag: Some("v0.1.0"),
    };

    let result = add_commit(args.clone());
    assert!(result.is_ok());

    args.file_name = "feat.txt";
    args.footer = Some("I added a feature");
    args.message = "feat: new thing";
    args.tag = Some("v0.2.0");

    let result = add_commit(args.clone());
    assert!(result.is_ok());

    args.file_name = "fix.txt";
    args.footer = None;
    args.message = "fix: fixed a thing";
    args.tag = Some("v0.2.1");

    let result = add_commit(args.clone());
    assert!(result.is_ok());

    args.file_name = "break.txt";
    args.footer = Some("BREAKING CHANGE: It broke");
    args.message = "fix!: fixed it by breaking it";
    args.tag = None;

    let result = add_commit(args.clone());
    assert!(result.is_ok());

    let result = env::set_current_dir(args.path);
    assert!(result.is_ok());

    Ok(tmp_dir)
}

fn tear_down(tmp_dir: TempDir) -> Result<()> {
    tmp_dir.close()?;
    Ok(())
}

#[test]
fn process_git_repository() {
    let result = setup();
    assert!(result.is_ok());
    let tmp_dir = result.unwrap();

    let config = Config::default();
    let result =
        GitCliffChangelog::new(config.changelog, config.packages[0].clone());
    assert!(result.is_ok());

    let changelog = result.unwrap();
    let result = changelog.generate();
    assert!(result.is_ok());
    let out = result.unwrap();

    let result = changelog.current_version();
    assert!(result.is_some());
    let current = result.unwrap();

    let result = changelog.next_version();
    assert!(result.is_some());
    let next = result.unwrap();

    let result = changelog.next_is_breaking();
    assert!(result.is_ok());
    let is_breaking = result.unwrap();

    assert_ne!(out, "");
    assert_eq!(current, "v0.2.1");
    assert_eq!(next, "v1.0.0");
    assert!(is_breaking);

    let result = tear_down(tmp_dir);
    assert!(result.is_ok());
}
