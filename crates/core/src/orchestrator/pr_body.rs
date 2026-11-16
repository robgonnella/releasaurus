//! The release PR body: one collapsible section per package.
//!
//! Each section is a `<details>` element holding three divs, keyed by the
//! package's [normalized](normalize_html_id) name:
//! - `{id}-header` and `{id}-footer`: text maintainers may edit, kept
//!   across updates of the PR and published with the release notes;
//! - `{id}`: the release notes, with the release tag in `data-tag` and
//!   the PR metadata either inline as an HTML comment or in a `data-meta`
//!   attribute, depending on the forge.
//!
//! [`render`] writes the body, falling back to [`short`] notes when the
//! full ones exceed the forge's limit; [`parse`] reads it back once the PR
//! is merged.

mod parse;
mod render;
mod short;

#[cfg(test)]
pub(crate) use parse::METADATA_REGEX;
pub(crate) use parse::{
    extract_preserved_header_footer, parse_pr_body, pr_body_has_package,
};
#[cfg(test)]
pub(crate) use render::CONDENSED_NOTES_MARKER;
pub(crate) use render::{PackageSection, render_release_pr_body};

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

#[cfg(test)]
mod tests {
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
}
