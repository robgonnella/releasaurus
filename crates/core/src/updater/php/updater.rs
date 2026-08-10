use std::collections::HashMap;
use std::path::Path;

use crate::{
    config::release_type::ReleaseType,
    forge::request::FileChange,
    packages::manifests::ManifestFile,
    result::Result,
    updater::{
        php::{composer_json::ComposerJson, composer_lock::ComposerLock},
        traits::FileUpdater,
    },
};

/// PHP package updater for Composer projects.
pub struct PhpUpdater {
    composer_json: ComposerJson,
    composer_lock: ComposerLock,
}

impl PhpUpdater {
    /// Create PHP updater for Composer composer.json files.
    pub fn new() -> Self {
        Self {
            composer_json: ComposerJson::new(),
            composer_lock: ComposerLock::new(),
        }
    }
}

impl Default for PhpUpdater {
    fn default() -> Self {
        PhpUpdater::new()
    }
}

impl FileUpdater for PhpUpdater {
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if !matches!(manifest.release_type, ReleaseType::Php) {
            return Ok(None);
        }

        if manifest.basename == "composer.json" {
            return self.composer_json.update(manifest);
        }

        // composer.lock is deliberately not handled here. Its content-hash
        // is an md5 over the *updated* composer.json beside it, which a
        // single-file call cannot see, so the lock is written by
        // `update_all`.
        Ok(None)
    }

    fn update_all(&self, files: &[ManifestFile]) -> Result<Vec<FileChange>> {
        let mut changes = vec![];

        // Keyed by the directory the pair shares, which is what makes a
        // lock's sibling unambiguous. Populated for every composer.json
        // present, changed or not, since an unchanged file still describes
        // the hash its lock should carry.
        let mut composer_json_content: HashMap<&Path, String> = HashMap::new();

        for file in files.iter() {
            if file.basename != "composer.json" {
                continue;
            }

            let content = match self.composer_json.update(file)? {
                Some(change) => {
                    let content = change.content.clone();
                    changes.push(change);
                    content
                }
                None => file.content.clone(),
            };

            composer_json_content
                .insert(file.path.parent().unwrap_or(Path::new("")), content);
        }

        for file in files.iter() {
            if file.basename != "composer.lock" {
                continue;
            }

            // No composer.json beside it in the repo, so there is nothing
            // to hash and nothing meaningful to write.
            let Some(json) = composer_json_content
                .get(file.path.parent().unwrap_or(Path::new("")))
            else {
                log::debug!(
                    "no composer.json beside {}: leaving its content-hash alone",
                    file.path.to_string_lossy()
                );
                continue;
            };

            if let Some(change) =
                self.composer_lock.get_lock_change(file, json)?
            {
                changes.push(change);
            }
        }

        Ok(changes)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{
        config::release_type::ReleaseType,
        forge::request::Tag,
        packages::manifests::{ManifestFile, ManifestPackage},
    };

    use super::*;

    #[test]
    fn processes_php_project() {
        let updater = PhpUpdater::new();
        let content = r#"{"name":"vendor/package","version":"1.0.0"}"#;

        let manifest = ManifestFile {
            path: Path::new("composer.json").to_path_buf(),
            basename: "composer.json".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Php,
            owner: Some(ManifestPackage {
                name: "vendor/package".to_string(),
                release_type: ReleaseType::Php,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = updater.update(&manifest).unwrap().unwrap();
        assert!(result.content.contains("2.0.0"));
    }

    #[test]
    fn returns_none_when_no_php_files() {
        let updater = PhpUpdater::new();

        let manifest = ManifestFile {
            path: Path::new("package.json").to_path_buf(),
            basename: "package.json".to_string(),
            content: "{\"version\": \"v1.0.0\"}".into(),
            release_type: ReleaseType::Php,
            owner: Some(ManifestPackage {
                name: "vendor/package".to_string(),
                release_type: ReleaseType::Php,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = updater.update(&manifest).unwrap();
        assert!(result.is_none());
    }

    fn owner(version: &str) -> ManifestPackage {
        ManifestPackage {
            name: "vendor/package".to_string(),
            release_type: ReleaseType::Php,
            tag: Tag {
                name: format!("v{version}"),
                semver: semver::Version::parse(version).unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
        }
    }

    fn manifest(
        path: &str,
        basename: &str,
        content: &str,
        owner: Option<ManifestPackage>,
    ) -> ManifestFile {
        ManifestFile {
            path: Path::new(path).to_path_buf(),
            basename: basename.to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Php,
            owner,
            releasing: vec![],
        }
    }

    fn composer_json(dir: &str, version: &str) -> ManifestFile {
        manifest(
            &format!("{dir}composer.json"),
            "composer.json",
            &format!(r#"{{"name":"vendor/package","version":"{version}"}}"#),
            Some(owner("2.0.0")),
        )
    }

    fn composer_lock(dir: &str) -> ManifestFile {
        manifest(
            &format!("{dir}composer.lock"),
            "composer.lock",
            r#"{"content-hash":"old","packages":[]}"#,
            Some(owner("2.0.0")),
        )
    }

    fn hash_of(change: &FileChange) -> String {
        let doc: serde_json::Value =
            serde_json::from_str(&change.content).expect("lock is json");
        doc["content-hash"].as_str().expect("content-hash").into()
    }

    /// The lock's hash has to describe the composer.json *after* the bump.
    /// Asserting the literal md5 is the point — a hash taken from the
    /// pre-bump JSON is still a well-formed hash, so a weaker assertion
    /// passes while composer refuses the lock as stale.
    #[test]
    fn hashes_the_lock_from_the_bumped_composer_json() {
        let updater = PhpUpdater::new();

        let changes = updater
            .update_all(&[composer_json("", "1.0.0"), composer_lock("")])
            .unwrap();

        assert_eq!(changes.len(), 2);

        let lock = changes
            .iter()
            .find(|c| c.path == "composer.lock")
            .expect("lock change");

        // md5 of {"name":"vendor\/package","version":"2.0.0"}
        assert_eq!(hash_of(lock), "5893e7fda4f3b7ebe83f0f513cc4e077");
    }

    /// A composer.json no releasing package owns is left as-is, so the lock
    /// beside it must be hashed against the content already on the branch
    /// rather than against nothing.
    #[test]
    fn hashes_the_lock_from_the_original_when_the_json_is_unowned() {
        let updater = PhpUpdater::new();

        let mut json = composer_json("", "1.0.0");
        json.owner = None;

        let changes = updater.update_all(&[json, composer_lock("")]).unwrap();

        // Only the lock: the unowned composer.json is untouched.
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "composer.lock");

        // md5 of {"name":"vendor\/package","version":"1.0.0"}
        assert_eq!(hash_of(&changes[0]), "bd52a08dcbbccd30c2476f110883ab70");
    }

    /// Each lock pairs with the composer.json in its own directory.
    #[test]
    fn pairs_each_lock_with_the_json_in_its_own_directory() {
        let updater = PhpUpdater::new();

        let changes = updater
            .update_all(&[
                composer_json("packages/a/", "1.0.0"),
                composer_lock("packages/a/"),
                composer_json("packages/b/", "1.9.0"),
                composer_lock("packages/b/"),
            ])
            .unwrap();

        // Both composer.json files bump to the same owner version, so both
        // locks land on the same hash; what matters is that each resolved a
        // sibling at all rather than erroring or being skipped.
        for dir in ["packages/a", "packages/b"] {
            let lock = changes
                .iter()
                .find(|c| c.path == format!("{dir}/composer.lock"))
                .unwrap_or_else(|| panic!("lock change for {dir}"));

            assert_eq!(hash_of(lock), "5893e7fda4f3b7ebe83f0f513cc4e077");
        }
    }

    /// Nothing to hash against, so the lock is left alone rather than
    /// erroring or being written from an empty document.
    #[test]
    fn skips_a_lock_with_no_composer_json_beside_it() {
        let updater = PhpUpdater::new();

        let changes = updater.update_all(&[composer_lock("")]).unwrap();

        assert!(changes.is_empty());
    }

    /// The single-file entry point cannot see the sibling it would need, so
    /// it declines rather than guessing. `update_all` is the way in.
    #[test]
    fn declines_a_lock_passed_through_the_single_file_entry_point() {
        let updater = PhpUpdater::new();

        assert!(updater.update(&composer_lock("")).unwrap().is_none());
    }
}
