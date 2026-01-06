use crate::{
    config::package::PackageConfig,
    path_helpers::package_path,
    updater::{manager::ManifestTarget, traits::ManifestTargets},
};

pub struct JavaManifests {}

impl ManifestTargets for JavaManifests {
    fn manifest_targets(pkg: &PackageConfig) -> Vec<ManifestTarget> {
        vec![
            ManifestTarget {
                path: package_path(pkg, Some("build.gradle")),
                basename: "build.gradle".into(),
            },
            ManifestTarget {
                path: package_path(pkg, Some("lib/build.gradle")),
                basename: "build.gradle".into(),
            },
            ManifestTarget {
                path: package_path(pkg, Some("build.gradle.kts")),
                basename: "build.gradle.kts".into(),
            },
            ManifestTarget {
                path: package_path(pkg, Some("lib/build.gradle.kts")),
                basename: "build.gradle.kts".into(),
            },
            ManifestTarget {
                path: package_path(pkg, Some("gradle.properties")),
                basename: "gradle.properties".into(),
            },
            ManifestTarget {
                path: package_path(pkg, Some("pom.xml")),
                basename: "pom.xml".into(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{package::PackageConfig, release_type::ReleaseType};

    fn create_test_package(path: &str) -> PackageConfig {
        PackageConfig {
            name: "test-package".to_string(),
            workspace_root: ".".to_string(),
            path: path.to_string(),
            release_type: Some(ReleaseType::Java),
            ..Default::default()
        }
    }

    #[test]
    fn returns_all_java_manifest_targets() {
        let pkg = create_test_package(".");
        let targets = JavaManifests::manifest_targets(&pkg);

        assert_eq!(targets.len(), 6);

        let basenames: Vec<_> = targets.iter().map(|t| &t.basename).collect();
        assert_eq!(
            basenames.iter().filter(|b| **b == "build.gradle").count(),
            2
        );
        assert_eq!(
            basenames
                .iter()
                .filter(|b| **b == "build.gradle.kts")
                .count(),
            2
        );
        assert!(basenames.contains(&&"gradle.properties".to_string()));
        assert!(basenames.contains(&&"pom.xml".to_string()));
    }

    #[test]
    fn generates_correct_paths_for_nested_package() {
        let pkg = create_test_package("packages/my-java-app");
        let targets = JavaManifests::manifest_targets(&pkg);

        let paths: Vec<_> = targets.iter().map(|t| t.path.as_str()).collect();
        assert!(paths.contains(&"packages/my-java-app/build.gradle"));
        assert!(paths.contains(&"packages/my-java-app/lib/build.gradle"));
        assert!(paths.contains(&"packages/my-java-app/build.gradle.kts"));
        assert!(paths.contains(&"packages/my-java-app/lib/build.gradle.kts"));
        assert!(paths.contains(&"packages/my-java-app/gradle.properties"));
        assert!(paths.contains(&"packages/my-java-app/pom.xml"));
    }
}
