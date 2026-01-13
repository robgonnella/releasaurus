use std::env;
use tokio::time::Duration;

use crate::{
    ForgeFactory,
    cli::{ForgeArgs, ForgeType},
    forge::{
        manager::ForgeOptions,
        tests::common::{gitlab::GitlabForgeTestHelper, run::run_forge_test},
    },
};

#[tokio::test]
#[test_log::test]
async fn test_gitlab_forge() {
    let repo = env::var("GITLAB_TEST_REPO").unwrap();
    let token = env::var("GITLAB_TEST_TOKEN").unwrap();
    let reset_sha = env::var("GITLAB_RESET_SHA").unwrap();

    let forge_args = ForgeArgs {
        forge: Some(ForgeType::Gitlab),
        repo: Some(repo.clone()),
        token: Some(token.clone()),
    };

    let remote = forge_args.get_remote().unwrap();
    let gitea_forge =
        ForgeFactory::create(&remote, ForgeOptions { dry_run: false })
            .await
            .unwrap();
    let helper = GitlabForgeTestHelper::new(&repo, &token, &reset_sha).await;

    run_forge_test(&gitea_forge, &helper, Duration::from_millis(2000)).await;
}
