use std::env;
use tokio::time::Duration;

use crate::{
    ForgeFactory,
    cli::{ForgeArgs, ForgeType},
    forge::tests::common::{
        github::GithubForgeTestHelper, run::run_forge_test,
    },
};

#[tokio::test]
#[test_log::test]
async fn test_github_forge() {
    let repo = env::var("GITHUB_TEST_REPO").unwrap();
    let token = env::var("GITHUB_TEST_TOKEN").unwrap();
    let reset_sha = env::var("GITHUB_RESET_SHA").unwrap();

    let forge_args = ForgeArgs {
        forge: Some(ForgeType::Github),
        repo: Some(repo.clone()),
        token: Some(token.clone()),
        dry_run: false,
    };

    let remote = forge_args.get_remote().unwrap();
    let gitea_forge = ForgeFactory::create(&remote).await.unwrap();
    let helper = GithubForgeTestHelper::new(&repo, &token, &reset_sha).await;

    run_forge_test(&gitea_forge, &helper, Duration::from_millis(10000)).await;
}
