//! Commit analysis and changelog generation for Releasaurus.
//!
//! This module provides the core functionality for analyzing Git commit history
//! and generating changelogs using the git-cliff library. It serves as the intelligence
//! behind Releasaurus's ability to understand commit patterns, categorize changes,
//! and produce meaningful release documentation.
//!
//! # Key Responsibilities
//!
//! - **Commit Analysis**: Parse and categorize commits based on conventional commit patterns
//! - **Version Detection**: Determine the next semantic version based on commit types
//! - **Changelog Generation**: Create formatted changelog entries with proper grouping
//! - **Release Boundaries**: Identify commit ranges for specific releases
//! - **Template Processing**: Apply customizable templates to format output
//!
//! # Architecture
//!
//! The analyzer module is built around the git-cliff library, which provides:
//! - Conventional commit parsing
//! - Configurable commit categorization
//! - Template-based changelog generation
//! - Git history traversal and analysis
//!
//! # Commit Categorization
//!
//! The analyzer recognizes and categorizes commits based on conventional commit patterns:
//!
//! - **feat**: New features (minor version bump)
//! - **fix**: Bug fixes (patch version bump)
//! - **BREAKING CHANGE**: Breaking changes (major version bump)
//! - **docs**: Documentation updates
//! - **style**: Code style changes
//! - **refactor**: Code refactoring
//! - **perf**: Performance improvements
//! - **test**: Test additions or modifications
//! - **chore**: Maintenance tasks
//!
//! # Version Determination
//!
//! Based on the commit analysis, the module determines the appropriate semantic
//! version increment:
//!
//! - **Major**: Presence of breaking changes
//! - **Minor**: New features without breaking changes
//! - **Patch**: Only bug fixes and non-functional changes
//!
//! # Template System
//!
//! Changelog generation uses the Tera templating engine, allowing for:
//! - Custom formatting of changelog entries
//! - Grouping of commits by type or scope
//! - Integration of repository metadata
//! - Flexible output formats (Markdown, plain text, etc.)
//!
pub mod cliff;
mod cliff_helpers;
pub mod config;
pub mod types;
