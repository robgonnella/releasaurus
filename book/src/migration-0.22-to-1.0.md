# Migration Guide

Upgrading from **v0.22.x** to **v1.0.0**.

This release restructures `releasaurus.toml` completely, changes two
`--set-package` override paths, drops support for two legacy release-PR
state formats, and reshapes the `releasaurus-core` public API. Nothing
here is optional: a 0.22 config will not load on 1.0.0.

## At a glance

1. [Before you upgrade](#before-you-upgrade-drain-open-release-prs) —
   drain release PRs created by versions older than v0.17.0.
2. [TOML config restructure](#toml-config-restructure) — every key now
   lives under `[repository]`, `[defaults]`, or `[[package]]`.
3. [CLI override paths](#cli-override-paths) — prerelease
   `--set-package` paths gained a `versioning.` segment.
4. [Monorepo commit message and PR
   title](#monorepo-commit-message-and-pr-title) — the default now
   includes the repository name.
5. [`custom_major_increment_regex`](#custom_major_increment_regex-now-groups-commits-as-breaking)
   — matching commits are now grouped as breaking, not just bumped.
6. [Library API](#library-api) — for Rust consumers of
   `releasaurus-core`.

Unknown keys are now rejected at config load. A key you forget to move
is a hard error naming the offending field, not a silent no-op, so the
upgrade fails loudly rather than quietly computing the wrong version.
Work through section 2 and let the error messages guide you.

## Before you upgrade: drain open release PRs

Releasaurus keeps release state in the PR itself — a `releasaurus`
label and a JSON metadata block in the PR body. v1.0.0 removes the
compatibility shims for the pre-v0.17.0 forms of both:

- The legacy pending label `releasaurus:pending` (single colon) is no
  longer recognized. Only the scoped `releasaurus::pending` is, which
  has been the format written since v0.17.0-rc.1.
- The legacy PR-body metadata format is no longer parsed. Only the
  current JSON HTML-comment block is, written since v0.17.0-rc.2.

**Who this affects:** only repositories with an open — or merged but
not yet released — release PR created by a version **older than
v0.17.0**. After upgrading, `releasaurus release` will not find that
PR, so its tag and release are never published.

**What to do:** while still on 0.22.x, either merge the PR and run
`releasaurus release` to finish it, or close it and let v1.0.0 open a
fresh one. If your last release ran on v0.17.0 or newer, there is
nothing to do.

## TOML config restructure

Configuration is now grouped under three top-level tables:

- `[repository]` — settings that apply to the repository as a whole.
- `[defaults]` — release defaults for every package, with
  `[defaults.versioning]` and `[defaults.changelog]` subtables.
- `[[package]]` — one entry per package, unchanged in spirit; each may
  override `[defaults]` with its own `versioning` and `changelog`.

The full reference lives in the
[Configuration Reference](./configuration-reference.md). The tables
below cover only what moved.

### Root-level keys

| v0.22.x                           | v1.0.0                                                  |
| --------------------------------- | ------------------------------------------------------- |
| `base_branch`                     | `[repository].base_branch`                              |
| `first_release_search_depth`      | `[repository].first_release_search_depth`               |
| `tag_search_depth`                | `[repository].tag_search_depth`                         |
| `separate_pull_requests`          | `[repository].separate_pull_requests`                   |
| `auto_start_next`                 | `[defaults.versioning].auto_start_next`                 |
| `breaking_always_increment_major` | `[defaults.versioning].breaking_always_increment_major` |
| `features_always_increment_minor` | `[defaults.versioning].features_always_increment_minor` |
| `custom_major_increment_regex`    | `[defaults.versioning].custom_major_increment_regex`    |
| `custom_minor_increment_regex`    | `[defaults.versioning].custom_minor_increment_regex`    |
| `[prerelease]`                    | `[defaults.versioning.prerelease]`                      |

### The `[changelog]` table

The old `[changelog]` table mixed two concerns, so its keys split three
ways. Options that only affect how commits are _rendered_ stay in
`[defaults.changelog]`. Options that determine which commits are
_counted_, and therefore change the computed version, moved to
`[defaults.versioning]`. `skip_shas` and `reword` act on the
repository's shared commit history and cannot be scoped to a package,
so they moved to `[repository]`.

| v0.22.x `[changelog]`   | v1.0.0                                            |
| ----------------------- | ------------------------------------------------- |
| `body`                  | `[defaults.changelog].body`                       |
| `include_author`        | `[defaults.changelog].include_author`             |
| `aggregate_prereleases` | `[defaults.changelog].aggregate_prereleases`      |
| `skip_merge_commits`    | `[defaults.versioning].skip_merge_commits`        |
| `skip_shas`             | `[repository].skip_shas`                          |
| `[[changelog.reword]]`  | `[[repository.reword]]`                           |
| the nine `skip_*` flags | `[defaults.versioning.named_parsers]` — see below |

### `skip_*` flags become `named_parsers`

The nine per-type skip flags are replaced by the `named_parsers` table,
where each commit group carries a `pattern`, `title`, `order`, and
`skip`. Setting `skip = true` on a group is the direct equivalent of
the old flag:

| v0.22.x              | v1.0.0               |
| -------------------- | -------------------- |
| `skip_ci`            | `ci.skip`            |
| `skip_chore`         | `chore.skip`         |
| `skip_doc`           | `documentation.skip` |
| `skip_test`          | `test.skip`          |
| `skip_style`         | `style.skip`         |
| `skip_refactor`      | `refactor.skip`      |
| `skip_perf`          | `performance.skip`   |
| `skip_revert`        | `revert.skip`        |
| `skip_miscellaneous` | `miscellaneous.skip` |

Three group names are spelled out rather than abbreviated — watch
`documentation` (not `doc`), `performance` (not `perf`), and `feature`
(not `feat`) if you go on to retitle or reorder groups. The full set is
`breaking`, `feature`, `fix`, `revert`, `refactor`, `performance`,
`documentation`, `style`, `test`, `chore`, `ci`, `miscellaneous`.

So this:

```text
[changelog]
skip_ci = true
skip_chore = true
```

becomes:

```toml
[defaults.versioning.named_parsers]
ci.skip = true
chore.skip = true
```

`named_parsers` merges field-by-field over the built-in defaults, so
naming one group leaves the other eleven untouched — you never have to
restate the whole set.

**Filtering behavior is unchanged.** In 0.22.x a skipped commit was
dropped before it reached version calculation, so skipping a group
already suppressed both its changelog entries and its version bump.
Only the config location moved. What is new is that groups the old
flags could not reach — `feature`, `fix`, `breaking` — can now be
skipped too, and that you can define additional groups with
`[[defaults.versioning.custom_parser]]`.

### Package-level keys

Six keys are no longer direct `[[package]]` keys. They now live under
the package's `versioning` table, mirroring `[defaults.versioning]`:

- `prerelease`
- `auto_start_next`
- `breaking_always_increment_major`
- `features_always_increment_minor`
- `custom_major_increment_regex`
- `custom_minor_increment_regex`

Unchanged: `name`, `path`, `workspace_root`, `release_type`,
`tag_prefix`, `sub_packages`, `additional_paths`, and
`additional_manifest_files`.

Because packages are an array of tables, set `versioning` as an
**inline table** on the package entry. A separate `[package.versioning]`
header would bind to whichever `[[package]]` was declared last, which
is almost never what you want:

Before:

```text
[[package]]
name = "backend"
path = "./services/api"
prerelease = { suffix = "alpha", strategy = "versioned" }
```

After:

```toml
[[package]]
name = "backend"
path = "./services/api"
versioning = { prerelease = { suffix = "alpha", strategy = "versioned" } }
```

One asymmetry to be aware of when splitting config across `[defaults]`
and packages: a package's `versioning` and `changelog` tables merge
field-by-field with their `[defaults]` counterpart, but `prerelease`
**replaces** the `[defaults]` one outright. `suffix` and `strategy`
describe a single prerelease identity, so restate `strategy` alongside
a package-level `suffix` whenever your default strategy is not the
built-in `versioned`.

### Worked example

This repository's own config, before and after. Before:

```text
#:schema ./schema/schema.json

tag_search_depth = 25

[changelog]
skip_ci = true
skip_chore = true
skip_miscellaneous = false
include_author = true
aggregate_prereleases = true

[prerelease]
strategy = "versioned"
suffix = "rc"

[[package]]
name = "workspace"
tag_prefix = "v"
release_type = "rust"
```

After:

```toml
#:schema ./schema/schema.json

[repository]
tag_search_depth = 25

[defaults.changelog]
include_author = true
aggregate_prereleases = true

[defaults.versioning.named_parsers]
ci.skip = true
chore.skip = true
miscellaneous.skip = false

[defaults.versioning.prerelease]
strategy = "versioned"
suffix = "rc"

[[package]]
name = "workspace"
tag_prefix = "v"
release_type = "rust"
```

## CLI override paths

Two `--set-package` paths gained a `versioning.` segment so that they
mirror the TOML layout they override:

| v0.22.x                             | v1.0.0                                         |
| ----------------------------------- | ---------------------------------------------- |
| `<pkg>.prerelease.suffix=<value>`   | `<pkg>.versioning.prerelease.suffix=<value>`   |
| `<pkg>.prerelease.strategy=<value>` | `<pkg>.versioning.prerelease.strategy=<value>` |

```bash
# v0.22.x
releasaurus release-pr --set-package frontend.prerelease.suffix=beta ...

# v1.0.0
releasaurus release-pr \
  --set-package frontend.versioning.prerelease.suffix=beta ...
```

`<pkg>.tag_prefix` is unchanged, and so are all the global flags:
`--base-branch`, `--tag-prefix`, `--prerelease-suffix`,
`--prerelease-strategy`, `--skip-sha`, and `--reword`. Emptying a
suffix to disable prereleases still works the same way
(`--set-package <pkg>.versioning.prerelease.suffix=`).

A stale path now fails with a config error naming the offending
override rather than being silently ignored, so a missed occurrence in
a CI workflow surfaces on the next run instead of producing an
unexpected version.

## Monorepo commit message and PR title

Release commit messages and PR titles are now rendered from Tera
templates, and the default for combined monorepo PRs includes the
repository name. This is one of two changes that alter output without
any config edit on your part (the other is
[`custom_major_increment_regex`](#custom_major_increment_regex-now-groups-commits-as-breaking)),
so check anything that matches on release PR titles — branch protection
rules, required status checks, CI `if:` conditions, merge automation.

0.22.x picked the format from how many packages happened to be in the
PR on that run:

- one package in the PR — `chore(<branch>): release <pkg> <tag>`
- more than one — a bare `chore(<branch>): release`

1.0.0 picks from config instead. A repo with `separate_pull_requests =
true`, or with a single `[[package]]`, uses the per-package template
(default `chore({{ branch }}): release {{ package_name }} {{ tag }}`,
identical to before). Any other repo produces a combined PR and uses
the monorepo template, whose default is:

```text
chore({{ branch }}): release {{ repo_name }}
```

For a multi-package repo with `separate_pull_requests = false`, that
means the title now always carries the repository name and no longer
collapses to `release <pkg> <tag>` on runs where only one package
changed. Deciding from config keeps the format stable from one release
to the next.

To keep the old bare title, set the templates explicitly:

```toml
[defaults]
monorepo_commit_message_template = "chore({{ branch }}): release"
monorepo_pr_title_template = "chore({{ branch }}): release"
```

The monorepo templates have `branch` and `repo_name` in scope. The
per-package templates additionally have `package_name`, `tag`, and
`semver`. Referencing a variable that is not in scope is rejected at
config load.

## `custom_major_increment_regex` now groups commits as breaking

The key moved to `[defaults.versioning]` like the rest, but its effect
also widened. In 0.22.x it only influenced the version bump; a matching
commit still appeared under whatever group its type prefix selected. In
1.0.0 a commit it matches is treated as breaking outright: grouped under
`❌ Breaking`, marked `[**breaking**]` in the default body template, and
bumping major as before.

**Who this affects:** anyone who already sets
`custom_major_increment_regex`. No config edit is required and the
computed version does not change — only which changelog heading those
commits appear under.

If you were relying on the old split — bump major, but keep the commit
filed under its own type — there is no longer a setting for that; the
two concepts are deliberately unified. The reverse case is now possible
though: `[defaults.versioning.named_parsers] breaking.pattern` is the
same mechanism under a different name, so pick whichever key reads
better and know they are combined if you set both.

`custom_minor_increment_regex` is unchanged — it affects the version
bump only, with no grouping effect.

## Library API

For Rust consumers of the `releasaurus-core` crate. The CLI binary
needs none of this.

| v0.22.x                                                                               | v1.0.0                                      |
| ------------------------------------------------------------------------------------- | ------------------------------------------- |
| `config::resolved::{GlobalOverrides, PackageOverrides, CommitModifiers, PackageName}` | `config::overrides::*` — same names         |
| `config::resolved::ResolvedConfig`                                                    | `resolver::ResolvedConfig` — reshaped       |
| `config::changelog::RewordedCommit`                                                   | `config::repository::RewordedCommit`        |
| `config::{DEFAULT_COMMIT_SEARCH_DEPTH, DEFAULT_TAG_SEARCH_DEPTH}`                     | `config::repository::*`                     |
| `Config { base_branch, changelog, prerelease, … }`                                    | `Config { repository, defaults, packages }` |

Beyond the moves:

- **`Resolver::resolve()` returns one value.** It now yields
  `Rc<ResolvedConfig>` rather than a
  `(ResolvedConfig, ResolvedPackageHash)` tuple; the resolved packages
  hang off `ResolvedConfig::package_configs`. Drop the
  `.package_configs(...)` call from `Orchestrator::builder()`. The
  crate-level quick start in
  [`crates/core/src/lib.rs`](https://github.com/robgonnella/releasaurus/blob/main/crates/core/src/lib.rs)
  shows the full builder chain, and [Library API](./library-api.md) has
  the narrative version.
- **`PrereleaseConfig::suffix` is a `String`**, not an
  `Option<String>`. The `suffix()` accessor that unwrapped it is gone —
  read the field directly, and treat `""` as "no prerelease".
- **New `config::versioning` module** holding `VersionType`, `Group`,
  `Parser`, `ParserList`, `VersioningConfig`, and `NAMED_PARSERS`.
  `Group`'s variants were renamed to spell out `Feature`,
  `Documentation`, `Performance`, and `CI` — which is why the TOML keys
  read `documentation` and `performance` rather than `doc` and `perf`.
- **`Forge` gained a required method**,
  `get_merged_pull_request_for_commit`, which resolves the merged PR
  that introduced a commit. If you implement `Forge` outside this
  crate, add it; returning `Ok(None)` is a valid no-op and simply means
  the `include_pr_link` changelog option renders nothing for that
  forge.

## Not affected

- **Existing tags and changelogs.** Git tags, `CHANGELOG.md` files, and
  published releases are read and appended to exactly as before. No
  retagging or history rewrite is needed.
- **Custom `body` templates.** Group headings still carry the
  `<!-- NN -->` ordering tag, so the standard
  `{{ group | striptags | trim }}` idiom keeps working unchanged. The
  template context only gained fields: `short_sha`, `include_pr_link`,
  and `commit.pr`.
- **Environment variables.** Every `RELEASAURUS_*` variable and bare
  `*_TOKEN` fallback behaves as it did, with the same precedence.
- **`release_type` values and version-file updaters.** All languages,
  manifest files, lock files, `sub_packages`, and
  `additional_manifest_files` behavior are unchanged.
- **The JSON schema location.** `#:schema ./schema/schema.json` still
  points at the right file; the schema itself was regenerated for the
  new layout, so editor completion reflects it.
