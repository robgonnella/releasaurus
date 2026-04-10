//! Tests for new-format PR metadata parsing.
//!
//! Tests for:
//! - New metadata regex pattern matching
//! - JSON extraction from new-format HTML comments
//! - Regex behaviour within a div (embedded context)

use crate::orchestrator::{core::PRMetadata, pr_body::METADATA_REGEX};

#[test]
fn metadata_regex_matches_new_format_comment() {
    let body = r#"<div id="test-pkg" data-tag="v1.2.3">
<!--{"metadata":{"sha_compare_link":"sha-url","tag_compare_link":"tag-url"}}-->

Release notes
</div>"#;

    assert!(METADATA_REGEX.captures(body).is_some());
}

#[test]
fn metadata_regex_extracts_json_from_comment() {
    let body = r#"<div id="test-pkg" data-tag="v1.2.3">
<!--{"metadata":{"sha_compare_link":"sha-url","tag_compare_link":"tag-url"}}-->

Release notes
</div>"#;

    let caps = METADATA_REGEX.captures(body).unwrap();
    let metadata_str = caps.name("metadata").unwrap().as_str();
    let parsed: PRMetadata = serde_json::from_str(metadata_str).unwrap();
    assert_eq!(parsed.metadata.sha_compare_link.as_deref(), Some("sha-url"));
    assert_eq!(parsed.metadata.tag_compare_link.as_deref(), Some("tag-url"));
}

#[test]
fn metadata_regex_handles_whitespace_around_json() {
    let body = r#"<div id="test-pkg" data-tag="v1.0.0">
<!--  {"metadata":{"sha_compare_link":"s","tag_compare_link":"t"}}  -->
</div>"#;

    assert!(METADATA_REGEX.captures(body).is_some());
}

#[test]
fn metadata_regex_does_not_match_plain_comment() {
    let body = r#"<div id="test-pkg" data-tag="v1.0.0">
<!-- This is just a regular comment -->
</div>"#;

    assert!(METADATA_REGEX.captures(body).is_none());
}
