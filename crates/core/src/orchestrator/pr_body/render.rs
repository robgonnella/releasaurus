//! Renders a release PR body and fits it within a forge's description
//! limit.
//!
//! A body within the limit keeps the release notes exactly as rendered.
//! A longer one falls back to [short notes](super::short) generated from
//! each package's commits, dropping lines from the bottom only as far as
//! needed to fit.

use crate::{
    analyzer::commit::Commit,
    forge::request::PrMetadataBlock,
    result::{ReleasaurusError, Result},
};

use super::{
    normalize_html_id,
    short::{ShortLine, short_notes},
};

/// Follows the short release notes in place of the full ones.
pub(crate) const CONDENSED_NOTES_MARKER: &str = "_Release notes condensed to \
     fit the pull request description limit; see the changelog for the full \
     release notes._";

/// Separates package sections in the body.
const SECTION_SEPARATOR: &str = "\n";

/// One package's section of a release PR body.
pub(crate) struct PackageSection<'a> {
    /// The package name as configured; its [normalized](normalize_html_id)
    /// form keys the section's divs.
    pub name: &'a str,
    pub tag_name: &'a str,
    /// Renders the `<details>` element expanded.
    pub open: bool,
    /// PR metadata as the forge encodes it: inside the notes div, such as
    /// an HTML comment, or as an attribute of it, such as `data-meta`.
    pub metadata: PrMetadataBlock,
    /// Preserved content of the header div, edited by maintainers.
    pub header: String,
    /// Preserved content of the footer div, edited by maintainers.
    pub footer: String,
    /// Release notes as rendered for the changelog, in Markdown.
    pub notes: String,
    /// Commits in the release, for the short notes used when the full
    /// notes do not fit.
    pub commits: &'a [Commit],
}

impl PackageSection<'_> {
    /// The text around this section's notes.
    fn wrapper(&self) -> Wrapper {
        Wrapper {
            before: self.before_notes(),
            after: self.after_notes(),
        }
    }

    /// Text before the notes, the header included.
    fn before_notes(&self) -> String {
        let details = if self.open {
            "<details open>"
        } else {
            "<details>"
        };
        let div_attribute = if self.metadata.div_attribute.is_empty() {
            String::new()
        } else {
            format!(" {}", self.metadata.div_attribute)
        };
        let inline = if self.metadata.inline_content.is_empty() {
            "\n".to_string()
        } else {
            format!("{}\n\n", self.metadata.inline_content)
        };
        format!(
            "{details}\n<summary>{tag}</summary>\n\
             <div id=\"{id}-header\">{header}</div>\n\
             <div id=\"{id}\" data-tag=\"{tag}\"{div_attribute}>\n{inline}",
            tag = self.tag_name,
            id = normalize_html_id(self.name),
            header = self.header,
        )
    }

    /// Text after the notes, the footer included.
    fn after_notes(&self) -> String {
        format!(
            "\n</div>\n<div id=\"{}-footer\">{}</div>\n</details>",
            normalize_html_id(self.name),
            self.footer
        )
    }
}

/// The text before and after one package's notes in the body.
struct Wrapper {
    before: String,
    after: String,
}

