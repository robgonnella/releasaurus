# Library API

The `releasaurus-core` crate exposes the full release pipeline as a
public Rust API — use it to embed release automation in your own tooling
instead of shelling out to the CLI. (For CI/CD and simple automation, the
CLI is the better choice.)

## Adding the Dependency

```toml
[dependencies]
releasaurus-core = "0.14"
tokio = { version = "1", features = ["full"] }  # async-first, built on Tokio
```

## Architecture

```text
Orchestrator            (pipeline entry point)
  └─ ResolvedConfig     (merged settings)
  └─ ResolvedPackageHash (resolved package configs)
  └─ ForgeManager       (caching + dry-run wrapper)
       └─ Forge         (GitHub / GitLab / Gitea / Local)
```

All operations go through `Orchestrator`, which needs three pieces:

1. A **`ForgeManager`** wrapping a concrete `Forge`.
2. A **`ResolvedConfig`** — built by `Resolver::builder()` from the loaded
   TOML plus any runtime overrides.
3. A **`ResolvedPackageHash`** — the resolved packages, produced alongside
   `ResolvedConfig` by `Resolver::resolve()`.

See the [crate-level quick start on docs.rs][docs-rs] for the full builder
chain with per-step comments.

Internally each call drives packages through typed stages —
`ResolvedPackage → PreparedPackage → AnalyzedPackage → ReleasablePackage
→ ReleasePRPackage`. The stage name appears in most error contexts, which
helps when reading errors.

## Constructing a RepoUrl

Forge constructors (`Github::new`, `Gitlab::new`, `Gitea::new`) take a
`RepoUrl` defined in this crate rather than a third-party URL type, so
your dependency tree stays stable. Build it from your parsed URL's
components:

```rust,no_run
use releasaurus_core::forge::{RepoUrl, config::Scheme};

let url = RepoUrl {
    scheme: Scheme::Https,
    host: "github.com".into(),
    owner: "my-org".into(),
    name: "my-repo".into(),
    // Full project path — nested GitLab groups may be "group/subgroup/repo"
    path: "my-org/my-repo".into(),
    port: None,
    token: None,
};
```

Set `token` only when the credential is embedded in the URL
(`https://TOKEN@host/...`); otherwise leave it `None` and pass the token
as `Option<secrecy::SecretString>` to the forge constructor (add
[`secrecy`](https://docs.rs/secrecy) to construct one).

## The Forge Trait

`Forge` is the extension point for platform support. The crate ships four
implementations:

| Type                      | When to use                     |
| ------------------------- | ------------------------------- |
| `forge::github::Github`   | GitHub (cloud or Enterprise)    |
| `forge::gitlab::Gitlab`   | GitLab (cloud or self-hosted)   |
| `forge::gitea::Gitea`     | Gitea self-hosted               |
| `forge::local::LocalRepo` | Local git2 operations (testing) |

To target a custom platform, implement `Forge` from
`releasaurus_core::forge::traits` and pass it to
`ForgeManager::new(Box::new(my_forge), ...)`:

```rust,no_run
use async_trait::async_trait;
use releasaurus_core::{
    config::Config,
    forge::{
        request::Tag,
        request::{
            Commit, CreateCommitRequest, CreatePrRequest,
            CreateReleaseBranchRequest, ForgeCommit,
            GetFileContentRequest, GetPrRequest, PrLabelsRequest,
            PullRequest, ReleaseByTagResponse, UpdatePrRequest,
        },
        traits::Forge,
    },
    result::Result,
};
use std::any::Any;
use url::Url;

pub struct MyForge { /* ... */ }

#[async_trait]
impl Forge for MyForge {
    fn repo_name(&self) -> String { todo!() }
    fn release_link_base_url(&self) -> Url { todo!() }
    fn compare_link_base_url(&self) -> Url { todo!() }
    fn default_branch(&self) -> String { todo!() }

    async fn load_config(
        &self,
        branch: Option<String>,
    ) -> Result<Config> { todo!() }

    async fn get_file_content(
        &self,
        req: GetFileContentRequest,
    ) -> Result<Option<String>> { todo!() }

    // ... remaining trait methods (see docs.rs for the full list)
    # async fn get_release_by_tag(&self, _: &str)
    #     -> Result<ReleaseByTagResponse> { todo!() }
    # async fn create_release_branch(&self, _: CreateReleaseBranchRequest)
    #     -> Result<Commit> { todo!() }
    # async fn create_commit(&self, _: CreateCommitRequest)
    #     -> Result<Commit> { todo!() }
    # async fn tag_commit(&self, _: &str, _: &str)
    #     -> Result<()> { todo!() }
    # async fn get_latest_tags_for_prefix(&self, _: &str, _: &str)
    #     -> Result<Vec<Tag>> { todo!() }
    # async fn get_commits(&self, _: Option<String>, _: Option<String>)
    #     -> Result<Vec<ForgeCommit>> { todo!() }
    # async fn get_open_release_pr(&self, _: GetPrRequest)
    #     -> Result<Option<PullRequest>> { todo!() }
    # async fn get_merged_release_pr(&self, _: GetPrRequest)
    #     -> Result<Option<PullRequest>> { todo!() }
    # async fn create_pr(&self, _: CreatePrRequest)
    #     -> Result<PullRequest> { todo!() }
    # async fn update_pr(&self, _: UpdatePrRequest)
    #     -> Result<()> { todo!() }
    # async fn replace_pr_labels(&self, _: PrLabelsRequest)
    #     -> Result<()> { todo!() }
    # async fn create_release(&self, _: &str, _: &str, _: &str)
    #     -> Result<()> { todo!() }
}
```

## Dry-Run & Testing

Pass `ForgeOptions { dry_run: true }` to `ForgeManager::new` to skip all
write operations (logged at `WARN`) while read operations proceed
normally. For tests, `LocalRepo` runs everything against a local git2
repository; the `Forge` trait is also `#[cfg_attr(test, automock)]`, so
`mockall`'s `MockForge` is available under `#[cfg(test)]`.

[docs-rs]: https://docs.rs/releasaurus-core
