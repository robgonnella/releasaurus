//! Manifest file compilation and validation.
//!
//! Handles the compilation of additional manifest file specifications
//! into validated, ready-to-use manifest descriptors with compiled
//! regex patterns.

use regex::Regex;
use std::path::{Path, PathBuf};

use crate::{ReleasaurusError, Result, config::package::PackageConfig};

use super::super::path_utils::normalize_path;

/// Compiled version of AdditionalManifest with pre-compiled regex.
///
/// This is populated during config resolution to avoid repeated
/// regex compilation during manifest processing.
#[derive(Debug, Clone)]
pub struct CompiledAdditionalManifest {
    /// The path to the manifest file relative to package path
    pub path: PathBuf,
    /// The compiled regex to use to match and replace versions
    pub version_regex: Regex,
}

/// Compiles additional manifest specifications into validated
/// manifests.
///
/// This function:
/// 1. Extracts manifest specs from package config
/// 2. Compiles regex patterns
/// 3. Validates that patterns have required 'version' capture group
/// 4. Normalizes paths relative to package
///
/// # Errors
///
/// Returns an error if:
/// - Regex pattern is invalid
/// - Regex pattern is missing 'version' capture group
/// - Version regex is unexpectedly None after conversion
pub fn compile_additional_manifests(
    normalized_full_package_path: &Path,
    package: &PackageConfig,
) -> Result<Vec<CompiledAdditionalManifest>> {
    let Some(manifest_specs) = package.additional_manifest_files.clone() else {
        return Ok(vec![]);
    };

    let mut compiled = Vec::with_capacity(manifest_specs.len());

    for spec in manifest_specs {
        let manifest = spec.into_manifest();

        let compiled_manifest = compile_single_manifest(
            normalized_full_package_path,
            manifest.path,
            manifest.version_regex,
        )?;

        compiled.push(compiled_manifest);
    }

    Ok(compiled)
}

/// Compiles a single manifest specification.
fn compile_single_manifest(
    base_path: &Path,
    manifest_path: String,
    version_regex: Option<String>,
) -> Result<CompiledAdditionalManifest> {
    let pattern = version_regex.ok_or_else(|| {
        ReleasaurusError::invalid_config(format!(
            "Missing version_regex for additional_manifest_files \
             entry '{}'. This should not happen after spec \
             conversion.",
            manifest_path
        ))
    })?;

    let version_regex = compile_and_validate_regex(&manifest_path, &pattern)?;

    let full_manifest_path =
        base_path.join(&manifest_path).to_string_lossy().to_string();

    let normalized_manifest_path = normalize_path(&full_manifest_path);
    let normalized_manifest_path_buf =
        Path::new(normalized_manifest_path.as_ref()).to_path_buf();

    Ok(CompiledAdditionalManifest {
        path: normalized_manifest_path_buf,
        version_regex,
    })
}

/// Compiles a regex pattern and validates it has a 'version'
/// capture group.
fn compile_and_validate_regex(
    manifest_path: &str,
    pattern: &str,
) -> Result<Regex> {
    let regex = Regex::new(pattern).map_err(|e| {
        ReleasaurusError::invalid_config(format!(
            "Invalid regex pattern in additional_manifest_files \
             for '{}': {}",
            manifest_path, e
        ))
    })?;

    // Validate that the regex has a 'version' capture group
    let has_version_group =
        regex.capture_names().any(|name| name == Some("version"));

    if !has_version_group {
        return Err(ReleasaurusError::invalid_config(format!(
            "Regex pattern for '{}' must include a named capture \
             group '(?<version>...)' to identify the version \
             number to replace",
            manifest_path
        )));
    }

    Ok(regex)
}

#[cfg(test)]
mod tests {
    use super::*;

    // compile_and_validate_regex tests

    #[test]
    fn validates_version_capture_group_required() {
        // Missing named group
        let result =
            compile_and_validate_regex("test.txt", r"version: (\d+\.\d+\.\d+)");
        assert!(result.is_err());
    }

    #[test]
    fn validates_version_capture_group_present() {
        // Has named group
        let regex = compile_and_validate_regex(
            "test.txt",
            r"version: (?<version>\d+\.\d+\.\d+)",
        )
        .unwrap();

        // Verify it actually matches and captures
        let caps = regex.captures(r#"version: 1.2.3"#).unwrap();
        assert_eq!(&caps["version"], "1.2.3");
    }

    #[test]
    fn rejects_invalid_regex() {
        let result = compile_and_validate_regex("test.txt", r"[invalid(");
        assert!(result.is_err());
    }
}
