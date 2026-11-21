//! Short release notes for a release PR body that would exceed the
//! forge's limit with the full notes.
//!
//! The short notes are generated from the release's commits, not from
//! the rendered notes: one heading per group and one line per commit, in
//! the layout of the default changelog template minus links, authors,
//! commit bodies, and breaking-change descriptions.

use regex::Regex;
use std::sync::LazyLock;

use crate::analyzer::commit::Commit;

/// What Tera's `striptags` filter removes, as the default template
/// applies it to group names to drop their `<!-- NN -->` sort tag.
static TAG_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(<!--.*?-->|<[^>]*>)").unwrap());

/// One line of short release notes.
pub(super) struct ShortLine {
    pub(super) text: String,
    /// Whether the line is a group heading rather than a commit.
    pub(super) heading: bool,
}

/// Short release notes for `commits`: merge commits left out, the rest
/// grouped by [`Commit::group`] in sort order, as the default changelog
/// template groups them.
pub(super) fn short_notes(commits: &[Commit]) -> Vec<ShortLine> {
    let mut commits = commits
        .iter()
        .filter(|commit| !commit.merge_commit)
        .collect::<Vec<_>>();
    // stable, so commits keep their order within a group
    commits.sort_by(|a, b| a.group.cmp(&b.group));

    let mut lines = Vec::new();
    let mut group = None;
    for commit in commits {
        if group != Some(&commit.group) {
            group = Some(&commit.group);
            lines.push(ShortLine {
                text: format!(
                    "### {}",
                    TAG_REGEX.replace_all(&commit.group, "").trim()
                ),
                heading: true,
            });
        }
        lines.push(ShortLine {
            text: commit_line(commit),
            heading: false,
        });
    }
    lines
}

/// The line for one commit: scope, breaking marker, title, and short SHA.
fn commit_line(commit: &Commit) -> String {
    let scope = commit
        .scope
        .as_ref()
        .map(|scope| format!("_({scope})_ "))
        .unwrap_or_default();
    let breaking = if commit.breaking {
        "[**breaking**]: "
    } else {
        ""
    };
    format!(
        "- {scope}{breaking}{} _({})_",
        commit.title, commit.short_id
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(group: &str, title: &str, short_id: &str) -> Commit {
        Commit {
            group: group.into(),
            title: title.into(),
            short_id: short_id.into(),
            link: format!("https://example.com/commit/{short_id}"),
            body: Some("a body that the short notes leave out".into()),
            ..Default::default()
        }
    }

    fn texts(commits: &[Commit]) -> Vec<String> {
        short_notes(commits)
            .into_iter()
            .map(|line| line.text)
            .collect()
    }

    #[test]
    fn groups_commits_in_sort_order_without_sort_tags() {
        let commits = [
            commit("<!-- 02 -->Bug Fixes", "correct a typo", "abc0"),
            commit("<!-- 01 -->Features", "add a flag", "abc1"),
            commit("<!-- 02 -->Bug Fixes", "handle empty input", "abc2"),
        ];

        assert_eq!(
            texts(&commits),
            [
                "### Features",
                "- add a flag _(abc1)_",
                "### Bug Fixes",
                "- correct a typo _(abc0)_",
                "- handle empty input _(abc2)_",
            ]
        );
    }

    #[test]
    fn marks_headings() {
        let commits = [commit("Features", "add a flag", "abc1")];

        let headings = short_notes(&commits)
            .into_iter()
            .map(|line| line.heading)
            .collect::<Vec<_>>();

        assert_eq!(headings, [true, false]);
    }

    #[test]
    fn leaves_out_merge_commits_and_their_only_group() {
        let commits = [
            Commit {
                merge_commit: true,
                ..commit("<!-- 00 -->Merges", "merge branch main", "abc0")
            },
            commit("<!-- 01 -->Features", "add a flag", "abc1"),
        ];

        assert_eq!(texts(&commits), ["### Features", "- add a flag _(abc1)_"]);
    }

    #[test]
    fn writes_scope_and_breaking_marker() {
        let commits = [
            Commit {
                scope: Some("api".into()),
                breaking: true,
                breaking_description: Some("migrate to v2".into()),
                ..commit("Breaking Changes", "drop the v1 endpoints", "brk1")
            },
            Commit {
                scope: Some("cli".into()),
                ..commit("Breaking Changes", "rename a flag", "abc2")
            },
        ];

        assert_eq!(
            texts(&commits),
            [
                "### Breaking Changes",
                "- _(api)_ [**breaking**]: drop the v1 endpoints _(brk1)_",
                "- _(cli)_ rename a flag _(abc2)_",
            ]
        );
    }

    #[test]
    fn keeps_titles_as_written() {
        let commits = [commit(
            "Features",
            "support `Vec<String>` and Option<T>",
            "abc1",
        )];

        assert_eq!(
            texts(&commits),
            [
                "### Features",
                "- support `Vec<String>` and Option<T> _(abc1)_",
            ]
        );
    }

    #[test]
    fn no_commits_give_no_lines() {
        assert!(short_notes(&[]).is_empty());
    }
}
