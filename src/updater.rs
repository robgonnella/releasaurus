//! Version file detection and updating across multiple programming languages.
//!
//! Automatically detects project types and updates version files for Rust,
//! Node.js, Python, Java, PHP, Ruby, and generic projects with trait-based
//! architecture for language-specific implementations.

pub mod framework;
pub mod generic;
mod java;
mod node;
mod php;
mod python;
mod ruby;
mod rust;
mod traits;
