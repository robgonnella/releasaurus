//! # releasaurus-core
//!
//! Core library powering [Releasaurus] release automation. Use this
//! crate to embed the full release pipeline in your own Rust tooling
//! without taking a dependency on the CLI binary.
//!
//! ## Architecture
//!
//! ```text
//! Orchestrator        (pipeline entry point)
//!   └─ OrchestratorConfig   (merged settings)
//!   └─ ForgeManager         (caching + dry-run wrapper)
//!        └─ Forge           (GitHub / GitLab / Gitea / Local)
//! ```
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use std::{collections::HashMap, rc::Rc};
//! use releasaurus_core::{
//!     forge::{
//!         github::Github,
//!         manager::{ForgeManager, ForgeOptions},
//!         config::{RepoUrl, Scheme},
//!     },
//!     orchestrator::{
//!         Orchestrator,
//!         config::{
//!             CommitModifiers, GlobalOverrides, OrchestratorConfig,
//!         },
//!         package::resolved::{
//!             ResolvedPackage, ResolvedPackageHash,
//!         },
//!     },
//! };
//!
//! #[tokio::main]
//! async fn main() -> releasaurus_core::error::Result<()> {
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
//!         forge_manager.load_config(None).await?,
//!     );
//!
//!     // 3. Build the orchestrator config.
//!     let orch_config = Rc::new(
//!         OrchestratorConfig::builder()
//!             .toml_config(Rc::clone(&config))
//!             .repo_name(forge_manager.repo_name())
//!             .repo_default_branch(
//!                 forge_manager.default_branch(),
//!             )
//!             .release_link_base_url(
//!                 forge_manager.release_link_base_url().clone(),
//!             )
//!             .compare_link_base_url(
//!                 forge_manager.compare_link_base_url().clone(),
//!             )
//!             .global_overrides(GlobalOverrides::default())
//!             .package_overrides(HashMap::new())
//!             .commit_modifiers(CommitModifiers::default())
//!             .build()?,
//!     );
//!
//!     // 4. Resolve each package declared in releasaurus.toml.
//!     let mut resolved = vec![];
//!     for pkg in config.packages.iter() {
//!         resolved.push(
//!             ResolvedPackage::builder()
//!                 .orchestrator_config(Rc::clone(&orch_config))
//!                 .package_config(pkg.clone())
//!                 .build()?,
//!         );
//!     }
//!
//!     // 5. Build the orchestrator and run the pipeline.
//!     let fm = Rc::new(forge_manager);
//!     let orchestrator = Orchestrator::builder()
//!         .config(Rc::clone(&orch_config))
//!         .package_configs(Rc::new(
//!             ResolvedPackageHash::new(resolved)?,
//!         ))
//!         .forge(Rc::clone(&fm))
//!         .build()?;
//!
//!     orchestrator.create_release_prs(None).await
//! }
//! ```
//!
//! ## Modules
//!
//! - [`analyzer`] — conventional commit parsing and version
//!   calculation
//! - [`config`] — TOML configuration types and deserialization
//! - [`error`] — [`ReleasaurusError`][error::ReleasaurusError]
//!   and [`Result`][error::Result]
//! - [`file_loader`] — trait for loading files from a forge or
//!   filesystem
//! - [`forge`] — [`Forge`][forge::traits::Forge] trait and
//!   platform implementations (GitHub, GitLab, Gitea, Local)
//! - [`orchestrator`] — all release operations flow through
//!   [`Orchestrator`][orchestrator::Orchestrator]
//! - [`updater`] — language-specific version file updaters
//!
//! [Releasaurus]: https://releasaurus.rgon.io

pub mod analyzer;
pub mod config;
pub mod error;
pub mod file_loader;
pub mod forge;
pub mod orchestrator;
pub mod updater;
