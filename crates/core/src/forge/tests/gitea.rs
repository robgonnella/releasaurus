use secrecy::SecretString;
use std::env;

use crate::{
    config::repository::GitUserConfig,
    forge::{
        gitea::Gitea,
        manager::{ForgeManager, ForgeOptions},
        tests::common::{
            gitea::GiteaForgeTestHelper,
            run::{parse_repo_url, run_forge_test},
        },
        traits::Forge,
    },
};

#[tokio::test]
#[test_log::test]
async fn test_gitea_forge() {
    let repo = parse_repo_url(
        &env::var("GITEA_TEST_REPO")
            .expect("GITEA_TEST_REPO env var must be set"),
    );

    let token_str = env::var("GITEA_TEST_TOKEN")
        .expect("GITEA_TEST_TOKEN env var must be set");

    let token_secret = SecretString::from(token_str.clone());

    let reset_sha = env::var("GITEA_RESET_SHA")
        .expect("GITEA_RESET_SHA env var must be set");

    let helper = GiteaForgeTestHelper::new(&repo, &token_str, &reset_sha).await;

    let mut gitea_forge = Gitea::new(repo, Some(token_secret), None)
        .await
        .expect("failed to create Gitea forge");

    gitea_forge.set_git_user(Some(GitUserConfig {
        name: "Test User".into(),
        email: "test@releasaurus.io".into(),
    }));

    let manager = ForgeManager::new(
        Box::new(gitea_forge),
        ForgeOptions { dry_run: false },
    );

    run_forge_test(&manager, &helper).await;
}
