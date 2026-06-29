# Configuration

Releasaurus works with zero configuration for changelog generation and
tagging. Add an optional `releasaurus.toml` at your repository root when
you need more. This page covers the common cases; for the exhaustive
option list see the [Configuration Reference](./configuration-reference.md).

## Do You Need a Config File?

You **don't** need one if you only want changelog generation and tagging
with the default format and the default `v` tag prefix.

You **do** need one to:

- update version files (set a `release_type`)
- manage multiple packages (monorepo)
- create prereleases (alpha/beta/rc/snapshot)
- customize the changelog or use custom tag prefixes

Place the file at the repository root:

```
my-project/
├── releasaurus.toml
├── src/
└── README.md
```

Config is organized under three top-level tables:

- **`[repository]`** — repo-wide settings (base branch, search depths,
  combined vs. separate PRs, and commit modifiers — `skip_shas`/`reword`).
- **`[global]`** — release defaults for every package, including
  `[global.prerelease]` and `[global.changelog]`. Most keys can be
  overridden per package.
- **`[[package]]`** — one entry per independently-versioned package.

See the [Configuration Reference](./configuration-reference.md) for every
key.

## Single Package

The most common setup — bump versions in one package's manifests:

```toml
[[package]]
path = "."
release_type = "node"  # or rust, python, java, php, ruby, go, generic
```

