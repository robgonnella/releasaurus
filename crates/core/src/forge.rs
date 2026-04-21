//! Forge platform abstractions and implementations.
//!
//! The [`traits::Forge`] trait defines the common interface.
//! Implementations: [`github`], [`gitlab`], [`gitea`], [`local`].
//! [`manager::ForgeManager`] wraps any `Forge` with caching,
//! logging, and dry-run support.

pub mod config;
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
