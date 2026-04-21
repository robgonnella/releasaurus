use secrecy::SecretString;
use std::env;

use crate::forge::{
    github::Github,
    manager::{ForgeManager, ForgeOptions},
    tests::common::{
        github::GithubForgeTestHelper,
        run::{parse_repo_url, run_forge_test},
    },
};

#[tokio::test]
#[test_log::test]
async fn test_github_forge() {
    let repo = parse_repo_url(
        &env::var("GITHUB_TEST_REPO")
            .expect("GITHUB_TEST_REPO env var must be set"),
    );
    let token_str = env::var("GITHUB_TEST_TOKEN")
        .expect("GITHUB_TEST_TOKEN env var must be set");
    let token_secret = SecretString::from(token_str.clone());
    let reset_sha = env::var("GITHUB_RESET_SHA")
        .expect("GITHUB_RESET_SHA env var must be set");

    let helper =
        GithubForgeTestHelper::new(&repo, &token_str, &reset_sha).await;
    let github_forge = Github::new(repo, Some(token_secret))
        .await
        .expect("failed to create Github forge");
    let manager = ForgeManager::new(
        Box::new(github_forge),
        ForgeOptions { dry_run: false },
    );

    run_forge_test(&manager, &helper).await;
}
