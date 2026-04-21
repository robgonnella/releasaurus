use crate::config::package::PackageConfig;

pub fn create_test_package(name: &str) -> PackageConfig {
    PackageConfig {
        name: name.to_string(),
        workspace_root: ".".to_string(),
        path: ".".to_string(),
        ..Default::default()
    }
}
