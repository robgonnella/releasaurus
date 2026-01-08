//! Tests for PR metadata parsing.
//!
//! Tests for:
//! - Metadata regex pattern matching
//! - JSON parsing from HTML comments
//! - Multiple metadata blocks handling

use super::super::{METADATA_REGEX, PRMetadata};

#[test]
fn metadata_regex_matches_json_in_html_comment() {
    let pr_body = r#"
<!--{"metadata":{"name":"pkg","tag":"v1.0.0","notes":"notes"}}-->
<details><summary>v1.0.0</summary>
notes
</details>
"#;

    let caps = METADATA_REGEX.captures(pr_body);
    assert!(caps.is_some());

    let metadata_str = caps.unwrap().name("metadata").unwrap().as_str();
    let parsed: PRMetadata = serde_json::from_str(metadata_str).unwrap();
    assert_eq!(parsed.metadata.name, "pkg");
    assert_eq!(parsed.metadata.tag, "v1.0.0");
}

#[test]
fn metadata_regex_handles_multiple_metadata_blocks() {
    let pr_body = r#"
<!--{"metadata":{"name":"pkg-a","tag":"v1.0.0","notes":"notes a"}}-->
<details><summary>v1.0.0</summary>
notes a
</details>

<!--{"metadata":{"name":"pkg-b","tag":"v2.0.0","notes":"notes b"}}-->
<details><summary>v2.0.0</summary>
notes b
</details>
"#;

    let matches: Vec<_> = METADATA_REGEX.captures_iter(pr_body).collect();
    assert_eq!(matches.len(), 2);
}

#[test]
fn metadata_regex_extracts_correct_metadata_from_multiple_blocks() {
    let pr_body = r#"
<!--{"metadata":{"name":"first-pkg","tag":"v1.0.0","notes":"first notes"}}-->
<details><summary>v1.0.0</summary>
first notes
</details>

<!--{"metadata":{"name":"second-pkg","tag":"v2.5.0","notes":"second notes"}}-->
<details><summary>v2.5.0</summary>
second notes
</details>
"#;

    let matches: Vec<_> = METADATA_REGEX.captures_iter(pr_body).collect();

    // Verify first metadata block
    let first_metadata_str = matches[0].name("metadata").unwrap().as_str();
    let first: PRMetadata = serde_json::from_str(first_metadata_str).unwrap();
    assert_eq!(first.metadata.name, "first-pkg");
    assert_eq!(first.metadata.tag, "v1.0.0");
    assert_eq!(first.metadata.notes, "first notes");

    // Verify second metadata block
    let second_metadata_str = matches[1].name("metadata").unwrap().as_str();
    let second: PRMetadata = serde_json::from_str(second_metadata_str).unwrap();
    assert_eq!(second.metadata.name, "second-pkg");
    assert_eq!(second.metadata.tag, "v2.5.0");
    assert_eq!(second.metadata.notes, "second notes");
}

#[test]
fn metadata_regex_matches_but_json_parsing_would_fail_for_non_json_comments() {
    let pr_body = r#"
<!-- This is just a regular comment -->
<details><summary>v1.0.0</summary>
Some content
</details>
"#;

    let caps = METADATA_REGEX.captures(pr_body);
    assert!(caps.is_some());

    // The regex matches, but JSON parsing would fail
    let metadata_str = caps.unwrap().name("metadata").unwrap().as_str();
    let parsed: Result<PRMetadata, _> = serde_json::from_str(metadata_str);
    assert!(parsed.is_err());
}

#[test]
fn metadata_regex_requires_details_tag_after_comment() {
    // Metadata comment without <details> tag should not match
    let pr_body = r#"
<!--{"metadata":{"name":"pkg","tag":"v1.0.0","notes":"notes"}}-->
Some other content
"#;

    let caps = METADATA_REGEX.captures(pr_body);
    assert!(caps.is_none());
}
