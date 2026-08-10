//! Acceptance scenarios for workspace-shaped repositories.
//!
//! These are the specification for the file-centric updater model: one
//! path means one file, with exactly one owner or none. Each scenario
//! builds a real repo and runs the real pipeline, so they catch what
//! per-leaf unit tests cannot — a shared workspace lock rewritten once
//! per member and reduced to the last writer's bump.

mod common;
mod config;
mod node;
mod php;
mod rust;
