use std::env;
use tokio::time::Duration;

use crate::{
    Cli, Command, ForgeFactory,
    cli::ForgeType,
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

    let cli = Cli {
        forge: Some(ForgeType::Github),
        base_branch: None,
        command: Command::Release, // doesn't matter just using to create forge
        debug: true,
        dry_run: false,
        repo: Some(repo.clone()),
        token: Some(token.clone()),
    };

    let remote = cli.get_remote().unwrap();
    let gitea_forge = ForgeFactory::create(&remote).await.unwrap();
    let helper = GithubForgeTestHelper::new(&repo, &token, &reset_sha).await;

    run_forge_test(&gitea_forge, &helper, Duration::from_millis(10000)).await;
}
