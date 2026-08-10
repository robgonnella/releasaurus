//! Framework and package management for multi-language support.
use indexmap::IndexMap;
use regex::Regex;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::packages::manifests::ManifestPackage;
use crate::{
    config::release_type::ReleaseType,
    forge::{request::FileChange, traits::FileLoader},
    packages::{
        manifests::{AdditionalManifestFile, ManifestFile},
        releasable::ReleasablePackage,
    },
    result::Result,
    updater::{dispatch::Updater, generic::updater::GenericUpdater},
};

/// A single manifest file path to load from the forge.
#[derive(Debug, Clone, Serialize)]
pub struct ManifestTarget {
    /// The file path relative to the package path
    pub path: PathBuf,
    /// The base name of the file path
    pub basename: String,
}

/// A deduped manifest target, tagged with the release type of the
/// package that claimed it. Carried on the target because a workspace
/// level file may end up with no owner to read it from.
struct ResolvedTarget {
    path: PathBuf,
    basename: String,
    release_type: ReleaseType,
}

/// A deduped additional manifest target. Unlike a language manifest,
/// these are named explicitly in one package's config, so the owner is
/// never in question.
struct AdditionalTarget {
    path: PathBuf,
    basename: String,
    version_regex: Regex,
    owner: ManifestPackage,
}

/// A releasing package paired with the directory that decides which
/// manifests it owns.
struct ReleasingPackage {
    path: PathBuf,
    manifest: ManifestPackage,
}

/// Programming language and package manager detection for determining which
/// version files to update.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct UpdateManager {}

impl UpdateManager {
    /// Generate all file changes needed to bump versions for a
    /// package.
    ///
    /// Handles the primary manifest, user-configured additional
    /// manifests, and sub-packages (for workspace-style repos).
    pub async fn get_file_changes_for_packages<F: FileLoader>(
        packages: &[&ReleasablePackage],
        file_loader: &F,
        branch: &str,
    ) -> Result<Vec<FileChange>> {
        let mut file_changes = vec![];

        let (manifest_targets, additional_targets) =
            Self::get_all_targets(packages);

        let mut content_map = Self::load_manifests(
            &manifest_targets,
            &additional_targets,
            file_loader,
            branch,
        )
        .await?;

        let releasing = Self::releasing_by_release_type(packages);

        // Grouped by release type so each updater sees every manifest it
        // owns in one pass. Most updaters treat the files independently, but
        // some cannot: PHP's composer.lock carries an md5 of the *updated*
        // composer.json beside it. An `IndexMap` keeps the emitted changes
        // in target order rather than release-type hash order.
        let mut by_release_type: IndexMap<ReleaseType, Vec<ManifestFile>> =
            IndexMap::new();

        for target in manifest_targets.iter() {
            // Removed rather than cloned: targets are deduped by path, so
            // each entry is consumed exactly once, and a lock file's
            // content can run to megabytes.
            let Some(content) = content_map.remove(&target.path) else {
                continue;
            };

            let peers = releasing
                .get(&target.release_type)
                .map(|p| p.as_slice())
                .unwrap_or_default();

            by_release_type
                .entry(target.release_type)
                .or_default()
                .push(ManifestFile {
                    path: target.path.clone(),
                    basename: target.basename.clone(),
                    content,
                    release_type: target.release_type,
                    owner: Self::owner_of(&target.path, peers),
                    releasing: peers
                        .iter()
                        .map(|p| p.manifest.clone())
                        .collect(),
                });
        }

        for (release_type, manifests) in by_release_type.iter() {
            let updater = Updater::new(*release_type);
            file_changes.extend(updater.update_all(manifests)?);
        }

        for target in additional_targets.iter() {
            // `get_all_targets` drops any additional target whose path is
            // also a manifest path, so the loop above cannot have taken
            // this entry.
            let Some(content) = content_map.remove(&target.path) else {
                continue;
            };

            let additional = AdditionalManifestFile {
                path: target.path.clone(),
                basename: target.basename.clone(),
                content,
                owner: target.owner.clone(),
                version_regex: target.version_regex.clone(),
            };

            if let Some(change) = GenericUpdater::update_manifest(
                &(&additional).into(),
                &additional.version_regex,
            ) {
                file_changes.push(change);
            }
        }

        Ok(file_changes)
    }

