use crate::{
    config::package::PackageConfig,
    path_helpers::package_path,
    updater::{manager::ManifestTarget, traits::ManifestTargets},
};

pub struct RubyManifests {}

impl ManifestTargets for RubyManifests {
    fn manifest_targets(pkg: &PackageConfig) -> Vec<ManifestTarget> {
        let pkg_gemspec = format!("{}.gemspec", pkg.name);
        let lib_pkg_version = format!("lib/{}/version.rb", pkg.name);

        vec![
            ManifestTarget {
                path: package_path(pkg, Some(&pkg_gemspec)),
                basename: pkg_gemspec,
            },
            ManifestTarget {
                path: package_path(pkg, Some(&lib_pkg_version)),
                basename: "version.rb".into(),
            },
            ManifestTarget {
                path: package_path(pkg, Some("lib/version.rb")),
                basename: "version.rb".into(),
            },
            ManifestTarget {
                path: package_path(pkg, Some("version.rb")),
                basename: "version.rb".into(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{package::PackageConfig, release_type::ReleaseType};

    fn create_test_package(name: &str, path: &str) -> PackageConfig {
        PackageConfig {
            name: name.to_string(),
            workspace_root: ".".to_string(),
            path: path.to_string(),
            release_type: Some(ReleaseType::Ruby),
            ..Default::default()
        }
    }

    #[test]
    fn returns_all_ruby_manifest_targets() {
        let pkg = create_test_package("my-gem", ".");
        let targets = RubyManifests::manifest_targets(&pkg);

        assert_eq!(targets.len(), 4);

        let basenames: Vec<_> = targets.iter().map(|t| &t.basename).collect();
        assert!(basenames.contains(&&"my-gem.gemspec".to_string()));
        assert_eq!(basenames.iter().filter(|b| **b == "version.rb").count(), 3);
    }

    #[test]
    fn generates_correct_paths_for_nested_package() {
        let pkg = create_test_package("my-gem", "packages/my-gem");
        let targets = RubyManifests::manifest_targets(&pkg);

        let paths: Vec<_> = targets.iter().map(|t| t.path.as_str()).collect();
        assert!(paths.contains(&"packages/my-gem/my-gem.gemspec"));
        assert!(paths.contains(&"packages/my-gem/lib/my-gem/version.rb"));
        assert!(paths.contains(&"packages/my-gem/lib/version.rb"));
        assert!(paths.contains(&"packages/my-gem/version.rb"));
    }
}
