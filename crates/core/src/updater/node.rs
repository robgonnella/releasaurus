//! Node.js package updater supporting npm, yarn, and pnpm projects.

use crate::forge::request::TagVersion;

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
