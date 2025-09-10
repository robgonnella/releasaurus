use nanoid::nanoid;
use std::{env, thread, time::Duration};

use crate::{
    cli::{Args, Command},
    command::{
        release::execute as execute_release,
        release_pr::execute as execute_release_pr, tests::common,
    },
    forge::types::GetPrRequest,
};

#[test]
fn github_e2e_test() {
    let token = env::var("GH_TEST_TOKEN").unwrap();

    let args = Args {
        command: Command::ReleasePR,
        debug: true,
        clone_depth: 0,
        github_repo: "https://github.com/robgonnella/test-repo".into(),
        github_token: token,
        gitea_repo: "".into(),
        gitea_token: "".into(),
        gitlab_repo: "".into(),
        gitlab_token: "".into(),
    };

    let (forge, repo, tmp) = common::init(&args).unwrap();

    let id = nanoid!();

    let msg = format!("feat({id}): my fancy feature");

    common::overwrite_file(tmp.path().join("README.md"), &msg).unwrap();

    repo.add_all().unwrap();

    repo.commit(&msg).unwrap();

    repo.push_branch("main").unwrap();

    execute_release_pr(&args).unwrap();

    // sleep to ensure time for created PR with label to be queryable
    thread::sleep(Duration::from_millis(2000));

    let req = GetPrRequest {
        base_branch: "main".into(),
        head_branch: "releasaurus-release--main".into(),
    };

    let pr = forge.get_open_release_pr(req).unwrap().unwrap();

    common::merge_github_release_pr(pr, forge.config()).unwrap();

    // sleep to ensure time for merged PR with label to be queryable
    thread::sleep(Duration::from_millis(2000));

    execute_release(&args).unwrap();

    // keep tmp dir around until tests finish
    drop(tmp);
}
