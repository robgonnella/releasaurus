//! Loads the available commands for this cli
pub mod common;
pub mod release;
pub mod release_pr;

#[cfg(test)]
#[cfg(feature = "_internal_e2e_tests")]
mod tests;
