//! Version file detection and updating across multiple programming languages.
//!
//! Automatically detects project types and updates version files for Rust,
//! Node.js, Python, Java, PHP, Ruby, and generic projects with trait-based
//! architecture for language-specific implementations.

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
