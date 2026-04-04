# Using releasaurus-core as a Library

The `releasaurus-core` crate exposes the full release pipeline as a
public Rust API. Use it when you want to embed release automation
directly in your own tooling rather than shelling out to the CLI.

## When to use the library vs the CLI

| Situation                                            | Recommendation  |
| ---------------------------------------------------- | --------------- |
| CI/CD pipeline, GitHub Actions                       | Use the CLI     |
| Custom Rust tooling or internal build system         | Use the library |
| Need to integrate release data into another workflow | Use the library |
| Simple changelog + tag automation                    | Use the CLI     |

## Adding the dependency

```toml
[dependencies]
releasaurus-core = "0.14"
```

The crate is async-first and built on [Tokio], so you will also need:

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
```

## Architecture overview

```text
Orchestrator        (pipeline entry point)
  └─ OrchestratorConfig   (merged settings)
  └─ ForgeManager         (caching + dry-run wrapper)
       └─ Forge           (GitHub / GitLab / Gitea / Local)
```

All release operations go through `Orchestrator`. Building one
requires three pieces:

1. A **`ForgeManager`** wrapping a concrete `Forge` implementation
2. An **`OrchestratorConfig`** built from the loaded TOML config plus
   any runtime overrides
3. A **`ResolvedPackageHash`** — the set of packages parsed from
   `releasaurus.toml`, each merged with the orchestrator config

See the [crate-level quick start on docs.rs][docs-rs] for the full
builder chain with inline comments for each step.

## Constructing a RepoUrl

The forge constructors (`Github::new`, `Gitlab::new`, `Gitea::new`)
take a `RepoUrl` — an owned struct defined in this crate — instead of
any third-party URL type. This keeps your dependency tree stable
regardless of which URL parser you prefer.

Construct it directly from the components of your parsed URL:

```rust,no_run
use releasaurus_core::forge::{RepoUrl, config::Scheme};

let url = RepoUrl {
    scheme: Scheme::Https,
    host: "github.com".into(),
    owner: "my-org".into(),
    name: "my-repo".into(),
    // Full project path — for nested GitLab groups this may be
    // "group/subgroup/repo"
    path: "my-org/my-repo".into(),
    port: None,
    token: None,
};
```

Set `token` only if the credential is embedded directly in the URL
(e.g. `https://TOKEN@host/...`); otherwise leave it `None` and pass
the token as `Option<secrecy::SecretString>` directly to the forge
constructor (`Github::new`, `Gitlab::new`, `Gitea::new`). You will
need to add [`secrecy`](https://docs.rs/secrecy) to your own
dependencies to construct a `SecretString`.

## The Forge trait

`Forge` is the extension point for platform support. The crate ships
four implementations:

| Type                      | When to use                     |
| ------------------------- | ------------------------------- |
| `forge::github::Github`   | GitHub (cloud or Enterprise)    |
| `forge::gitlab::Gitlab`   | GitLab (cloud or self-hosted)   |
| `forge::gitea::Gitea`     | Gitea self-hosted               |
| `forge::local::LocalRepo` | Local git2 operations (testing) |

To target a custom platform, implement the `Forge` trait from
`releasaurus_core::forge::traits`:

```rust,no_run
use async_trait::async_trait;
use releasaurus_core::{
    analyzer::release::Tag,
    config::Config,
    error::Result,
    forge::{
        request::{
            Commit, CreateCommitRequest, CreatePrRequest,
            CreateReleaseBranchRequest, ForgeCommit,
            GetFileContentRequest, GetPrRequest, PrLabelsRequest,
            PullRequest, ReleaseByTagResponse, UpdatePrRequest,
        },
        traits::Forge,
    },
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

    // ... remaining trait methods
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

Pass your implementation to `ForgeManager::new(Box::new(my_forge), ...)`.

## The processing pipeline

Internally, each `Orchestrator` call drives packages through a
sequence of typed stages:

```text
ResolvedPackage
  → PreparedPackage    (commits fetched, filtered by path + timestamp)
  → AnalyzedPackage    (conventional commits parsed, next version
                        calculated, changelog generated)
  → ReleasablePackage  (manifest files loaded, file changes prepared)
  → ReleasePRPackage   (PR branch created or updated)
```

`create_releases` runs a shorter path: it reads the merged PR body,
extracts the embedded release metadata, tags the commit, and calls
the forge's `create_release`.

Understanding the stages helps when reading error messages — the
stage name appears in most error contexts.

## Dry-run mode

Pass `ForgeOptions { dry_run: true }` to `ForgeManager::new`. In
dry-run mode all write operations (branch creation, PR creation,
tagging, release publishing) are skipped and logged at `WARN` level.
Read operations proceed normally, so you can safely preview what
would happen.

## Testing without a live forge

`LocalRepo` (from `releasaurus_core::forge::local`) runs all
operations against a local git repository using `git2`. It is the
fastest way to integration-test a custom pipeline without hitting any
remote API.

Alternatively, the `Forge` trait is annotated with `#[cfg_attr(test,
automock)]`, so `MockForge` from `mockall` is available under
`#[cfg(test)]` for unit tests.

[Tokio]: https://tokio.rs
[docs-rs]: https://docs.rs/releasaurus-core
