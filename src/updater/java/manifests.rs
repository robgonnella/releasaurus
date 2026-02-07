use std::path::Path;

use crate::updater::{manager::ManifestTarget, traits::ManifestTargets};

pub struct JavaManifests {}

impl ManifestTargets for JavaManifests {
    fn manifest_targets(
        _pkg_name: &str,
        _workspace_path: &Path,
        pkg_path: &Path,
    ) -> Vec<ManifestTarget> {
        vec![
            ManifestTarget {
                path: pkg_path.join("build.gradle"),
                basename: "build.gradle".into(),
            },
            ManifestTarget {
                path: pkg_path.join("lib/build.gradle"),
                basename: "build.gradle".into(),
            },
            ManifestTarget {
                path: pkg_path.join("build.gradle.kts"),
                basename: "build.gradle.kts".into(),
            },
            ManifestTarget {
                path: pkg_path.join("lib/build.gradle.kts"),
                basename: "build.gradle.kts".into(),
            },
            ManifestTarget {
                path: pkg_path.join("gradle.properties"),
                basename: "gradle.properties".into(),
            },
            ManifestTarget {
                path: pkg_path.join("pom.xml"),
                basename: "pom.xml".into(),
            },
            ManifestTarget {
                path: pkg_path.join("gradle/libs.versions.toml"),
                basename: "libs.versions.toml".into(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn returns_all_java_manifest_targets() {
        let workspace_path = Path::new("").to_path_buf();

        let targets = JavaManifests::manifest_targets(
            "tstpkg",
            &workspace_path,
            &workspace_path.clone(),
        );

        assert_eq!(targets.len(), 7);

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
        assert!(basenames.contains(&&"libs.versions.toml".to_string()));
    }

    #[test]
    fn generates_correct_paths_for_nested_package() {
        let workspace_path = Path::new("packages/my-java-app").to_path_buf();
        let targets = JavaManifests::manifest_targets(
            "tstpkg",
            &workspace_path,
            &workspace_path.clone(),
        );

        let paths: Vec<_> =
            targets.iter().map(|t| t.path.to_str().unwrap()).collect();
        assert!(paths.contains(&"packages/my-java-app/build.gradle"));
        assert!(paths.contains(&"packages/my-java-app/lib/build.gradle"));
        assert!(paths.contains(&"packages/my-java-app/build.gradle.kts"));
        assert!(paths.contains(&"packages/my-java-app/lib/build.gradle.kts"));
        assert!(paths.contains(&"packages/my-java-app/gradle.properties"));
        assert!(paths.contains(&"packages/my-java-app/pom.xml"));
        assert!(
            paths.contains(&"packages/my-java-app/gradle/libs.versions.toml")
        );
    }
}
