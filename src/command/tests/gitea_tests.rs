use nanoid::nanoid;
use std::env;

use crate::{
    cli::{Args, Command},
    command::{
        release::execute as execute_release,
        release_pr::execute as execute_release_pr, tests::common,
    },
    forge::types::GetPrRequest,
};

#[test]
fn gitea_e2e_test() {
    let token = env::var("GT_TEST_TOKEN").unwrap();

    let args = Args {
        command: Command::ReleasePR,
        debug: true,
        dry_run: false,
        github_repo: "".into(),
        github_token: "".into(),
        gitea_repo: "https://gitea.com/rgon/test-repo".into(),
        gitea_token: token,
        gitlab_repo: "".into(),
        gitlab_token: "".into(),
    };

    let (forge, repo, tmp) = common::init(&args).unwrap();

    common::switch_current_directory(tmp.path(), || {
        let id = nanoid!();

        let msg = format!("feat({}): my fancy feature", id);

        common::overwrite_file("./README.md", &msg).unwrap();

        repo.add_all().unwrap();

        repo.commit(&msg).unwrap();

        repo.push_branch("main").unwrap();

        execute_release_pr(&args).unwrap();

        let req = GetPrRequest {
            base_branch: "main".into(),
            head_branch: "releasaurus-release--main".into(),
        };

        let pr = forge.get_open_release_pr(req).unwrap().unwrap();

        common::merge_gitea_release_pr(pr, forge.config()).unwrap();

        execute_release(&args).unwrap();
    })
    .unwrap();
}
