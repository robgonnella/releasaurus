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
fn github_e2e_test() {
    let token = env::var("GITHUB_TOKEN").unwrap();

    let args = Args {
        command: Command::ReleasePR,
        debug: true,
        dry_run: false,
        github_repo: "https://github.com/robgonnella/test-repo".into(),
        github_token: token,
        gitea_repo: "".into(),
        gitea_token: "".into(),
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

        let result = execute_release_pr(&args);

        assert!(result.is_ok());

        let req = GetPrRequest {
            base_branch: "main".into(),
            head_branch: "releasaurus-release--main".into(),
        };

        let pr = forge.get_open_release_pr(req).unwrap().unwrap();

        common::merge_github_release_pr(pr, forge.config()).unwrap();

        let result = execute_release(&args);

        assert!(result.is_ok());
    })
    .unwrap();
}
