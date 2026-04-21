use crate::config::{Config, resolved::GlobalOverrides};

pub fn resolve_base_branch(
    config: &Config,
    global_overrides: &GlobalOverrides,
    repo_default_branch: &str,
) -> String {
    global_overrides
        .base_branch
        .clone()
        .or_else(|| config.base_branch.clone())
        .unwrap_or_else(|| repo_default_branch.to_string())
}