    ////////////////////////////////////////////////////////////////////////////
    // private
    ////////////////////////////////////////////////////////////////////////////

    /// Every package and sub-package being released, keyed by release
    /// type. A manifest only ever consults entries of its own type: a
    /// Rust `pkg-b` must not bump a Node `pkg-b`.
    fn releasing_by_release_type(
        packages: &[&ReleasablePackage],
    ) -> HashMap<ReleaseType, Vec<ReleasingPackage>> {
        let mut by_type: HashMap<ReleaseType, Vec<ReleasingPackage>> =
            HashMap::new();

        for pkg in packages.iter() {
            by_type.entry(pkg.release_type).or_default().push(
                ReleasingPackage {
                    path: pkg.path.clone(),
                    manifest: ManifestPackage {
                        name: pkg.name.clone(),
                        tag: pkg.tag.clone(),
                        release_type: pkg.release_type,
                    },
                },
            );

            // A sub-package has no tag of its own; it rides the
            // parent's.
            for sub in pkg.sub_packages.iter() {
                by_type.entry(sub.release_type).or_default().push(
                    ReleasingPackage {
                        path: sub.path.clone(),
                        manifest: ManifestPackage {
                            name: sub.name.clone(),
                            tag: pkg.tag.clone(),
                            release_type: sub.release_type,
                        },
                    },
                );
            }
        }

        by_type
    }

    /// The package whose directory holds `path`, if any is being
    /// released.
    ///
    /// `None` means no releasing package's own version fields live in
    /// this file — a virtual workspace manifest, or a root lock file in
    /// a workspace whose root is not part of this release.
    fn owner_of(
        file_path: &Path,
        releasing: &[ReleasingPackage],
    ) -> Option<ManifestPackage> {
        let dir = file_path.parent()?;

        releasing
            .iter()
            .find(|p| p.path == dir)
            .map(|p| p.manifest.clone())
    }

    /// Unions every package's, sub-package's, and additional manifest
    /// target, deduped by path.
    ///
    /// Deduping here rather than after loading is what makes one path
    /// mean one file: a workspace lock sits in every member's target
    /// list, and rewriting it once per member from the same base content
    /// leaves only the last member's bump.
    ///
    /// The two lists dedupe separately and are reconciled at the end, so
    /// a path claimed by both always resolves to the language manifest.
    /// Deduping them against one shared set instead would hand the path
    /// to whichever package happened to be visited first, which can drop
    /// a real manifest and leave that package unbumped.
    fn get_all_targets(
        packages: &[&ReleasablePackage],
    ) -> (Vec<ResolvedTarget>, Vec<AdditionalTarget>) {
        let mut all_manifest_targets = vec![];
        let mut all_additional_manifest_targets = vec![];

        let mut manifest_paths = HashSet::new();
        let mut additional_paths = HashSet::new();

        for pkg in packages {
            let owned = pkg
                .manifest_targets
                .iter()
                .map(|t| (t, pkg.release_type))
                .chain(pkg.sub_packages.iter().flat_map(|sub| {
                    sub.manifest_targets.iter().map(|t| (t, sub.release_type))
                }));

            for (target, release_type) in owned {
                if !manifest_paths.insert(target.path.clone()) {
                    continue;
                }

                all_manifest_targets.push(ResolvedTarget {
                    path: target.path.clone(),
                    basename: target.basename.clone(),
                    release_type,
                });
            }

            for (target, version_regex) in
                pkg.additional_manifest_targets.iter()
            {
                if !additional_paths.insert(target.path.clone()) {
                    continue;
                }

                all_additional_manifest_targets.push(AdditionalTarget {
                    path: target.path.clone(),
                    basename: target.basename.clone(),
                    version_regex: version_regex.clone(),
                    owner: ManifestPackage {
                        name: pkg.name.clone(),
                        tag: pkg.tag.clone(),
                        release_type: pkg.release_type,
                    },
                });
            }
        }

        all_additional_manifest_targets.retain(|target| {
            let claimed = manifest_paths.contains(&target.path);

            if claimed {
                log::warn!(
                    "additional manifest is already handled by its \
                     language updater: ignoring its version_regex: {}",
                    target.path.to_string_lossy()
                );
            }

            !claimed
        });

        (all_manifest_targets, all_additional_manifest_targets)
    }

