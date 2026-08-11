//! # releasaurus-core
//!
//! Core library powering [Releasaurus] release automation. Use this
//! crate to embed the full release pipeline in your own Rust tooling
//! without taking a dependency on the CLI binary.
//!
//! ## Architecture
//!
//! ```text
//! Orchestrator          (pipeline entry point)
//!   └─ ResolvedConfig   (merged, validated settings)
//!        └─ ResolvedPackageHash (resolved package configs)
//!   └─ ForgeManager     (caching + dry-run wrapper)
//!        └─ Forge       (GitHub / GitLab / Gitea / Local)
//! ```
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use std::{collections::HashMap, rc::Rc};
//! use releasaurus_core::{
//!     config::overrides::{CommitModifiers, GlobalOverrides},
//!     forge::{
//!         github::Github,
//!         manager::{ForgeManager, ForgeOptions},
//!         config::{RepoUrl, Scheme},
//!     },
//!     orchestrator::{Orchestrator, SerializableReleasablePackage},
//!     resolver::Resolver,
//! };
//!
//! #[tokio::main]
//! async fn main() -> releasaurus_core::result::Result<()> {
//!     let url = RepoUrl {
//!         scheme: Scheme::Https,
//!         host: "github.com".into(),
//!         owner: "my-org".into(),
//!         name: "my-repo".into(),
//!         path: "my-org/my-repo".into(),
//!         port: None,
//!         token: None,
//!     };
//!
//!     // 1. Build a forge client.
//!     let forge = Github::new(url, None).await?;
//!     let forge_manager = ForgeManager::new(
//!         Box::new(forge),
//!         ForgeOptions { dry_run: false },
//!     );
//!
//!     // 2. Load releasaurus.toml from the repository.
//!     let config = Rc::new(
//!         forge_manager.load_config(None, None).await?,
//!     );
//!
//!     // 3. Resolve packages and build config.
//!     let resolved_config = Resolver::builder()
//!         .toml_config(Rc::clone(&config))
//!         .repo_name(forge_manager.repo_name())
//!         .repo_default_branch(forge_manager.default_branch())
//!         .release_link_base_url(
//!             forge_manager.release_link_base_url().clone(),
//!         )
//!         .compare_link_base_url(
//!             forge_manager.compare_link_base_url().clone(),
//!         )
//!         .global_overrides(GlobalOverrides::default())
//!         .package_overrides(HashMap::new())
//!         .commit_modifiers(CommitModifiers::default())
//!         .build()?
//!         .resolve(config.packages.clone())?;
//!
//!     // 4. Build the orchestrator and run the pipeline.
//!     let fm = Rc::new(forge_manager);
//!     let orchestrator = Orchestrator::builder()
//!         .config(resolved_config)
//!         .forge(Rc::clone(&fm))
//!         .build()?;
//!
//!     // 5. Inspect the projected releases before acting on them.
//!     let next: Vec<SerializableReleasablePackage> =
//!         orchestrator.get_next_releases(None).await?;
//!     for pkg in &next {
//!         println!("{} → {}", pkg.name, pkg.release.tag.name);
//!     }
//!
//!     orchestrator.create_release_prs(None).await
//! }
//! ```
//!
//! ## Modules
//!
//! - [`config`] — TOML configuration types and deserialization;
//!   [`config::overrides`] holds the runtime override types callers
//!   supply from CLI flags
//! - [`result`] — [`ReleasaurusError`][result::ReleasaurusError]
//!   and [`Result`][result::Result]
//! - [`forge`] — [`Forge`][forge::traits::Forge] trait and platform
//!   implementations (GitHub, GitLab, Gitea, Local)
//! - [`resolver`] — merges TOML config, CLI overrides, and defaults
//!   into a [`resolver::ResolvedConfig`] holding the resolved
//!   [`resolver::ResolvedPackage`]s
//! - [`orchestrator`] — all release operations flow through
//!   [`Orchestrator`][orchestrator::Orchestrator]; commit analysis,
//!   package lifecycle staging, and language-specific manifest
//!   updating are internal to the pipeline, and only the types the
//!   orchestrator hands back are re-exported there
//!
//! [Releasaurus]: https://releasaurus.rgon.io

mod analyzer;
pub mod config;
pub mod forge;
pub mod orchestrator;
mod packages;
pub mod resolver;
pub mod result;
mod updater;
