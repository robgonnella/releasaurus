use std::path::Path;

use crate::updater::{manager::ManifestTarget, traits::ManifestTargets};

pub struct RubyManifests {}

impl ManifestTargets for RubyManifests {
    fn manifest_targets(
        pkg_name: &str,
        _workspace_path: &Path,
        pkg_path: &Path,
    ) -> Vec<ManifestTarget> {
        let pkg_gemspec = format!("{pkg_name}.gemspec");
        let lib_pkg_version = format!("lib/{pkg_name}/version.rb");

        vec![
            ManifestTarget {
                path: pkg_path.join(&pkg_gemspec),
                basename: pkg_gemspec,
            },
            ManifestTarget {
                path: pkg_path.join(&lib_pkg_version),
                basename: "version.rb".into(),
            },
            ManifestTarget {
                path: pkg_path.join("lib/version.rb"),
                basename: "version.rb".into(),
            },
            ManifestTarget {
                path: pkg_path.join("version.rb"),
                basename: "version.rb".into(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn returns_all_ruby_manifest_targets() {
        let pkg_name = "my-gem";
        let workspace_path = Path::new("").to_path_buf();
        let pkg_path = workspace_path.clone();

        let targets = RubyManifests::manifest_targets(
            pkg_name,
            &workspace_path,
            &pkg_path,
        );

        assert_eq!(targets.len(), 4);

        let basenames: Vec<_> = targets.iter().map(|t| &t.basename).collect();
        assert!(basenames.contains(&&"my-gem.gemspec".to_string()));
        assert_eq!(basenames.iter().filter(|b| **b == "version.rb").count(), 3);
    }

    #[test]
    fn generates_correct_paths_for_nested_package() {
        let pkg_name = "my-gem";
        let workspace_path = Path::new("").to_path_buf();
        let pkg_path = Path::new("packages/my-gem").to_path_buf();

        let targets = RubyManifests::manifest_targets(
            pkg_name,
            &workspace_path,
            &pkg_path,
        );

        let paths: Vec<_> =
            targets.iter().map(|t| t.path.to_str().unwrap()).collect();

        assert!(paths.contains(&"packages/my-gem/my-gem.gemspec"));
        assert!(paths.contains(&"packages/my-gem/lib/my-gem/version.rb"));
        assert!(paths.contains(&"packages/my-gem/lib/version.rb"));
        assert!(paths.contains(&"packages/my-gem/version.rb"));
    }
}
