use git_url_parse::GitUrl;
use std::env;
use tokio::time::Duration;

use crate::{
    ForgeFactory,
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
    let token = env::var("GITEA_TEST_TOKEN").unwrap();
    let reset_sha = env::var("GITEA_RESET_SHA").unwrap();

    let forge_args = ForgeArgs {
        forge: Some(ForgeType::Gitea),
        repo: Some(repo.clone()),
        token: Some(token.clone()),
    };

    let remote = forge_args.get_remote().unwrap();
    let gitea_forge =
        ForgeFactory::create(&remote, ForgeOptions { dry_run: false })
            .await
            .unwrap();
    let helper = GiteaForgeTestHelper::new(&repo, &token, &reset_sha).await;

    run_forge_test(&gitea_forge, &helper, Duration::from_millis(2000)).await;
}
