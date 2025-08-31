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
fn gitlab_e2e_test() {
    let token = env::var("GITLAB_TOKEN").unwrap();

    let args = Args {
        command: Command::ReleasePR,
        debug: true,
        dry_run: false,
        github_repo: "".into(),
        github_token: "".into(),
        gitea_repo: "".into(),
        gitea_token: "".into(),
        gitlab_repo: "https://gitlab.com/rgon/test-repo".into(),
        gitlab_token: token,
    };

    let (forge, repo, tmp) = common::init(&args).unwrap();

    common::switch_current_directory(tmp.path(), || {
        let id = nanoid!();

        let service1_msg = format!("fix(service-1): my fancy feature {}", id);

        common::overwrite_file("./service-1/README.md", &service1_msg).unwrap();

        repo.add_all().unwrap();

        repo.commit(&service1_msg).unwrap();

        let service2_msg = format!("feat(service-2)!: my fancy feature {}\n\nBREAKING CHANGE: Adds a feature but breaks compatibility", id);

        common::overwrite_file("./service-2/README.md", &service2_msg).unwrap();

        repo.add_all().unwrap();

        repo.commit(&service2_msg).unwrap();

        repo.push_branch("main").unwrap();

        let result = execute_release_pr(&args);

        assert!(result.is_ok());

        let req = GetPrRequest {
            base_branch: "main".into(),
            head_branch: "releasaurus-release--main".into(),
        };

        let pr = forge.get_open_release_pr(req).unwrap().unwrap();

        common::merge_gitlab_release_pr(pr, forge.config()).unwrap();

        let result = execute_release(&args);

        assert!(result.is_ok());
    })
    .unwrap();
}
