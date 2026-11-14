//! Node.js package updater supporting npm, yarn, and pnpm projects.

use serde::Serialize;
use serde_json::{Serializer, Value, ser::PrettyFormatter};

use crate::{forge::request::TagVersion, result::Result};

pub mod manifests;
pub mod package_json;
pub mod package_lock;
pub mod updater;
pub mod yarn_lock;

/// Rewrites a dependency spec on a released package to `version`, keeping a
/// `^` or `~` prefix. Returns `None` for anything else (`workspace:`, `*`,
/// `>=1 <2`, ...), which is left as written.
pub fn bumped_dependency_spec(
    spec: &str,
    version: &TagVersion,
) -> Option<String> {
    let (prefix, current) = match spec.strip_prefix(['^', '~']) {
        Some(rest) => (&spec[..1], rest),
        None => ("", spec),
    };
    semver::Version::parse(current).ok()?;
    Some(format!("{prefix}{version}"))
}

/// Serializes `doc` with the original file's indentation and trailing
/// newline, defaulting to two spaces.
pub fn format_like(doc: &Value, original: &str) -> Result<String> {
    let indent = original
        .lines()
        .skip(1)
        .map(|line| {
            let rest = line.trim_start_matches([' ', '\t']);
            &line[..line.len() - rest.len()]
        })
        .find(|ws| !ws.is_empty())
        .unwrap_or("  ");

    let mut buf = Vec::new();
    let formatter = PrettyFormatter::with_indent(indent.as_bytes());
    let mut serializer = Serializer::with_formatter(&mut buf, formatter);
    doc.serialize(&mut serializer)?;

    let mut formatted = String::from_utf8(buf)?;
    if original.ends_with('\n') {
        formatted.push('\n');
    }
    Ok(formatted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bumped_dependency_spec_keeps_prefix() {
        let version =
            TagVersion::Semantic(semver::Version::parse("2.0.0").unwrap());

        let cases = [
            // supported prefixes: kept, version replaced
            ("^1.0.0", Some("^2.0.0")),
            ("~1.0.0", Some("~2.0.0")),
            ("1.0.0", Some("2.0.0")),
            ("^1.0.0-rc.1", Some("^2.0.0")),
            // anything else: left as written
            ("workspace:*", None),
            ("repo:^1.0.0", None),
            ("*", None),
            ("1.x", None),
            (">=1.0.0 <2.0.0", None),
            ("file:../b", None),
            ("npm:other@1.0.0", None),
        ];

        for (spec, expected) in cases {
            assert_eq!(
                bumped_dependency_spec(spec, &version).as_deref(),
                expected,
                "{spec}"
            );
        }
    }
}
