use std::sync::LazyLock;

use color_eyre::eyre::eyre;
use regex::Regex;

use crate::{
    error::{ReleasaurusError, Result},
    orchestrator::core::PRMetadata,
};

pub(crate) static METADATA_REGEX_LEGACY: LazyLock<Regex> =
    LazyLock::new(|| {
        Regex::new(r#"(?ms)^<!--(?<metadata>.*?)-->\n*<details"#).unwrap()
    });

pub(crate) static METADATA_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?ms)^<!--\s*(?<metadata>\{.*?\})\s*-->"#).unwrap()
});

/// Normalizes a package name for use as an HTML `id` attribute.
///
/// Replaces any character that is not ASCII alphanumeric, `-`, or `_`
/// with `-`, ensuring the value is safe for `id=` and compatible with
/// `get_element_by_id` lookups.
pub fn normalize_html_id(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// Extracts the inner HTML of an element with the given `id` from a
/// pre-parsed `VDom`. Returns an empty string if the element is absent
/// or its content is empty.
fn extract_section_from_dom(dom: &tl::VDom, id: &str) -> String {
    let parser = dom.parser();
    let Some(handle) = dom.get_element_by_id(id.as_bytes()) else {
        return String::new();
    };
    let Some(node) = handle.get(parser) else {
        return String::new();
    };
    node.inner_html(parser).trim().to_string()
}

/// Extracts the inner HTML of an element with the given `id` from `body`.
/// Returns an empty string if the element is not found or its content is
/// empty, so callers can treat a missing section as no preserved content.
pub fn extract_preserved_section(body: &str, id: &str) -> String {
    let Ok(dom) = tl::parse(body, tl::ParserOptions::default()) else {
        log::debug!(
            "extract_preserved_section: failed to parse body for id={id}"
        );
        return String::new();
    };
    extract_section_from_dom(&dom, id)
}

pub fn parse_pr_body(
    package_name: &str,
    pr_number: u64,
    body: &str,
) -> Result<(String, String)> {
    let dom = tl::parse(body, tl::ParserOptions::default()).map_err(|e| {
        ReleasaurusError::Other(eyre!(
            "failed to parse merged PR body: pkg={} pr={} - {}",
            package_name,
            pr_number,
            e
        ))
    })?;

    let parser = dom.parser();

    let normalized_id = normalize_html_id(package_name);

    let handle = dom.get_element_by_id(normalized_id.as_bytes()).ok_or(
        ReleasaurusError::Other(eyre!(
            "failed to find details in PR body for package: pkg={} pr={}",
            package_name,
            pr_number,
        )),
    )?;

    let div = handle.get(parser).ok_or(ReleasaurusError::Other(eyre!(
        "failed to create PR parser for package: pkg={} pr={}",
        package_name,
        pr_number,
    )))?;

    let div_tag = div.as_tag().ok_or(ReleasaurusError::Other(eyre!(
        "failed to find details tag matching package: pkg={} pr={}",
        package_name,
        pr_number,
    )))?;

    let pkg_tag = div_tag
            .attributes()
            .get("data-tag")
            .flatten()
            .ok_or(ReleasaurusError::Other(eyre!(
                "failed to find data-tag attribute for package details: pkg={} pr={}",
                package_name,
                pr_number
            )))?.as_utf8_str();

    let notes = div.inner_html(parser);

    let cap =
        METADATA_REGEX
            .captures(&notes)
            .ok_or(ReleasaurusError::Other(eyre!(
                "failed to find metadata for package: pkg={} pr={}",
                package_name,
                pr_number
            )))?;

    let metadata_str = cap
        .name("metadata")
        .ok_or(ReleasaurusError::Other(eyre!(
            "failed to parse metadata from PR body: pkg={} pr={}",
            package_name,
            pr_number,
        )))?
        .as_str();

    log::debug!("parsing metadata string: {:#?}", metadata_str);

    let json: PRMetadata = serde_json::from_str(metadata_str)?;

    let tag_compare_link =
        json.metadata
            .tag_compare_link
            .ok_or(ReleasaurusError::Other(eyre!(
                "failed to find tag_compare_link in PR metadata: pkg={} pr={}",
                package_name,
                pr_number
            )))?;

    let sha_compare_link =
        json.metadata
            .sha_compare_link
            .ok_or(ReleasaurusError::Other(eyre!(
                "failed to find sha_compare_link in PR metadata: pkg={} pr={}",
                package_name,
                pr_number
            )))?;

    let notes = METADATA_REGEX.replace(&notes, "");
    let notes = notes.replace(&sha_compare_link, &tag_compare_link);

    // Collect header and footer preserved sections, if present.
    // Re-use the already-parsed dom to avoid two redundant parses.
    let header =
        extract_section_from_dom(&dom, &format!("{normalized_id}-header"));
    let footer =
        extract_section_from_dom(&dom, &format!("{normalized_id}-footer"));

    let header = header.as_str();
    let footer = footer.as_str();
    let notes = notes.trim();

    let mut release_notes = String::new();

    if !header.is_empty() {
        release_notes.push_str(header);
        release_notes.push('\n');
    }

    release_notes.push_str(notes);

    if !footer.is_empty() {
        release_notes.push('\n');
        release_notes.push_str(footer);
    }

    Ok((pkg_tag.to_string(), release_notes))
}

pub fn parse_legacy_pr_body(
    package_name: &str,
    pr_number: u64,
    body: &str,
) -> Result<Option<(String, String)>> {
    let meta_caps = METADATA_REGEX_LEGACY.captures_iter(body);

    for cap in meta_caps {
        let metadata_str = cap
            .name("metadata")
            .ok_or(ReleasaurusError::Other(eyre!(
                "failed to parse metadata from PR body: pkg={} pr={}",
                package_name,
                pr_number
            )))?
            .as_str();

        log::debug!("parsing legacy metadata string: {:#?}", metadata_str);

        let json: PRMetadata = serde_json::from_str(metadata_str)?;

        if let Some(name) = json.metadata.name.as_deref()
            && name == package_name
        {
            let tag = json.metadata.tag.ok_or(ReleasaurusError::Other(
              eyre!(
                "failed to find tag in legacy metadata: pkg={package_name} pr={pr_number}"
              )
            ))?;
            let notes = json.metadata.notes.ok_or(ReleasaurusError::Other(
              eyre!(
                "failed to find notes in legacy metadata: pkg={package_name} pr={pr_number}"
              )
            ))?;
            return Ok(Some((tag, notes)));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use crate::orchestrator::tests::common::{PrBodyInput, make_pr_body};

    use super::*;

    // normalize_html_id

    #[test]
    fn normalize_html_id_passthrough() {
        assert_eq!(normalize_html_id("my-pkg_v2"), "my-pkg_v2");
    }

    #[test]
    fn normalize_html_id_replaces_slash() {
        assert_eq!(normalize_html_id("@scope/pkg"), "-scope-pkg");
    }

    #[test]
    fn normalize_html_id_replaces_space() {
        assert_eq!(normalize_html_id("my pkg"), "my-pkg");
    }

    #[test]
    fn normalize_html_id_replaces_dot() {
        assert_eq!(normalize_html_id("pkg.name"), "pkg-name");
    }

    #[test]
    fn normalize_html_id_empty_string() {
        assert_eq!(normalize_html_id(""), "");
    }

    // extract_preserved_section

    #[test]
    fn extract_preserved_section_returns_content() {
        let html = r#"<div id="hdr">User text</div>"#;
        assert_eq!(extract_preserved_section(html, "hdr"), "User text");
    }

    #[test]
    fn extract_preserved_section_returns_empty_for_missing_id() {
        let html = r#"<div id="other">content</div>"#;
        assert_eq!(extract_preserved_section(html, "hdr"), "");
    }

    #[test]
    fn extract_preserved_section_returns_empty_for_empty_element() {
        let html = r#"<div id="hdr"></div>"#;
        assert_eq!(extract_preserved_section(html, "hdr"), "");
    }

    #[test]
    fn extract_preserved_section_graceful_on_malformed_html() {
        // Should not panic and return empty string
        let result = extract_preserved_section("<<broken>>>", "hdr");
        assert_eq!(result, "");
    }

    // parse_pr_body

    #[test]
    fn parse_pr_body_happy_path() {
        let body = make_pr_body(&PrBodyInput {
            pkg: "test-pkg",
            tag: "v1.2.3",
            notes: "Release notes",
            tag_link: "tag_link",
            sha_link: "sha-link",
            header: "",
            footer: "",
        });
        let (tag, notes) = parse_pr_body("test-pkg", 1, &body).unwrap();
        assert_eq!(tag, "v1.2.3");
        assert!(notes.contains("Release notes"));
    }

    #[test]
    fn parse_pr_body_strips_metadata_comment() {
        let body = make_pr_body(&PrBodyInput {
            pkg: "test-pkg",
            tag: "v1.2.3",
            notes: "Release notes",
            tag_link: "tag_link",
            sha_link: "sha-link",
            header: "",
            footer: "",
        });
        let (_, notes) = parse_pr_body("test-pkg", 1, &body).unwrap();
        assert!(!notes.contains("<!--"));
        assert!(!notes.contains("sha-link"));
        assert!(!notes.contains("tag-link"));
    }

    #[test]
    fn parse_pr_body_replaces_sha_link_with_tag_link() {
        let sha = "https://example.com/sha1...sha2";
        let tag = "https://example.com/v1.2.2...v1.2.3";
        let notes = format!("Compare: {sha}");
        let body = make_pr_body(&PrBodyInput {
            pkg: "test-pkg",
            tag: "v1.2.3",
            notes: &notes,
            tag_link: tag,
            sha_link: sha,
            header: "",
            footer: "",
        });
        let (_, notes) = parse_pr_body("test-pkg", 1, &body).unwrap();
        assert!(notes.contains(tag));
        assert!(!notes.contains(sha));
    }

    #[test]
    fn parse_pr_body_with_header() {
        let body = make_pr_body(&PrBodyInput {
            pkg: "test-pkg",
            tag: "v1.2.3",
            notes: "Notes",
            tag_link: "tag-link",
            sha_link: "sha-link",
            header: "Header text",
            footer: "",
        });
        let (_, notes) = parse_pr_body("test-pkg", 1, &body).unwrap();
        assert!(notes.starts_with("Header text"));
        assert!(notes.contains("Notes"));
    }

    #[test]
    fn parse_pr_body_with_footer() {
        let body = make_pr_body(&PrBodyInput {
            pkg: "test-pkg",
            tag: "v1.2.3",
            notes: "Notes",
            tag_link: "tag-link",
            sha_link: "sha-link",
            header: "",
            footer: "Footer text",
        });

        let (_, notes) = parse_pr_body("test-pkg", 1, &body).unwrap();
        assert!(notes.ends_with("Footer text"));
        assert!(notes.contains("Notes"));
    }

    #[test]
    fn parse_pr_body_with_header_and_footer() {
        let body = make_pr_body(&PrBodyInput {
            pkg: "test-pkg",
            tag: "v1.2.3",
            notes: "Notes",
            tag_link: "tag-link",
            sha_link: "sha-link",
            header: "Header text",
            footer: "Footer text",
        });

        let (_, notes) = parse_pr_body("test-pkg", 1, &body).unwrap();
        assert!(notes.starts_with("Header text"));
        assert!(notes.ends_with("Footer text"));
        assert!(notes.contains("Notes"));
    }

    #[test]
    fn parse_pr_body_error_missing_div() {
        let body = r#"<details open><summary>v1.0.0</summary></details>"#;
        assert!(parse_pr_body("test-pkg", 1, body).is_err());
    }

    #[test]
    fn parse_pr_body_error_missing_data_tag() {
        let body = r#"<div id="test-pkg">
<!--{"metadata":{"sha_compare_link":"sha","tag_compare_link":"tag"}}-->
notes
</div>"#;
        assert!(parse_pr_body("test-pkg", 1, body).is_err());
    }

    #[test]
    fn parse_pr_body_error_missing_metadata_comment() {
        let body = r#"<div id="test-pkg" data-tag="v1.0.0">
just notes, no metadata comment
</div>"#;
        assert!(parse_pr_body("test-pkg", 1, body).is_err());
    }

    #[test]
    fn parse_pr_body_error_missing_tag_compare_link() {
        let body = r#"<div id="test-pkg" data-tag="v1.0.0">
<!--{"metadata":{"sha_compare_link":"sha"}}-->
notes
</div>"#;
        assert!(parse_pr_body("test-pkg", 1, body).is_err());
    }

    #[test]
    fn parse_pr_body_error_missing_sha_compare_link() {
        let body = r#"<div id="test-pkg" data-tag="v1.0.0">
<!--{"metadata":{"tag_compare_link":"tag"}}-->
notes
</div>"#;
        assert!(parse_pr_body("test-pkg", 1, body).is_err());
    }

    // parse_legacy_pr_body

    #[test]
    fn parse_legacy_pr_body_returns_match() {
        let body = r#"
<!--{"metadata":{"name":"test-pkg","tag":"v1.0.0","notes":"Release notes"}}-->
<details><summary>v1.0.0</summary>
Release notes
</details>"#;
        let result = parse_legacy_pr_body("test-pkg", 1, body).unwrap();
        let (tag, notes) = result.unwrap();
        assert_eq!(tag, "v1.0.0");
        assert_eq!(notes, "Release notes");
    }

    #[test]
    fn parse_legacy_pr_body_returns_none_for_missing_package() {
        let body = r#"
<!--{"metadata":{"name":"other-pkg","tag":"v1.0.0","notes":"Notes"}}-->
<details><summary>v1.0.0</summary>
</details>"#;
        let result = parse_legacy_pr_body("test-pkg", 1, body).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn parse_legacy_pr_body_finds_correct_block_among_multiple() {
        let body = r#"
<!--{"metadata":{"name":"pkg-a","tag":"v1.0.0","notes":"Notes A"}}-->
<details><summary>v1.0.0</summary>
</details>

<!--{"metadata":{"name":"pkg-b","tag":"v2.0.0","notes":"Notes B"}}-->
<details><summary>v2.0.0</summary>
</details>"#;
        let result = parse_legacy_pr_body("pkg-b", 1, body).unwrap();
        let (tag, notes) = result.unwrap();
        assert_eq!(tag, "v2.0.0");
        assert_eq!(notes, "Notes B");
    }

    #[test]
    fn parse_legacy_pr_body_error_missing_tag_field() {
        let body = r#"
<!--{"metadata":{"name":"test-pkg","notes":"Notes"}}-->
<details><summary>v1.0.0</summary>
</details>"#;
        assert!(parse_legacy_pr_body("test-pkg", 1, body).is_err());
    }

    #[test]
    fn parse_legacy_pr_body_error_missing_notes_field() {
        let body = r#"
<!--{"metadata":{"name":"test-pkg","tag":"v1.0.0"}}-->
<details><summary>v1.0.0</summary>
</details>"#;
        assert!(parse_legacy_pr_body("test-pkg", 1, body).is_err());
    }
}
