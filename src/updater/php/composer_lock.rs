use std::collections::BTreeMap;

use serde_json::json;

use crate::{
    Result,
    forge::request::{FileChange, FileUpdateType},
    updater::manager::ManifestFile,
};

/// Load and parse json file from repository into serde_json Value.
fn load_doc(content: &str) -> Result<Option<serde_json::Value>> {
    let doc: serde_json::Value = serde_json::from_str(content)?;
    Ok(Some(doc))
}

/// Escape forward slashes in a JSON string to match PHP's default json_encode
/// behavior. PHP escapes `/` as `\/` by default (unless JSON_UNESCAPED_SLASHES
/// is used).
fn escape_forward_slashes(json: &str) -> String {
    json.replace("/", "\\/")
}

// Relevant keys for content hash (same as Composer's Locker.php)
const RELEVANT_KEYS: [&str; 11] = [
    "name",
    "version",
    "require",
    "require-dev",
    "conflict",
    "replace",
    "provide",
    "minimum-stability",
    "prefer-stable",
    "repositories",
    "extra",
];

/// Handles composer.lock file content-hash updates.
pub struct ComposerLock {}

impl ComposerLock {
    /// Create ComposerLock handler for composer.lock version updates.
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_lock_change(
        &self,
        manifest: &ManifestFile,
        new_composer_json: &str,
    ) -> Result<Option<FileChange>> {
        // Use BTreeMap to ensure keys are sorted alphabetically
        // (like PHP's ksort)
        let mut relevant_data: BTreeMap<String, serde_json::Value> =
            BTreeMap::new();

        let json_doc = load_doc(new_composer_json)?;

        if let Some(doc) = json_doc
            && let Some(obj) = doc.as_object()
        {
            for key in RELEVANT_KEYS {
                if let Some(value) = obj.get(key) {
                    // NOTE: Do NOT sort require/require-dev - PHP preserves
                    // original order and only ksort() is applied to the
                    // top-level keys
                    relevant_data.insert(key.to_string(), value.clone());
                }
            }

            // Also include config.platform if present
            // (Composer includes this too)
            if let Some(config) = obj.get("config")
                && let Some(config_obj) = config.as_object()
                && let Some(platform) = config_obj.get("platform")
            {
                let mut config_map = serde_json::Map::new();
                config_map.insert("platform".to_string(), platform.clone());
                relevant_data.insert(
                    "config".to_string(),
                    serde_json::Value::Object(config_map),
                );
            }
        }

        let serialized = serde_json::to_string(&relevant_data)?;

        // PHP's json_encode escapes forward slashes by default
        let serialized = escape_forward_slashes(&serialized);

        // Compute MD5 checksum
        let digest = md5::compute(&serialized);
        let new_content_hash = format!("{:x}", digest);

        let lock_doc = load_doc(&manifest.content)?;

        if let Some(mut doc) = lock_doc {
            doc["content-hash"] = json!(new_content_hash);
            let formatted_json = serde_json::to_string_pretty(&doc)?;
            return Ok(Some(FileChange {
                path: manifest.path.to_string_lossy().into(),
                content: formatted_json,
                update_type: FileUpdateType::Replace,
            }));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn make_manifest(path: &str, content: &str) -> ManifestFile {
        ManifestFile {
            path: PathBuf::from(path),
            basename: "composer.lock".to_string(),
            content: content.to_string(),
        }
    }

    fn make_lock_content(content_hash: &str) -> String {
        format!(
            r#"{{"content-hash":"{}","packages":[],"packages-dev":[]}}"#,
            content_hash
        )
    }

    #[test]
    fn updates_content_hash_in_lock_file() {
        let lock = ComposerLock::new();
        let manifest =
            make_manifest("composer.lock", &make_lock_content("old"));
        let composer_json = r#"{"name":"vendor/pkg","version":"2.0.0"}"#;

        let result = lock.get_lock_change(&manifest, composer_json).unwrap();

        let change = result.unwrap();
        assert!(change.content.contains("content-hash"));
        assert!(!change.content.contains("\"old\""));
    }

    #[test]
    fn computes_deterministic_hash() {
        let lock = ComposerLock::new();
        let manifest = make_manifest("composer.lock", &make_lock_content("x"));
        let composer_json = r#"{"name":"vendor/pkg","version":"1.0.0"}"#;

        let result1 = lock.get_lock_change(&manifest, composer_json).unwrap();
        let result2 = lock.get_lock_change(&manifest, composer_json).unwrap();

        assert_eq!(result1.unwrap().content, result2.unwrap().content);
    }

    #[test]
    fn hash_changes_when_relevant_keys_change() {
        let lock = ComposerLock::new();
        let manifest = make_manifest("composer.lock", &make_lock_content("x"));
        let json_v1 = r#"{"name":"vendor/pkg","version":"1.0.0"}"#;
        let json_v2 = r#"{"name":"vendor/pkg","version":"2.0.0"}"#;

        let result1 = lock.get_lock_change(&manifest, json_v1).unwrap();
        let result2 = lock.get_lock_change(&manifest, json_v2).unwrap();

        assert_ne!(result1.unwrap().content, result2.unwrap().content);
    }

    #[test]
    fn ignores_irrelevant_keys() {
        let lock = ComposerLock::new();
        let manifest = make_manifest("composer.lock", &make_lock_content("x"));
        let json1 = r#"{"name":"vendor/pkg","description":"A"}"#;
        let json2 = r#"{"name":"vendor/pkg","description":"B"}"#;

        let result1 = lock.get_lock_change(&manifest, json1).unwrap();
        let result2 = lock.get_lock_change(&manifest, json2).unwrap();

        // description is not a relevant key, so hash should be the same
        assert_eq!(result1.unwrap().content, result2.unwrap().content);
    }

    #[test]
    fn includes_config_platform_in_hash() {
        let lock = ComposerLock::new();
        let manifest = make_manifest("composer.lock", &make_lock_content("x"));
        let json_no_platform = r#"{"name":"vendor/pkg"}"#;
        let json_with_platform =
            r#"{"name":"vendor/pkg","config":{"platform":{"php":"8.1"}}}"#;

        let result1 =
            lock.get_lock_change(&manifest, json_no_platform).unwrap();
        let result2 =
            lock.get_lock_change(&manifest, json_with_platform).unwrap();

        assert_ne!(result1.unwrap().content, result2.unwrap().content);
    }

    #[test]
    fn escapes_forward_slashes_for_php_compatibility() {
        // PHP's json_encode escapes / as \/ by default
        assert_eq!(escape_forward_slashes("a/b"), r"a\/b");
        assert_eq!(
            escape_forward_slashes("https://example.com"),
            r"https:\/\/example.com"
        );
    }

    #[test]
    fn preserves_existing_lock_file_structure() {
        let lock = ComposerLock::new();
        let lock_content = r#"{
            "content-hash": "old",
            "packages": [{"name": "dep/pkg", "version": "1.0.0"}],
            "packages-dev": []
        }"#;

        let expected_content = r#"{
  "content-hash": "252fbc3a285e5be4bd945c007cbcfc9c",
  "packages": [
    {
      "name": "dep/pkg",
      "version": "1.0.0"
    }
  ],
  "packages-dev": []
}"#;

        let manifest = make_manifest("composer.lock", lock_content);
        let composer_json = r#"{"name":"vendor/pkg"}"#;
        let result = lock.get_lock_change(&manifest, composer_json).unwrap();
        let content = result.unwrap().content;

        assert_eq!(content, expected_content);
    }
}