/// Renders a release PR body from `sections`, fitted within `limit` UTF-16
/// code units when there is one.
///
/// A body over the limit gets every package's short notes in place of
/// the full ones: a heading per commit group and a line per commit,
/// followed by [`CONDENSED_NOTES_MARKER`], with lines dropped from the
/// bottom only as far as needed to fit. Packages with few commits keep
/// all their lines, long ones are cut to a fair share. Headers and footers
/// are always kept as they are.
///
/// # Errors
///
/// Returns a forge error when the body exceeds the limit even with no
/// release notes lines; for a PR with several packages, it suggests
/// `separate_pull_requests`.
pub(crate) fn render_release_pr_body(
    sections: &[PackageSection],
    limit: Option<usize>,
) -> Result<String> {
    let wrappers = sections
        .iter()
        .map(PackageSection::wrapper)
        .collect::<Vec<_>>();
    let full = assemble(
        &wrappers,
        sections.iter().map(|section| section.notes.as_str()),
    );
    let Some(limit) = limit else {
        return Ok(full);
    };
    let full_len = utf16_len(&full);
    if full_len <= limit {
        return Ok(full);
    }

    let condensed = sections
        .iter()
        .map(|section| CondensedNotes::new(section.commits))
        .collect::<Vec<_>>();

    // what the body takes with every package's notes reduced to the marker
    let fixed = wrappers
        .iter()
        .map(|wrapper| utf16_len(&wrapper.before) + utf16_len(&wrapper.after))
        .sum::<usize>()
        + utf16_len(SECTION_SEPARATOR) * wrappers.len().saturating_sub(1);
    let minimal = fixed
        + condensed
            .iter()
            .map(|notes| notes.lengths[0])
            .sum::<usize>();
    if minimal > limit {
        return Err(too_large_error(sections, minimal, limit));
    }

    let kept = allocate(&condensed, limit - fixed);
    let body = render_condensed(&wrappers, &condensed, &kept);
    warn_condensed(sections, &condensed, &kept, full_len, limit, &body);
    Ok(body)
}

/// The error for a release PR that exceeds `limit` even with no release
/// notes: `minimal` is its length then.
fn too_large_error(
    sections: &[PackageSection],
    minimal: usize,
    limit: usize,
) -> ReleasaurusError {
    let subject = match sections {
        [section] => format!("release PR for package {}", section.name),
        _ => format!("release PR for {} packages", sections.len()),
    };
    let hint = if sections.len() > 1 {
        "; set `separate_pull_requests = true` under `[repository]` to \
         open a separate release PR for each package"
    } else {
        ""
    };
    ReleasaurusError::forge(format!(
        "{subject} cannot fit the forge's PR description limit of {limit} \
         UTF-16 code units: it needs {minimal} even with the release notes \
         left out{hint}"
    ))
}

/// Length of `text` in UTF-16 code units, as Azure DevOps counts it: a
/// character outside the Basic Multilingual Plane, such as an emoji,
/// counts twice.
fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// A package's short release notes, with their length for every number
/// of lines kept.
struct CondensedNotes {
    lines: Vec<ShortLine>,
    /// `lengths[k]`: UTF-16 length of the condensed notes keeping the
    /// first `k` lines, marker included.
    lengths: Vec<usize>,
}

impl CondensedNotes {
    fn new(commits: &[Commit]) -> Self {
        let lines = short_notes(commits);
        let marker = utf16_len(CONDENSED_NOTES_MARKER);
        let mut lengths = Vec::with_capacity(lines.len() + 1);
        lengths.push(marker);
        let mut kept = 0;
        for line in &lines {
            // each line is followed by "\n", and the last one by "\n\n"
            // before the marker
            kept += utf16_len(&line.text) + 1;
            lengths.push(kept + 1 + marker);
        }
        Self { lines, lengths }
    }

    /// Whether the notes may end after the first `k` lines: never right
    /// after a heading, which would be left without entries.
    fn can_end_at(&self, k: usize) -> bool {
        k == 0 || !self.lines[k - 1].heading
    }

    /// The most lines that fit within `cap`, or none.
    fn keep_within(&self, cap: usize) -> usize {
        // lengths grow with every line kept
        let fitting = self.lengths.partition_point(|len| *len <= cap);
        (1..fitting)
            .rev()
            .find(|&k| self.can_end_at(k))
            .unwrap_or(0)
    }

    fn text(&self, kept: usize) -> String {
        let lines = self.lines[..kept]
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>();
        if lines.is_empty() {
            CONDENSED_NOTES_MARKER.to_string()
        } else {
            format!("{}\n\n{CONDENSED_NOTES_MARKER}", lines.join("\n"))
        }
    }
}

