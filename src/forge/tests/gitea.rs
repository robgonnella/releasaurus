use git_url_parse::GitUrl;
use secrecy::SecretString;
use std::env;
use tokio::time::Duration;

use crate::{
    ForgeManager,
    cli::{ForgeArgs, ForgeType},
    forge::{
        manager::ForgeOptions,
        tests::common::{gitea::GiteaForgeTestHelper, run::run_forge_test},
    },
};

#[tokio::test]
#[test_log::test]
async fn test_gitea_forge() {
    let repo = GitUrl::parse(&env::var("GITEA_TEST_REPO").unwrap()).unwrap();
    let token_str = env::var("GITEA_TEST_TOKEN").unwrap();
    let token_secret = SecretString::from(token_str.clone());

    let reset_sha = env::var("GITEA_RESET_SHA").unwrap();

    let forge_args = ForgeArgs {
        forge: Some(ForgeType::Gitea),
        repo: Some(repo.clone()),
        token: Some(token_secret),
    };

    let forge = forge_args.forge().await.unwrap();

    let manager = ForgeManager::new(forge, ForgeOptions { dry_run: false });

    let helper = GiteaForgeTestHelper::new(&repo, &token_str, &reset_sha).await;

    run_forge_test(&manager, &helper, Duration::from_millis(2000)).await;
}
