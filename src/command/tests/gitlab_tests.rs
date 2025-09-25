// use nanoid::nanoid;
// use std::{env, thread, time::Duration};

// use crate::{
//     cli::{Args, Command, DEFAULT_COMMIT_SEARCH_DEPTH},
//     command::{
//         release::execute as execute_release,
//         release_pr::execute as execute_release_pr, tests::common,
//     },
//     forge::request::GetPrRequest,
// };

// #[test]
// fn gitlab_e2e_test() {
//     let token = env::var("GL_TEST_TOKEN").unwrap();

//     let args = Args {
//         command: Command::ReleasePR,
//         debug: true,
//         commit_search_depth: DEFAULT_COMMIT_SEARCH_DEPTH,
//         github_repo: "".into(),
//         github_token: "".into(),
//         gitea_repo: "".into(),
//         gitea_token: "".into(),
//         gitlab_repo: "https://gitlab.com/rgon/test-repo".into(),
//         gitlab_token: token,
//     };

//     let (forge, repo, tmp) = common::init(&args).unwrap();

//     let id = nanoid!();

//     let service1_msg = format!("fix(service-1): my fancy feature {}", id);

//     common::overwrite_file(
//         tmp.path().join("service-1/README.md"),
//         &service1_msg,
//     )
//     .unwrap();

//     repo.add_all().unwrap();

//     repo.commit(&service1_msg).unwrap();

//     let service2_msg = format!(
//         "feat(service-2)!: my fancy feature {id}\n\nBREAKING CHANGE: Adds a feature but breaks compatibility"
//     );

//     common::overwrite_file(
//         tmp.path().join("service-2/README.md"),
//         &service2_msg,
//     )
//     .unwrap();

//     repo.add_all().unwrap();

//     repo.commit(&service2_msg).unwrap();

//     repo.push_branch("main").unwrap();

//     execute_release_pr(&args).unwrap();

//     // sleep to ensure time for created PR with label to be queryable
//     thread::sleep(Duration::from_millis(2000));

//     let req = GetPrRequest {
//         base_branch: "main".into(),
//         head_branch: "releasaurus-release--main".into(),
//     };

//     let pr = forge.get_open_release_pr(req).unwrap().unwrap();

//     common::merge_gitlab_release_pr(pr, forge.config()).unwrap();

//     // sleep to ensure time for merged PR with label to be queryable
//     thread::sleep(Duration::from_millis(2000));

//     execute_release(&args).unwrap();

//     // keep tmp dir around until tests finish
//     drop(tmp);
// }
