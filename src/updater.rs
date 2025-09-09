//! Version file updating and management across multiple programming languages.
//!
//! This module provides the core functionality for detecting and updating version
//! information across different programming languages and frameworks. It serves as
//! the engine that keeps version numbers synchronized during release processes,
//! ensuring consistency across all project files that declare version information.
//!
//! # Key Responsibilities
//!
//! - **Language Detection**: Automatically identify project types and frameworks
//! - **Version File Discovery**: Locate all files that contain version information
//! - **Version Updates**: Update version numbers using language-specific conventions
//! - **Validation**: Ensure version updates are applied correctly and consistently
//! - **Multi-Language Support**: Handle projects with multiple language components
//!
//! # Architecture
//!
//! The updater system is built on a trait-based architecture that allows for
//! language-specific implementations while maintaining a consistent interface.
//! The system consists of several key components:
//!
//! ## Core Components
//!
//! - **Manager**: Central coordinator that orchestrates version updates
//! - **Detection**: Language and framework detection algorithms
//! - **Framework**: Base infrastructure and common functionality
//! - **Traits**: Common interfaces that all updaters must implement
//! - **Language Updaters**: Specific implementations for each supported language
//! - **Generic Updater**: Fallback for unsupported or custom version patterns
//!
//! # Supported Languages & Frameworks
//!
//! ## Rust
//! - `Cargo.toml` version field
//! - Workspace dependency versions
//! - `Cargo.lock` consistency
//!
//! ## Node.js/JavaScript
//! - `package.json` version field
//! - `package-lock.json` synchronization
//! - Yarn and npm workspace support
//! - `lerna.json` for monorepos
//!
//! ## Python
//! - `pyproject.toml` (PEP 621 standard)
//! - `setup.py` version arguments
//! - `setup.cfg` metadata
//! - `requirements.txt` and `requirements-*.txt` files
//! - Poetry and setuptools configurations
//!
//! ## Java
//! - Maven `pom.xml` version elements
//! - Gradle `build.gradle` and `build.gradle.kts`
//! - Multi-module project support
//! - Parent/child project relationships
//!
//! ## PHP
//! - `composer.json` version field
//! - `composer.lock` consistency
//! - Custom version constants in source files
//!
//! ## Ruby
//! - `Gemfile` version specifications
//! - `*.gemspec` version declarations
//! - `VERSION` files
//! - Bundler workspace support
//!
//! ## Generic
//! - Custom version file patterns
//! - Regex-based version matching
//! - User-defined version file locations
//! - Plain text version files
//!
//! # Update Process
//!
//! The version update process follows a consistent workflow:
//!
//! 1. **Detection**: Analyze the project structure to identify languages and frameworks
//! 2. **Discovery**: Locate all version-bearing files for detected languages
//! 3. **Validation**: Verify current version information and consistency
//! 4. **Update**: Apply the new version using language-specific conventions
//! 5. **Verification**: Confirm updates were applied correctly
//! 6. **Reporting**: Provide detailed information about changes made
//!
//! # Version Conventions
//!
//! The updater respects language-specific version conventions and formats:
//!
//! - **Semantic Versioning**: Standard `MAJOR.MINOR.PATCH` format
//! - **Pre-release Identifiers**: Alpha, beta, RC suffixes
//! - **Build Metadata**: Build numbers and commit information
//! - **Language-Specific**: Platform conventions (e.g., Python PEP 440)
//!
//! # Error Handling
//!
//! The updater system provides comprehensive error handling:
//!
//! - **Graceful Degradation**: Continue with supported languages if some fail
//! - **Rollback Capability**: Restore previous versions on critical failures
//! - **Detailed Reporting**: Clear error messages with context and suggestions
//! - **Validation Warnings**: Non-fatal issues that may need attention
//!
//! # Usage Examples
//!
//! ## Basic Version Update
//!
//! ```rust,ignore
//! use crate::updater::manager::UpdateManager;
//!
//! let manager = UpdateManager::new(project_path)?;
//! let results = manager.update_version("1.2.3").await?;
//!
//! for result in results {
//!     println!("Updated {}: {} -> {}", result.file, result.old_version, result.new_version);
//! }
//! ```
//!
//! ## Language-Specific Updates
//!
//! ```rust,ignore
//! use crate::updater::rust::RustUpdater;
//! use crate::updater::traits::VersionUpdater;
//!
//! let updater = RustUpdater::new();
//! let files = updater.discover_version_files(project_path)?;
//! updater.update_files(&files, "1.2.3")?;
//! ```
//!
//! # Extensibility
//!
//! The system is designed for easy extension to support new languages or frameworks:
//!
//! 1. Implement the `VersionUpdater` trait
//! 2. Add detection logic for the new language
//! 3. Register the updater with the manager
//! 4. Add comprehensive tests
//!
//! # Configuration
//!
//! Version updating behavior can be customized through:
//!
//! - Project-specific configuration files
//! - Command-line options for override behavior
//! - Environment variables for default settings
//! - Language-specific configuration within project files

mod detection;
mod framework;
mod generic;
mod java;
pub mod manager;
mod node;
mod php;
mod python;
mod ruby;
mod rust;
mod traits;
