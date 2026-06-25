# Configuration Reference

Complete reference for `releasaurus.toml`, environment variables, and
supported languages. For guidance and examples, see
[Configuration](./configuration.md).

## Global Settings

Top-level keys, all optional:

| Key                               | Type    | Default      | Description                                                                                       |
| --------------------------------- | ------- | ------------ | ------------------------------------------------------------------------------------------------- |
| `base_branch`                     | string  | repo default | Branch targeted for PRs, tagging, and releases. Override: `--base-branch`.                        |
| `first_release_search_depth`      | integer | `400`        | Commits to analyze for the **first** release (when no matching tag exists).                       |
| `tag_search_depth`                | integer | `100`        | Max tags fetched when searching for a previous release. `0` = all tags.                           |
| `separate_pull_requests`          | bool    | `false`      | One PR per package (`true`) vs. a single combined PR (`false`).                                   |
| `auto_start_next`                 | bool    | `false`      | Bump patch versions automatically after a release (see [`start-next`](./commands.md#start-next)). |
| `breaking_always_increment_major` | bool    | `true`       | Breaking changes (`feat!:`, `BREAKING CHANGE:`) bump major.                                       |
| `features_always_increment_minor` | bool    | `true`       | `feat:` commits bump minor.                                                                       |
| `custom_major_increment_regex`    | string  | none         | Additional regex that triggers a major bump.                                                      |
| `custom_minor_increment_regex`    | string  | none         | Additional regex that triggers a minor bump.                                                      |

### Custom increment regexes

`custom_major_increment_regex` and `custom_minor_increment_regex` are
**additive** — breaking changes always bump major and `feat:` always
bumps minor regardless. The pattern is matched against the full commit
message. In TOML double-quoted strings, escape backslashes (`\\`):

```toml
custom_major_increment_regex = "\\[MAJOR\\]"   # matches "[MAJOR]"
custom_minor_increment_regex = "FEATURE"        # no escaping needed
```

## `[prerelease]`

Global prerelease config; can be overridden per package via a package
`prerelease` table. See [Prereleases](./configuration.md#prereleases).

| Key        | Type   | Default       | Description                                                                                     |
| ---------- | ------ | ------------- | ----------------------------------------------------------------------------------------------- |
| `suffix`   | string | none (stable) | Identifier such as `alpha`, `beta`, `rc`, `SNAPSHOT`. Override: `--prerelease-suffix`.          |
| `strategy` | string | `versioned`   | `versioned` (adds `.1`, `.2`, …) or `static` (suffix as-is). Override: `--prerelease-strategy`. |

```toml
[prerelease]
suffix = "beta"
strategy = "versioned"
```

## `[changelog]`

Controls changelog generation. See
[Changelog Customization](./changelog.md) for the template and variables.

| Key                     | Type     | Default           | Description                                                                                                                                   |
| ----------------------- | -------- | ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `default_parsers`       | table    | built-in groups   | Override built-in commit groups (`pattern`/`title`/`skip` per group). See [Changelog Customization](./changelog.md#commit-groups--filtering). |
| `custom_parsers`        | array    | none              | Define additional commit groups, checked before the defaults.                                                                                 |
| `skip_merge_commits`    | bool     | `true`            | Exclude merge commits.                                                                                                                        |
| `include_author`        | bool     | `false`           | Include commit author names.                                                                                                                  |
| `aggregate_prereleases` | bool     | `false`           | On graduation, fold prior prerelease notes into the stable release.                                                                           |
| `skip_shas`             | string[] | none              | Skip commits by SHA prefix (7+ chars). CLI: `--skip-sha`.                                                                                     |
| `reword`                | object[] | none              | Rewrite commit messages (affects changelog **and** version bump). CLI: `--reword`.                                                            |
| `body`                  | string   | standard template | Tera template for the changelog body.                                                                                                         |

```toml
[changelog]
include_author = true

[changelog.default_parsers]
ci.skip = true
chore.skip = true

[[changelog.reword]]
sha = "abc123d"
message = "fix: corrected description"
```

## `[[package]]`

One entry per package; repeatable.

| Key                               | Type                | Default                          | Description                                                                                                               |
| --------------------------------- | ------------------- | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `path`                            | string              | `.`                              | Package directory, relative to `workspace_root`.                                                                          |
| `workspace_root`                  | string              | `.`                              | Workspace root, relative to repo root.                                                                                    |
| `name`                            | string              | derived from path                | Explicit package name; must be unique.                                                                                    |
| `release_type`                    | string              | none                             | Language for version updates (see [Supported Languages](#supported-languages)). Omit for changelog/tagging only.          |
| `tag_prefix`                      | string              | `v` (root) / `<name>-v` (nested) | Git tag prefix. Override: `--tag-prefix` or `--set-package <name>.tag_prefix=`.                                           |
| `prerelease`                      | table               | inherits global                  | Per-package prerelease override. Override: `--set-package <name>.prerelease.suffix=`.                                     |
| `sub_packages`                    | object[]            | none                             | Group packages under one shared tag/changelog (see [Grouped Releases](./configuration.md#grouped-releases-sub-packages)). |
| `additional_paths`                | string[]            | none                             | Extra directories whose changes trigger a release for this package.                                                       |
| `additional_manifest_files`       | string[] / object[] | none                             | Extra files to version-bump (see below).                                                                                  |
| `auto_start_next`                 | bool                | inherits global                  | Per-package `auto_start_next` override.                                                                                   |
| `breaking_always_increment_major` | bool                | inherits global                  | Per-package override.                                                                                                     |
| `features_always_increment_minor` | bool                | inherits global                  | Per-package override.                                                                                                     |
| `custom_major_increment_regex`    | string              | inherits global                  | Per-package override.                                                                                                     |
| `custom_minor_increment_regex`    | string              | inherits global                  | Per-package override.                                                                                                     |

`sub_packages` entries take `name`, `path`, and `release_type`.

### `additional_manifest_files`

Extra files whose version strings should be kept in sync — custom
`VERSION` files, docs, config, etc. Accepts plain string paths (using a
default regex) or objects with a custom `version_regex`. All paths are
relative to the package `path`.

```toml
[[package]]
path = "."
release_type = "rust"
additional_manifest_files = [
    "VERSION",                    # default regex
    "README.md",                  # default regex
    { path = "helm/Chart.yaml", version_regex = "appVersion:\\s*\"?(?<version>\\d+\\.\\d+\\.\\d+)\"?" },
]
```

The default regex matches common forms like `version = "1.0.0"`,
`version: "1.0.0"`, `VERSION='1.0.0'`, and `"version": "1.0.0"`. A custom
`version_regex` **must** include a named capture group `(?<version>...)`;
only that group is replaced. Files without a match are skipped; an invalid
regex errors during config resolution.

## Complete Example

```toml
# Global settings
base_branch = "main"
first_release_search_depth = 400
separate_pull_requests = false
auto_start_next = false
breaking_always_increment_major = true
features_always_increment_minor = true

[prerelease]
suffix = "beta"
strategy = "versioned"

[changelog]
include_author = false

[changelog.default_parsers]
ci.skip = true
chore.skip = true

[[package]]
name = "frontend"
path = "./apps/web"
release_type = "node"
tag_prefix = "web-v"

[[package]]
name = "backend"
path = "./services/api"
release_type = "rust"
tag_prefix = "api-v"
prerelease = { suffix = "alpha", strategy = "versioned" }
```

## Environment Variables

Releasaurus selects the auth token automatically from the `--forge` type;
`--token` overrides it. The `RELEASAURUS_*` variables are fallbacks for
their matching CLI flags, and flags always win.

For the auth token, each forge accepts two env vars: a
`RELEASAURUS_`-prefixed name and the bare name. The prefixed name takes
precedence. Prefer it on Gitea/Forgejo CI runners (including Codeberg),
which auto-inject their own limited token into the bare `*_TOKEN` name
and would otherwise shadow your PAT — see
[the CI/CD integration notes](./ci-cd-integration.md) and
[this known limitation][token-limit].

[token-limit]: ./commands.md#gitea-and-forgejo-actions-injected-token-shadows-your-pat

| Variable                                                | Purpose                                                             |
| ------------------------------------------------------- | ------------------------------------------------------------------- |
| `RELEASAURUS_GITHUB_TOKEN` / `GITHUB_TOKEN`             | GitHub auth token                                                   |
| `RELEASAURUS_GITLAB_TOKEN` / `GITLAB_TOKEN`             | GitLab auth token                                                   |
| `RELEASAURUS_GITEA_TOKEN` / `GITEA_TOKEN`               | Gitea auth token                                                    |
| `RELEASAURUS_FORGEJO_TOKEN` / `FORGEJO_TOKEN`           | Forgejo auth token                                                  |
| `RELEASAURUS_AZURE_DEVOPS_TOKEN` / `AZURE_DEVOPS_TOKEN` | Azure DevOps PAT (experimental)                                     |
| `RELEASAURUS_FORGE`                                     | Default `--forge`                                                   |
| `RELEASAURUS_REPO`                                      | Default `--repo`                                                    |
| `RELEASAURUS_LOCAL_PATH`                                | Default `--local-path` (hybrid mode)                                |
| `RELEASAURUS_CONFIG`                                    | Default `--config`                                                  |
| `RELEASAURUS_DEBUG`                                     | Enable debug logging when set to any non-empty value                |
| `RELEASAURUS_DRY_RUN`                                   | Enable dry-run (auto-enables debug) when set to any non-empty value |

### Required token scopes

| Forge                     | Scopes / permissions                                                                                                                      |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| **GitHub** (classic)      | `repo`                                                                                                                                    |
| **GitHub** (fine-grained) | Contents, Issues, Pull requests — all read & write. Add Actions/Workflows read & write only if using the Action to modify workflow files. |
| **GitLab**                | `api`, `write_repository`                                                                                                                 |
| **Gitea**                 | repository (read/write), issue (read/write), misc (read/write) management                                                                 |
| **Forgejo**               | repository (read/write), issue (read/write), misc (read/write) management                                                                 |
| **Azure DevOps**          | `Code: Read & Write`, `Pull Request Threads: Read & Write`                                                                                |

`RELEASAURUS_DEBUG` and `RELEASAURUS_DRY_RUN` are enabled by _any_
non-empty value (including `false` or `0`); unset or empty to disable.
The `--debug` / `--dry-run` flags always enable regardless of the
variable.

## Supported Languages

Set `release_type` on a package and Releasaurus updates the matching
manifest and lock files. Lock files are updated when present, and all
languages support workspace/monorepo layouts.

| `release_type` | Files updated                                                                                   |
| -------------- | ----------------------------------------------------------------------------------------------- |
| `generic`      | Custom files via [`additional_manifest_files`](#additional_manifest_files)                      |
| `go`           | `version.go`, `version/version.go`, `internal/version.go`, `internal/version/version.go`        |
| `java`         | `pom.xml`, `build.gradle`, `build.gradle.kts`, `gradle.properties`, `gradle/libs.versions.toml` |
| `node`         | `package.json`, `package-lock.json`, `yarn.lock`                                                |
| `php`          | `composer.json`, `composer.lock`                                                                |
| `python`       | `pyproject.toml`, `setup.py`, `setup.cfg`                                                       |
| `ruby`         | `*.gemspec`, `Gemfile`, `Gemfile.lock`                                                          |
| `rust`         | `Cargo.toml`, `Cargo.lock`                                                                      |
