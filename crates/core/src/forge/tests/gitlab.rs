use secrecy::SecretString;
use std::env;

use crate::forge::{
    gitlab::Gitlab,
    manager::{ForgeManager, ForgeOptions},
    tests::common::{
        gitlab::GitlabForgeTestHelper,
        run::{parse_repo_url, run_forge_test},
    },
};

#[tokio::test]
#[test_log::test]
async fn test_gitlab_forge() {
    let repo = parse_repo_url(
        &env::var("GITLAB_TEST_REPO")
            .expect("GITLAB_TEST_REPO env var must be set"),
    );
    let token_str = env::var("GITLAB_TEST_TOKEN")
        .expect("GITLAB_TEST_TOKEN env var must be set");
    let token_secret = SecretString::from(token_str.clone());
    let reset_sha = env::var("GITLAB_RESET_SHA")
        .expect("GITLAB_RESET_SHA env var must be set");

    let helper =
        GitlabForgeTestHelper::new(&repo, &token_str, &reset_sha).await;
    let gitlab_forge = Gitlab::new(repo, Some(token_secret))
        .await
        .expect("failed to create Gitlab forge");
    let manager = ForgeManager::new(
        Box::new(gitlab_forge),
        ForgeOptions { dry_run: false },
    );

    run_forge_test(&manager, &helper).await;
}
