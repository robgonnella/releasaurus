//! Pipeline stage types that packages progress through.
//!
//! Packages move through these stages in order:
//! [`resolved::ResolvedPackage`] → [`prepared::PreparedPackage`] →
//! [`analyzed::AnalyzedPackage`] →
//! [`releasable::ReleasablePackage`] →
//! [`release_pr::ReleasePRPackage`]

pub mod analyzed;
pub mod manifests;
pub mod prepared;
pub mod releasable;
pub mod releasable_builder;
pub mod release_pr;
pub mod resolved;
pub mod resolved_hash;
