//! Language-specific version file updaters.
//!
//! [`manager::UpdateManager`] orchestrates manifest loading and
//! file change generation. [`traits::PackageUpdater`] is the trait
//! implemented by each language module. [`dispatch::Updater`]
//! selects the right updater at runtime using static dispatch.

pub mod composite;
pub mod dispatch;
pub mod generic;
pub mod go;
pub mod java;
pub mod manager;
pub mod node;
pub mod php;
pub mod python;
pub mod ruby;
pub mod rust;
pub mod traits;