`release_type` selects which manifest and lock files are updated. See
[Supported Languages](./configuration-reference.md#supported-languages)
for the file list per language.

## Monorepos

Define one `[[package]]` per independently-versioned package. Each gets
its own version, tag prefix, and manifest updates.

```toml
[[package]]
path = "./frontend"
release_type = "node"
tag_prefix = "frontend-v"

[[package]]
path = "./backend"
release_type = "rust"
tag_prefix = "backend-v"
```

Tag prefix defaults to `v` for a root package (`path = "."`) and
`<name>-v` for nested packages.

### Combined vs. Separate PRs

By default all packages with changes are released in a **single** PR. Set
`separate_pull_requests = true` under `[repository]` to give each package
its own PR (branches like `releasaurus-release-main-frontend`):

```toml
[repository]
separate_pull_requests = true

[[package]]
path = "./frontend"
release_type = "node"
tag_prefix = "frontend-v"

[[package]]
path = "./backend"
release_type = "rust"
tag_prefix = "backend-v"
```

- **Combined (default)** — best for tightly-coupled packages that release
  together and a single, atomic review.
- **Separate** — best for large monorepos and independently-versioned
  packages with different release cadences or owners.

In either mode, target one package with `--package <name>` on `release-pr`
and `release`.

### Tracking Shared Code

Use `additional_paths` so a package also releases when shared directories
change:

```toml
[[package]]
path = "./apps/web"
release_type = "node"
tag_prefix = "web-v"
additional_paths = ["shared/types", "shared/utils"]
```

### Workspaces in a Subdirectory

When a workspace isn't at the repo root, set `workspace_root` so lock
files resolve correctly:

```toml
[[package]]
name = "api-server"
workspace_root = "backend"
path = "services/api"
release_type = "rust"
tag_prefix = "api-v"
```

This updates `backend/services/api/Cargo.toml` and the workspace
`backend/Cargo.lock`.

### Naming & Path Rules

- **Names must be unique** across all packages. If omitted, the name is
  derived from the last path component. Match the manifest's `name` field
  where one exists (`package.json`, `Cargo.toml`, etc.).
- **The full path (`workspace_root` + `path`) must be unique.** Two
  packages may share a `path` only if their `workspace_root` differs.

## Grouped Releases (Sub-Packages)

Use `sub_packages` to release several packages under **one** shared tag,
changelog, and release, while each sub-package still gets its own manifest
updates based on its `release_type`. A sub-package does **not** produce its
own tag.

```toml
[[package]]
name = "platform"
workspace_root = "."
path = "."
tag_prefix = "v"
sub_packages = [
    { name = "web", path = "packages/web", release_type = "node" },
    { name = "cli", path = "packages/cli", release_type = "rust" },
]
```

Result: one tag (`v1.0.0`), one changelog covering everything, one
release — with `package.json` (web) and `Cargo.toml` (cli) updated
independently. Reach for this when a group of packages must always ship
together with the same version.

> **Sub-packages vs. separate packages:** separate `[[package]]` entries
> are versioned and tagged independently; `sub_packages` share the
> parent's single tag and changelog.

## Prereleases

Publish alpha/beta/rc/snapshot versions before a stable release. Configure
globally with `[global.prerelease]` or per-package with a `prerelease`
table.

```toml
[global.prerelease]
suffix = "alpha"
strategy = "versioned"  # or "static"

[[package]]
path = "."
release_type = "node"
```

### Strategies

- **`versioned`** (default) — appends an incrementing counter:
  `1.1.0-alpha.1`, `1.1.0-alpha.2`, …
- **`static`** — appends the suffix as-is, with no counter:
  `1.0.1-SNAPSHOT` (common in Java).

### Lifecycle

Change behavior by editing the config and opening a new release PR:

| From             | Config change                                   | Result           |
| ---------------- | ----------------------------------------------- | ---------------- |
| `v1.0.0`         | `suffix = "alpha"` (+ feature commit)           | `v1.1.0-alpha.1` |
| `v1.1.0-alpha.1` | unchanged (+ fix commit)                        | `v1.1.0-alpha.2` |
| `v1.0.0-alpha.3` | `suffix = "beta"` (+ feature)                   | `v1.1.0-beta.1`  |
| `v1.0.0-alpha.5` | remove `[global.prerelease]` (or `suffix = ""`) | `v1.0.0`         |

Switching the suffix recalculates the base version and resets the
counter. Removing the prerelease config graduates to a stable release.

### Per-Package Overrides

```toml
[global.prerelease]
suffix = "beta"
strategy = "versioned"

[[package]]
path = "./stable"
release_type = "rust"
# inherits the global beta prerelease

[[package]]
path = "./experimental"
release_type = "rust"
prerelease = { suffix = "alpha", strategy = "versioned" }
```

### Aggregating Prerelease Notes

When graduating to stable, include the changelog entries from every prior
prerelease:

```toml
[global.changelog]
aggregate_prereleases = true
```

You can also override prerelease settings per run without editing the
config — see [Configuration Overrides](./commands.md#configuration-overrides)
(`--prerelease-suffix`, `--prerelease-strategy`, `--set-package`).

## Per-Package Changelog

Changelog settings normally live under `[global.changelog]` and apply to
every package. A single package can override them on its own `changelog`
key. Since packages are an array of tables (`[[package]]`), set it as an
inline table so it stays scoped to that entry:

```toml
[global.changelog]
include_author = true

[[package]]
name = "frontend"
path = "./apps/web"
release_type = "node"
changelog = { include_author = false }
```

> **A package `changelog` merges field-by-field with `[global.changelog]`.**
> Any field you set on the package wins; any field you omit is inherited
> from your `[global.changelog]` values (and then the built-in defaults).
> Global and package `custom_parsers` are combined, and `named_parsers`
> overrides apply per group. See
> [Changelog Customization](./changelog.md#per-package-changelog).

## Skipping or Rewording Commits

`skip_shas` and `reword` live under `[repository]`. They operate on the
repository's shared commit history and affect **version calculation as well
as** the changelog, so they are repo-wide and cannot be overridden per
package.

`skip_shas` removes specific commits by SHA prefix (use 7+ characters) —
handy for commits that shouldn't affect versioning or appear in the
changelog:

```toml
[repository]
skip_shas = ["abc123d", "def456e"]
```

`reword` rewrites a commit's message. The new message affects **both** the
changelog text **and** the version bump — changing `fix:` to `feat:`, for
example, bumps minor instead of patch:

```toml
[[repository.reword]]
sha = "abc123d"
message = "feat: added user authentication"
```

Both have CLI equivalents for one-off runs: `--skip-sha <sha>` and
`--reword <sha>=<message>`; a `--reword` for a SHA already in config wins.
See the [Configuration Reference](./configuration-reference.md#repository)
for the terse lookup form.

`skip_shas` and `reword` only affect each package's next release.
Releasaurus processes a package's commits from its most recent tag forward,
so once a release is tagged those commits are never reprocessed. (In a
monorepo a single entry can therefore apply to more than one package — but
only that package's next release in each case.)

If you just want to change how a single release's notes read — without
affecting the version bump — edit them directly in the release PR instead;
see [Editing Release Notes](./release-notes-editing.md).

## Testing Your Configuration

Validate any config change locally before pushing — no token, no remote
changes:

```bash
releasaurus release-pr --forge local --repo "."
```

Check that packages are detected, tag prefixes match, and the
combined/separate PR strategy behaves as expected. See
[Local Repository Mode](./commands.md#local-repository-mode).

## Next Steps

- **[Changelog Customization](./changelog.md)** — filter commits and
  customize the template.
- **[Configuration Reference](./configuration-reference.md)** — every
  option, default, and the full example config.
