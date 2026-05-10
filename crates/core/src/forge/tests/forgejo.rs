use secrecy::SecretString;
use std::env;

use crate::forge::{
    forgejo::Forgejo,
    manager::{ForgeManager, ForgeOptions},
    tests::common::{
        forgejo::ForgejoForgeTestHelper,
        run::{parse_repo_url, run_forge_test},
    },
};

#[tokio::test]
#[test_log::test]
async fn test_forgejo_forge() {
    let repo = parse_repo_url(
        &env::var("FORGEJO_TEST_REPO")
            .expect("FORGEJO_TEST_REPO env var must be set"),
    );

    let token_str = env::var("FORGEJO_TEST_TOKEN")
        .expect("FORGEJO_TEST_TOKEN env var must be set");

    let token_secret = SecretString::from(token_str.clone());

    let reset_sha = env::var("FORGEJO_RESET_SHA")
        .expect("FORGEJO_RESET_SHA env var must be set");

    let helper =
        ForgejoForgeTestHelper::new(&repo, &token_str, &reset_sha).await;

    let forgejo_forge = Forgejo::new(repo, Some(token_secret))
        .await
        .expect("failed to create Forgejo forge");

    let manager = ForgeManager::new(
        Box::new(forgejo_forge),
        ForgeOptions { dry_run: false },
    );

    run_forge_test(&manager, &helper).await;
}