/// Picks how many condensed lines each package keeps so the notes together
/// fit `budget`, which must hold every package's notes with no lines kept.
///
/// Finds by bisection the largest per-package cap on notes length whose
/// picks fit: packages with condensed notes under the cap keep all their
/// lines, longer ones are cut to it.
fn allocate(condensed: &[CondensedNotes], budget: usize) -> Vec<usize> {
    let picks_for = |cap: usize| -> (Vec<usize>, usize) {
        let picks = condensed
            .iter()
            .map(|notes| notes.keep_within(cap))
            .collect::<Vec<_>>();
        let total = condensed
            .iter()
            .zip(&picks)
            .map(|(notes, kept)| notes.lengths[*kept])
            .sum();
        (picks, total)
    };

    let (mut picks, _) = picks_for(0);
    let (mut low, mut high) = (0, budget);
    while low < high {
        let mid = (low + high).div_ceil(2);
        let (mid_picks, total) = picks_for(mid);
        if total <= budget {
            low = mid;
            picks = mid_picks;
        } else {
            high = mid - 1;
        }
    }
    picks
}

fn render_condensed(
    wrappers: &[Wrapper],
    condensed: &[CondensedNotes],
    kept: &[usize],
) -> String {
    assemble(
        wrappers,
        condensed
            .iter()
            .zip(kept)
            .map(|(notes, kept)| notes.text(*kept)),
    )
}

/// Joins the package sections into a body, each with its notes from
/// `notes` inside its wrapper.
fn assemble(
    wrappers: &[Wrapper],
    notes: impl Iterator<Item = impl AsRef<str>>,
) -> String {
    let mut body = String::new();
    for (index, (wrapper, notes)) in wrappers.iter().zip(notes).enumerate() {
        if index > 0 {
            body.push_str(SECTION_SEPARATOR);
        }
        body.push_str(&wrapper.before);
        body.push_str(notes.as_ref());
        body.push_str(&wrapper.after);
    }
    body
}

