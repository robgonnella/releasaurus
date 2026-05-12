use secrecy::SecretString;
use std::env;

use crate::forge::{
    azure_devops::{AzureDevops, url_parse::azure_git_url_to_repo_url},
    manager::{ForgeManager, ForgeOptions},
    tests::common::{
        azure_devops::AzureDevopsForgeTestHelper, run::run_forge_test,
    },
};

#[tokio::test]
#[test_log::test]
async fn test_azure_devops_forge() {
    let repo = azure_git_url_to_repo_url(
        &env::var("AZURE_DEVOPS_TEST_REPO")
            .expect("AZURE_DEVOPS_TEST_REPO env var must be set"),
    )
    .expect("AZURE_DEVOPS_TEST_REPO must be a valid azure devops url");

    let token_str = env::var("AZURE_DEVOPS_TEST_TOKEN")
        .expect("AZURE_DEVOPS_TEST_TOKEN env var must be set");

    let token_secret = SecretString::from(token_str.clone());

    let reset_sha = env::var("AZURE_DEVOPS_RESET_SHA")
        .expect("AZURE_DEVOPS_RESET_SHA env var must be set");

    let helper =
        AzureDevopsForgeTestHelper::new(&repo, &token_str, &reset_sha).await;

    let azure_forge = AzureDevops::new(repo, Some(token_secret))
        .await
        .expect("failed to create AzureDevops forge");

    let manager = ForgeManager::new(
        Box::new(azure_forge),
        ForgeOptions { dry_run: false },
    );

    run_forge_test(&manager, &helper).await;
}
