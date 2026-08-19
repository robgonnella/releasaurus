//! Forge platform abstractions and implementations.
//!
//! The [`traits::Forge`] trait defines the common interface.
//! Implementations: [`github`], [`gitlab`], [`gitea`], [`forgejo`],
//! [`azure_devops`], [`local`].
//! [`manager::ForgeManager`] wraps any `Forge` with caching,
//! logging, and dry-run support.
//! [`config_loader`] reads `releasaurus.toml` from a repository
//! before the manager is built.

pub mod azure_devops;
pub mod config;
pub mod config_loader;
pub mod forgejo;
pub mod gitea;
pub mod github;
pub mod gitlab;
pub mod local;
pub mod manager;
pub mod request;
pub mod traits;

#[cfg(test)]
#[cfg(feature = "_integration_tests")]
mod tests;