fn warn_condensed(
    sections: &[PackageSection],
    condensed: &[CondensedNotes],
    kept: &[usize],
    full_len: usize,
    limit: usize,
    body: &str,
) {
    let trimmed = sections
        .iter()
        .zip(condensed.iter().zip(kept))
        .filter(|(_, (notes, kept))| **kept < notes.lines.len())
        .map(|(section, _)| section.name)
        .collect::<Vec<_>>();
    let trimmed = if trimmed.is_empty() {
        String::new()
    } else {
        format!(", with lines dropped for packages: {}", trimmed.join(", "))
    };
    log::warn!(
        "release PR body is {full_len} UTF-16 code units, over the forge \
         limit of {limit}: release notes shortened to one line per commit \
         to fit in {} UTF-16 code units{trimmed}; the changelog keeps the \
         full notes",
        utf16_len(body)
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::pr_body::parse_pr_body;

    const LIMIT: usize = 4000;

    /// A commit link as the default changelog template renders it on Azure
    /// DevOps.
    fn link(package: &str, index: usize) -> String {
        format!(
            "https://dev.azure.com/org/project/_git/repo/commit/\
             {package}{index:034}"
        )
    }

    fn commit(group: &str, title: String, short_id: String) -> Commit {
        Commit {
            group: group.into(),
            title,
            short_id,
            ..Default::default()
        }
    }

    /// A breaking change and `count` fixes.
    fn commits(package: &str, count: usize) -> Vec<Commit> {
        let breaking = Commit {
            breaking: true,
            body: Some(format!(
                "The v1 endpoints are gone; migrate with:\n```sh\n\
                 {package} migrate --to v2\n```"
            )),
            ..commit(
                "<!-- 00 -->Breaking Changes",
                format!("drop the v1 endpoints of {package}"),
                format!("{package}brk"),
            )
        };
        let fixes = (0..count).map(|index| {
            commit(
                "<!-- 02 -->Bug Fixes",
                format!(
                    "correct a fairly long commit subject number {index} in \
                     {package}"
                ),
                format!("{package}{index}"),
            )
        });
        std::iter::once(breaking).chain(fixes).collect()
    }

    /// Full notes for `commits` in the shape the default changelog
    /// template renders: links to every commit and quoted commit bodies.
    fn full_notes(package: &str, commits: &[Commit]) -> String {
        let mut notes = format!(
            "# [1.0.1](https://example.com/{package}/compare) - 2026-11-13\n"
        );
        let mut group = None;
        for (index, commit) in commits.iter().enumerate() {
            if group != Some(&commit.group) {
                group = Some(&commit.group);
                notes.push_str(&format!("\n### {}\n\n", commit.group));
            }
            notes.push_str(&format!(
                "- {} [_({})_]({})\n",
                commit.title,
                commit.short_id,
                link(package, index)
            ));
            for line in commit.body.iter().flat_map(|body| body.lines()) {
                notes.push_str(&format!("> {line}\n"));
            }
        }
        notes
    }

    fn metadata(package: &str) -> String {
        format!(
            "<!--{{\"metadata\":{{\"sha_compare_link\":\"{package}-sha\",\
             \"tag_compare_link\":\"{package}-tag\"}}}}-->"
        )
    }

    struct Fixture {
        package: String,
        tag: String,
        metadata: String,
        header: String,
        footer: String,
        notes: String,
        commits: Vec<Commit>,
    }

    impl Fixture {
        fn new(package: &str, commits: Vec<Commit>) -> Self {
            Self {
                package: package.into(),
                tag: format!("{package}-v1.0.1"),
                metadata: metadata(package),
                header: String::new(),
                footer: String::new(),
                notes: full_notes(package, &commits),
                commits,
            }
        }

        fn section(&self, open: bool) -> PackageSection<'_> {
            PackageSection {
                name: &self.package,
                tag_name: &self.tag,
                open,
                metadata: PrMetadataBlock {
                    inline_content: self.metadata.clone(),
                    div_attribute: String::new(),
                },
                header: self.header.clone(),
                footer: self.footer.clone(),
                notes: self.notes.clone(),
                commits: &self.commits,
            }
        }
    }

    fn render(fixtures: &[Fixture], limit: Option<usize>) -> Result<String> {
        let sections = fixtures
            .iter()
            .map(|fixture| fixture.section(fixtures.len() == 1))
            .collect::<Vec<_>>();
        render_release_pr_body(&sections, limit)
    }

    /// Asserts every heading in `body` is followed by a bullet, never by
    /// the marker or the end of the notes.
    fn assert_no_empty_headings(body: &str) {
        let lines = body.lines().collect::<Vec<_>>();
        for (index, line) in lines.iter().enumerate() {
            if line.starts_with('#') {
                let next = lines[index + 1..]
                    .iter()
                    .find(|line| !line.trim().is_empty())
                    .unwrap();
                assert!(
                    next.starts_with("- "),
                    "heading {line:?} is followed by {next:?}"
                );
            }
        }
    }

    /// Asserts each package's tag and metadata still parse from `body`.
    fn assert_metadata_round_trips(fixtures: &[Fixture], body: &str) {
        for fixture in fixtures {
            let (tag, _) = parse_pr_body(&fixture.package, 1, body).unwrap();
            assert_eq!(tag, fixture.tag);
        }
    }

    #[test]
    fn body_within_limit_keeps_full_notes() {
        let fixtures = [Fixture::new("core", commits("core", 5))];

        let unlimited = render(&fixtures, None).unwrap();
        let limited = render(&fixtures, Some(LIMIT)).unwrap();

        assert_eq!(limited, unlimited);
        assert!(limited.contains(&fixtures[0].notes));
        assert!(!limited.contains(CONDENSED_NOTES_MARKER));
    }

    #[test]
    fn short_notes_fit_with_every_line() {
        let fixtures = [Fixture::new("core", commits("core", 30))];
        assert!(utf16_len(&render(&fixtures, None).unwrap()) > LIMIT);

        let body = render(&fixtures, Some(LIMIT)).unwrap();

        assert!(utf16_len(&body) <= LIMIT);
        assert!(body.contains(CONDENSED_NOTES_MARKER));
        assert!(body.contains(
            "### Breaking Changes\n\
             - [**breaking**]: drop the v1 endpoints of core _(corebrk)_\n\
             ### Bug Fixes\n\
             - correct a fairly long commit subject number 0 in core _(core0)_"
        ));
        assert!(body.contains("commit subject number 29 in core"));
        assert!(!body.contains("dev.azure.com"));
        assert!(!body.contains("migrate --to v2"));
        assert_metadata_round_trips(&fixtures, &body);
    }

    #[test]
    fn short_notes_are_trimmed_to_a_fair_share() {
        let fixtures = [
            Fixture::new("core", commits("core", 80)),
            Fixture::new("cli", commits("cli", 3)),
            Fixture::new("web", commits("web", 60)),
        ];

        let body = render(&fixtures, Some(LIMIT)).unwrap();

        assert!(utf16_len(&body) <= LIMIT);
        assert_eq!(body.matches(CONDENSED_NOTES_MARKER).count(), 3);
        // the short package keeps every line beside the trimmed long ones
        assert!(body.contains("commit subject number 2 in cli"));
        assert!(body.contains("commit subject number 0 in core"));
        assert!(!body.contains("commit subject number 79 in core"));
        assert_no_empty_headings(&body);
        assert_metadata_round_trips(&fixtures, &body);
    }

    #[test]
    fn length_is_counted_in_utf16_code_units() {
        // 🚀 is one character but two UTF-16 code units, as Azure DevOps
        // counts it
        let commits = (0..15)
            .map(|index| {
                commit(
                    "Features",
                    format!("{index} {}", "🚀".repeat(100)),
                    format!("abc{index}"),
                )
            })
            .collect::<Vec<_>>();
        let fixtures = [Fixture::new("core", commits)];
        let unlimited = render(&fixtures, None).unwrap();
        assert!(unlimited.chars().count() <= LIMIT);
        assert!(utf16_len(&unlimited) > LIMIT);

        let body = render(&fixtures, Some(LIMIT)).unwrap();

        assert!(utf16_len(&body) <= LIMIT);
        assert!(body.contains(CONDENSED_NOTES_MARKER));
    }

    #[test]
    fn preserved_header_and_footer_are_kept_while_notes_make_room() {
        let mut fixture = Fixture::new("core", commits("core", 80));
        fixture.header = "Read the migration guide first.".into();
        fixture.footer = "Ship it on Monday.".into();
        let fixtures = [fixture];

        let body = render(&fixtures, Some(LIMIT)).unwrap();

        assert!(utf16_len(&body) <= LIMIT);
        assert!(body.contains("Read the migration guide first."));
        assert!(body.contains("Ship it on Monday."));
    }

    #[test]
    fn oversized_footer_is_an_error_naming_the_package() {
        let mut fixture = Fixture::new("@acme/web-ui", commits("web", 3));
        fixture.footer = "x".repeat(LIMIT);

        let result = render(&[fixture], Some(LIMIT));

        let Err(ReleasaurusError::ForgeError(message)) = result else {
            panic!("expected a forge error, got {result:?}");
        };
        assert!(
            message
                .starts_with("release PR for package @acme/web-ui cannot fit"),
            "{message}"
        );
        // separate PRs would not help a single package
        assert!(!message.contains("separate_pull_requests"));
    }

    #[test]
    fn packages_over_limit_without_any_notes_suggest_separate_prs() {
        let fixtures = (0..40)
            .map(|index| {
                Fixture::new(&format!("package-{index}"), commits("pkg", 2))
            })
            .collect::<Vec<_>>();

        let result = render(&fixtures, Some(LIMIT));

        let Err(ReleasaurusError::ForgeError(message)) = result else {
            panic!("expected a forge error, got {result:?}");
        };
        assert!(message.starts_with("release PR for 40 packages cannot fit"));
        assert!(message.contains(&format!("limit of {LIMIT}")));
        assert!(message.contains("`separate_pull_requests = true`"));
    }

    #[test]
    fn many_commits_fit_quickly() {
        let fixtures = [Fixture::new("core", commits("core", 20_000))];

        let body = render(&fixtures, Some(LIMIT)).unwrap();

        assert!(utf16_len(&body) <= LIMIT);
        assert!(body.contains("commit subject number 0 in core"));
        assert_no_empty_headings(&body);
    }
}