    /// Fetches every deduped target once, keyed by path.
    async fn load_manifests<F: FileLoader>(
        manifest_targets: &[ResolvedTarget],
        additional_targets: &[AdditionalTarget],
        file_loader: &F,
        base_branch: &str,
    ) -> Result<HashMap<PathBuf, String>> {
        let mut manifests = HashMap::new();

        let load_manifest = async |path: &Path| -> Result<Option<String>> {
            log::debug!("Loading manifest target: {}", path.to_string_lossy());

            if let Some(content) = file_loader
                .load_file(
                    Some(base_branch.into()),
                    path.to_string_lossy().to_string(),
                )
                .await?
            {
                log::info!("Loaded manifest: {}", path.to_string_lossy());
                Ok(Some(content))
            } else {
                log::debug!("Manifest not found: {}", path.to_string_lossy());
                Ok(None)
            }
        };

        for target in manifest_targets.iter() {
            if let Some(content) = load_manifest(&target.path).await? {
                manifests.insert(target.path.clone(), content);
            }
        }

        for target in additional_targets.iter() {
            if let Some(content) = load_manifest(&target.path).await? {
                manifests.insert(target.path.clone(), content);
            }
        }

        Ok(manifests)
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use std::path::Path;
    use std::sync::Mutex;

    use crate::{
        config::package::GENERIC_VERSION_REGEX,
        forge::request::Tag,
        packages::releasable::ReleasableSubPackage,
        updater::{
            php::manifests::PhpManifests, rust::manifests::RustManifests,
            traits::ManifestTargets,
        },
    };

    use super::*;

    /// Serves a fixed set of files and records every path asked for, so
    /// tests can assert a shared manifest is fetched exactly once.
    struct StubLoader {
        files: HashMap<String, String>,
        loads: Mutex<Vec<String>>,
    }

    impl StubLoader {
        fn new(files: &[(&str, &str)]) -> Self {
            Self {
                files: files
                    .iter()
                    .map(|(p, c)| (p.to_string(), c.to_string()))
                    .collect(),
                loads: Mutex::new(vec![]),
            }
        }

        fn load_count(&self, path: &str) -> usize {
            self.loads
                .lock()
                .unwrap()
                .iter()
                .filter(|p| p.as_str() == path)
                .count()
        }
    }

    #[async_trait]
    impl FileLoader for StubLoader {
        async fn load_file(
            &self,
            _branch: Option<String>,
            path: String,
        ) -> Result<Option<String>> {
            self.loads.lock().unwrap().push(path.clone());
            Ok(self.files.get(&path).cloned())
        }
    }

    fn tag(version: &str) -> Tag {
        Tag {
            name: format!("v{version}"),
            semver: semver::Version::parse(version).unwrap(),
            sha: "abc".into(),
            ..Tag::default()
        }
    }

    /// Targets come from the real `RustManifests` so the tests stay
    /// honest about which paths a package actually claims.
    fn rust_package(
        name: &str,
        version: &str,
        pkg_path: &str,
    ) -> ReleasablePackage {
        ReleasablePackage {
            name: name.into(),
            path: Path::new(pkg_path).to_path_buf(),
            release_type: ReleaseType::Rust,
            tag: tag(version),
            manifest_targets: RustManifests::manifest_targets(
                name,
                Path::new(""),
                Path::new(pkg_path),
            ),
            ..Default::default()
        }
    }

    fn rust_sub_package(name: &str, pkg_path: &str) -> ReleasableSubPackage {
        ReleasableSubPackage {
            name: name.into(),
            path: Path::new(pkg_path).to_path_buf(),
            release_type: ReleaseType::Rust,
            manifest_targets: RustManifests::manifest_targets(
                name,
                Path::new(""),
                Path::new(pkg_path),
            ),
        }
    }

    fn php_package(
        name: &str,
        version: &str,
        pkg_path: &str,
    ) -> ReleasablePackage {
        ReleasablePackage {
            name: name.into(),
            path: Path::new(pkg_path).to_path_buf(),
            release_type: ReleaseType::Php,
            tag: tag(version),
            manifest_targets: PhpManifests::manifest_targets(
                name,
                Path::new(""),
                Path::new(pkg_path),
            ),
            ..Default::default()
        }
    }

    fn count(changes: &[FileChange], path: &str) -> usize {
        changes.iter().filter(|c| c.path == path).count()
    }

    fn content_hash(changes: &[FileChange], path: &str) -> String {
        let change = changes
            .iter()
            .find(|c| c.path == path)
            .unwrap_or_else(|| panic!("no change for {path}"));

        let doc: serde_json::Value =
            serde_json::from_str(&change.content).expect("lock is json");

        doc["content-hash"].as_str().expect("content-hash").into()
    }

    /// `composer.lock`'s `content-hash` is an md5 over the relevant keys of
    /// the *updated* `composer.json`, so the lock cannot be derived from its
    /// own content alone. Asserting the concrete hash is the point: a hash
    /// computed from the pre-bump JSON is still a well-formed hash.
    #[tokio::test]
    async fn a_composer_lock_is_hashed_from_the_updated_composer_json() {
        let loader = StubLoader::new(&[
            (
                "composer.json",
                r#"{"name":"vendor/pkg","version":"1.0.0"}"#,
            ),
            ("composer.lock", r#"{"content-hash":"old","packages":[]}"#),
        ]);

        let pkg = php_package("vendor/pkg", "2.0.0", "");

        let changes = UpdateManager::get_file_changes_for_packages(
            &[&pkg],
            &loader,
            "main",
        )
        .await
        .unwrap();

        assert_eq!(count(&changes, "composer.json"), 1);
        assert_eq!(count(&changes, "composer.lock"), 1);

        // md5 of {"name":"vendor\/pkg","version":"2.0.0"} — the bumped
        // version, not 1.0.0.
        assert_eq!(
            content_hash(&changes, "composer.lock"),
            "0f45ae3becee5f46f6f7479c3f1eb426"
        );
    }

    /// Each lock pairs with the `composer.json` beside it. Keying the pair
    /// by anything but the shared directory silently crosses them.
    #[tokio::test]
    async fn each_composer_lock_pairs_with_its_own_sibling() {
        let loader = StubLoader::new(&[
            (
                "packages/a/composer.json",
                r#"{"name":"vendor/pkg","version":"1.0.0"}"#,
            ),
            (
                "packages/a/composer.lock",
                r#"{"content-hash":"old-a","packages":[]}"#,
            ),
            (
                "packages/b/composer.json",
                r#"{"name":"vendor/other","version":"1.0.0"}"#,
            ),
            (
                "packages/b/composer.lock",
                r#"{"content-hash":"old-b","packages":[]}"#,
            ),
        ]);

        let a = php_package("vendor/pkg", "2.0.0", "packages/a");
        let b = php_package("vendor/other", "3.0.0", "packages/b");

        let changes = UpdateManager::get_file_changes_for_packages(
            &[&a, &b],
            &loader,
            "main",
        )
        .await
        .unwrap();

        assert_eq!(count(&changes, "packages/a/composer.lock"), 1);
        assert_eq!(count(&changes, "packages/b/composer.lock"), 1);

        // a's hash covers vendor/pkg@2.0.0; b's covers vendor/other@3.0.0.
        // Crossed pairing would give them each other's value.
        assert_eq!(
            content_hash(&changes, "packages/a/composer.lock"),
            "0f45ae3becee5f46f6f7479c3f1eb426"
        );
        assert_ne!(
            content_hash(&changes, "packages/a/composer.lock"),
            content_hash(&changes, "packages/b/composer.lock"),
        );
    }

    /// A lock with no `composer.json` beside it in the repo has nothing to
    /// hash, and must not error.
    #[tokio::test]
    async fn a_lock_without_a_composer_json_is_left_alone() {
        let loader = StubLoader::new(&[(
            "composer.lock",
            r#"{"content-hash":"old","packages":[]}"#,
        )]);

        let pkg = php_package("vendor/pkg", "2.0.0", "");

        let changes = UpdateManager::get_file_changes_for_packages(
            &[&pkg],
            &loader,
            "main",
        )
        .await
        .unwrap();

        assert!(changes.is_empty());
    }

    const ROOT_LOCK: &str = r#"version = 3

[[package]]
name = "sub-a"
version = "1.0.0"

[[package]]
name = "sub-b"
version = "1.0.0"
"#;

    /// The workspace root's `Cargo.toml` and `Cargo.lock` sit in the
    /// parent's target list and in both sub-packages'. Each must still
    /// produce exactly one change, carrying every member's bump.
    #[tokio::test]
    async fn emits_one_change_per_path_for_a_parent_with_sub_packages() {
        let loader = StubLoader::new(&[
            (
                "Cargo.toml",
                "[workspace]\nmembers = [\"crates/*\"]\n\n\
                 [workspace.dependencies]\nsub-a = \"1.0.0\"\n",
            ),
            ("Cargo.lock", ROOT_LOCK),
            (
                "crates/a/Cargo.toml",
                "[package]\nname = \"sub-a\"\nversion = \"1.0.0\"\n",
            ),
            (
                "crates/b/Cargo.toml",
                "[package]\nname = \"sub-b\"\nversion = \"1.0.0\"\n",
            ),
        ]);

        let mut parent = rust_package("workspace", "2.0.0", "");
        parent.sub_packages = vec![
            rust_sub_package("sub-a", "crates/a"),
            rust_sub_package("sub-b", "crates/b"),
        ];

        let changes = UpdateManager::get_file_changes_for_packages(
            &[&parent],
            &loader,
            "main",
        )
        .await
        .unwrap();

        assert_eq!(count(&changes, "Cargo.toml"), 1);
        assert_eq!(count(&changes, "Cargo.lock"), 1);
        assert_eq!(count(&changes, "crates/a/Cargo.toml"), 1);
        assert_eq!(count(&changes, "crates/b/Cargo.toml"), 1);

        // crates/{a,b}/Cargo.lock are claimed as targets but absent from
        // the repo, so they contribute nothing.
        assert_eq!(changes.len(), 4);

        assert_eq!(loader.load_count("Cargo.toml"), 1);
        assert_eq!(loader.load_count("Cargo.lock"), 1);
    }

    /// One lock, one write, both members bumped — the shared lock must
    /// not be recomputed per member from the same base and overwritten.
    #[tokio::test]
    async fn the_shared_lock_carries_every_member_bump() {
        let loader = StubLoader::new(&[
            ("Cargo.toml", "[workspace]\n"),
            ("Cargo.lock", ROOT_LOCK),
        ]);

        let mut parent = rust_package("workspace", "2.0.0", "");
        parent.sub_packages = vec![
            rust_sub_package("sub-a", "crates/a"),
            rust_sub_package("sub-b", "crates/b"),
        ];

        let changes = UpdateManager::get_file_changes_for_packages(
            &[&parent],
            &loader,
            "main",
        )
        .await
        .unwrap();

        let lock = changes
            .iter()
            .find(|c| c.path == "Cargo.lock")
            .expect("lock change");

        assert_eq!(lock.content.matches("version = \"2.0.0\"").count(), 2);
    }

    /// Two top-level packages in one workspace both claim the root
    /// files. Dedup has to span packages, not just a single package's
    /// own target list.
    #[tokio::test]
    async fn emits_one_change_per_path_for_sibling_top_level_packages() {
        let loader = StubLoader::new(&[
            ("Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n"),
            (
                "Cargo.lock",
                "version = 3\n\n[[package]]\nname = \"pkg-a\"\nversion = \"1.0.0\"\n\n\
                 [[package]]\nname = \"pkg-b\"\nversion = \"1.0.0\"\n",
            ),
            (
                "crates/a/Cargo.toml",
                "[package]\nname = \"pkg-a\"\nversion = \"1.0.0\"\n",
            ),
            (
                "crates/b/Cargo.toml",
                "[package]\nname = \"pkg-b\"\nversion = \"1.0.0\"\n",
            ),
        ]);

        let a = rust_package("pkg-a", "2.0.0", "crates/a");
        let b = rust_package("pkg-b", "3.0.0", "crates/b");

        let changes = UpdateManager::get_file_changes_for_packages(
            &[&a, &b],
            &loader,
            "main",
        )
        .await
        .unwrap();

        assert_eq!(count(&changes, "Cargo.lock"), 1);
        assert_eq!(count(&changes, "crates/a/Cargo.toml"), 1);
        assert_eq!(count(&changes, "crates/b/Cargo.toml"), 1);
        assert_eq!(loader.load_count("Cargo.lock"), 1);

        let lock = changes
            .iter()
            .find(|c| c.path == "Cargo.lock")
            .expect("lock change");

        assert!(lock.content.contains("version = \"2.0.0\""));
        assert!(lock.content.contains("version = \"3.0.0\""));
    }

    /// A virtual manifest has no `[package]` to bump, so the root
    /// `Cargo.toml` yields no change rather than a fabricated one.
    #[tokio::test]
    async fn a_virtual_manifest_produces_no_root_package_change() {
        let loader = StubLoader::new(&[
            ("Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n"),
            (
                "crates/a/Cargo.toml",
                "[package]\nname = \"pkg-a\"\nversion = \"1.0.0\"\n",
            ),
        ]);

        let pkg = rust_package("pkg-a", "2.0.0", "crates/a");

        let changes = UpdateManager::get_file_changes_for_packages(
            &[&pkg],
            &loader,
            "main",
        )
        .await
        .unwrap();

        assert_eq!(count(&changes, "Cargo.toml"), 0);
        assert_eq!(count(&changes, "crates/a/Cargo.toml"), 1);
    }

    /// A sub-package's peers include its siblings, not just its parent.
    /// `releasaurus.toml` is exactly this shape, and
    /// `crates/cli/Cargo.toml` depends on `crates/core`.
    #[tokio::test]
    async fn a_sub_package_sees_its_siblings_as_peers() {
        let loader = StubLoader::new(&[
            ("Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n"),
            (
                "crates/a/Cargo.toml",
                "[package]\nname = \"sub-a\"\nversion = \"1.0.0\"\n\n\
                 [dependencies]\n\
                 sub-b = { path = \"../b\", version = \"1.0.0\" }\n",
            ),
            (
                "crates/b/Cargo.toml",
                "[package]\nname = \"sub-b\"\nversion = \"1.0.0\"\n",
            ),
        ]);

        let mut parent = rust_package("workspace", "2.0.0", "");
        parent.sub_packages = vec![
            rust_sub_package("sub-a", "crates/a"),
            rust_sub_package("sub-b", "crates/b"),
        ];

        let changes = UpdateManager::get_file_changes_for_packages(
            &[&parent],
            &loader,
            "main",
        )
        .await
        .unwrap();

        let sub_a = changes
            .iter()
            .find(|c| c.path == "crates/a/Cargo.toml")
            .expect("sub-a change");

        // Its own version, inherited from the parent's tag.
        assert!(sub_a.content.contains("version = \"2.0.0\""));

        // Its sibling's, which a parent-only peer list would miss. The
        // path must survive the rewrite.
        let dep_line = sub_a
            .content
            .lines()
            .find(|l| l.starts_with("sub-b ="))
            .expect("sub-b dependency line");

        assert!(dep_line.contains("version = \"2.0.0\""), "{dep_line}");
        assert!(dep_line.contains("path = \"../b\""), "{dep_line}");
    }

    /// Two packages listing the same additional manifest share one file,
    /// so it is fetched once and rewritten once.
    #[tokio::test]
    async fn a_shared_additional_manifest_yields_one_change() {
        let loader = StubLoader::new(&[("VERSION", "version = \"1.0.0\"\n")]);

        let target = ManifestTarget {
            path: Path::new("VERSION").to_path_buf(),
            basename: "VERSION".into(),
        };

        let mut a = rust_package("pkg-a", "2.0.0", "crates/a");
        a.additional_manifest_targets =
            vec![(target.clone(), GENERIC_VERSION_REGEX.clone())];

        let mut b = rust_package("pkg-b", "3.0.0", "crates/b");
        b.additional_manifest_targets =
            vec![(target, GENERIC_VERSION_REGEX.clone())];

        let changes = UpdateManager::get_file_changes_for_packages(
            &[&a, &b],
            &loader,
            "main",
        )
        .await
        .unwrap();

        assert_eq!(count(&changes, "VERSION"), 1);
        assert_eq!(loader.load_count("VERSION"), 1);
    }
}
