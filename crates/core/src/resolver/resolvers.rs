//! Individual resolution functions used by
//! [`Resolver::resolve`][crate::resolver::Resolver::resolve].
//!
//! Each sub-module handles one aspect of package config resolution:
//! base branch, tag prefix, prerelease settings, commit modifiers,
//! sub-packages, etc.

pub mod analyzer;
pub mod auto_start;
pub mod base_branch;
pub mod commit_modifiers;
pub mod manifest;
pub mod package;
pub mod package_name;
pub mod path_utils;
pub mod prerelease;
pub mod sub_packages;
pub mod tag_prefix;
pub mod version_increment;

#[cfg(test)]
pub mod test_helper;
