# Configuration Reference

Complete reference of all configuration options for `releasaurus.toml`.

## Global Settings

### `base_branch`

**Type**: String (optional)

**Default**: Repository's default branch

Base branch to target for release PRs, tagging, and releases.

```toml
base_branch = "main"
```

**Override**: `--base-branch` CLI flag

### `first_release_search_depth`

**Type**: Integer (optional)

**Default**: 400

Number of commits to analyze for the first release (when no tags
exist).

```toml
first_release_search_depth = 400
```

**When to adjust:**

- Large repos: decrease to 100-200 for faster analysis
- Need more history: increase to 1000+
- CI/CD: use smaller values for speed

### `separate_pull_requests`

**Type**: Boolean (optional)

**Default**: false

Create separate PRs for each package (true) or one combined PR
(false).

```toml
separate_pull_requests = true
```

See [Monorepo Configuration](./configuration-monorepo.md) for details.

### `auto_start_next`

**Type**: Boolean (optional)

**Default**: false

Automatically bump patch versions after publishing a release.

```toml
auto_start_next = true
```

Package-level settings override global. See [`start-next`
command](./commands.md#start-next).

### `breaking_always_increment_major`

**Type**: Boolean (optional)

**Default**: true

Breaking changes (`feat!:` or `BREAKING CHANGE:`) increment major
version.

```toml
breaking_always_increment_major = false
```

### `features_always_increment_minor`

**Type**: Boolean (optional)

**Default**: true

Feature commits (`feat:`) increment minor version.

```toml
features_always_increment_minor = false
```

### `custom_major_increment_regex`

**Type**: String (optional)

**Default**: None

Custom regex pattern to trigger major version bumps (additive with
breaking changes).

```toml
custom_major_increment_regex = "MAJOR"
```

### `custom_minor_increment_regex`

**Type**: String (optional)

**Default**: None

Custom regex pattern to trigger minor version bumps (additive with
`feat:` commits).

```toml
custom_minor_increment_regex = "FEATURE"
```

## Prerelease Section

### `[prerelease]`

Global prerelease configuration for all packages.

#### `suffix`

**Type**: String (optional)

**Default**: None (stable releases)

Prerelease identifier (e.g., "alpha", "beta", "rc", "SNAPSHOT").

```toml
[prerelease]
suffix = "alpha"
```

**Override**: `--prerelease-suffix` CLI flag

#### `strategy`

**Type**: String (optional)

**Default**: "versioned"

Prerelease versioning strategy:

- `"versioned"` - Adds incremental counter (`.1`, `.2`)
- `"static"` - Uses suffix as-is

```toml
[prerelease]
suffix = "beta"
strategy = "versioned"
```

**Override**: `--prerelease-strategy` CLI flag

See [Prerelease Configuration](./configuration-prerelease.md) for
complete details.

## Changelog Section

### `[changelog]`

Customize changelog generation and formatting.

#### `skip_ci`

**Type**: Boolean (optional)

**Default**: false

Exclude CI/CD commits from changelog.

```toml
[changelog]
skip_ci = true
```

#### `skip_chore`

**Type**: Boolean (optional)

**Default**: false

Exclude chore commits from changelog.

```toml
[changelog]
skip_chore = true
```

#### `skip_miscellaneous`

**Type**: Boolean (optional)

**Default**: false

Exclude non-conventional commits from changelog.

```toml
[changelog]
skip_miscellaneous = true
```

#### `skip_merge_commits`

**Type**: Boolean (optional)

**Default**: true

Exclude merge commits from changelog.

```toml
[changelog]
skip_merge_commits = false
```

#### `skip_release_commits`

**Type**: Boolean (optional)

**Default**: true

Exclude release commits created by Releasaurus.

```toml
[changelog]
skip_release_commits = false
```

#### `include_author`

**Type**: Boolean (optional)

**Default**: false

Include commit author names in changelog.

```toml
[changelog]
include_author = true
```

#### `skip_shas`

**Type**: Array of strings (optional)

**Default**: None

Skip specific commits by SHA prefix. Use 7+ character prefixes.

```toml
[changelog]
skip_shas = ["abc123d", "def456e"]
```

#### `reword`

**Type**: Array of objects (optional)

**Default**: None

Rewrite commit messages for specific commits. Affects both changelog and
version calculation.

```toml
[[changelog.reword]]
sha = "abc123d"
message = "fix: corrected description"

[[changelog.reword]]
sha = "def456e"
message = "feat: improved feature"
```

#### `body`

**Type**: String (optional)

**Default**: Standard template

Tera template for changelog body. See [Changelog
Configuration](./configuration-changelog.md) for template variables.

```toml
[changelog]
body = """## Release {{ version }}
..."""
```

## Package Section

### `[[package]]`

Define packages in your repository. Can have multiple.

#### `path`

**Type**: String (required)

**Default**: None

Directory path to the package, relative to `workspace_root`.

```toml
[[package]]
path = "."
```

#### `workspace_root`

**Type**: String (optional)

**Default**: "."

Workspace root directory, relative to repository root.

```toml
[[package]]
workspace_root = "backend"
path = "services/api"
```

#### `name`

**Type**: String (optional)

**Default**: Derived from path

Explicit package name. If not set, derived from directory name.

```toml
[[package]]
name = "my-custom-name"
path = "packages/backend"
```

#### `release_type`

**Type**: String (required for version updates)

**Default**: None

Language/framework for version file updates:

- `"generic"` - Changelog/tagging only
- `"go"` - version.go
- `"java"` - pom.xml, build.gradle, build.gradle.kts, gradle.properties,
  gradle/libs.versions.toml
- `"node"` - package.json, lock files
- `"php"` - composer.json, composer.lock
- `"python"` - pyproject.toml, setup.py, setup.cfg
- `"ruby"` - .gemspec, Gemfile
- `"rust"` - Cargo.toml, Cargo.lock

```toml
[[package]]
path = "."
release_type = "node"
```

#### `tag_prefix`

**Type**: String (optional)

**Default**: Derived from package name

Git tag prefix. Defaults:

- Root packages: `"v"`
- Nested packages: `"<name>-v"`

```toml
[[package]]
path = "."
release_type = "rust"
tag_prefix = "v"
```

**Override**: `--tag-prefix` (global) or `--set-package <name>.tag_prefix=<value>`
(per-package) CLI flags

#### `sub_packages`

**Type**: Array of objects (optional)

**Default**: None

Groups multiple packages under a single release that shares one changelog, tag,
and release. Each sub-package gets independent manifest updates based on its
`release_type`.

**Use when:** Multiple packages should always be released together with the same
version and share the same changelog

```toml
[[package]]
name = "platform"
workspace_root = "."
path = "."
sub_packages = [
    { name = "web", path = "packages/web", release_type = "node" },
    { name = "cli", path = "packages/cli", release_type = "rust" }
]
```

See [Grouped Releases](./configuration-monorepo.md#grouped-releases-sub-packages)
for details.

#### `auto_start_next`

**Type**: Boolean (optional)

**Default**: Inherits global setting

Override global `auto_start_next` for this package.

```toml
[[package]]
path = "."
release_type = "node"
auto_start_next = false
```

#### `prerelease`

**Type**: Inline table (optional)

**Default**: Inherits global prerelease

Override global prerelease configuration.

```toml
[[package]]
path = "."
release_type = "rust"
prerelease = { suffix = "beta", strategy = "versioned" }
```

**Override**: `--set-package <name>.prerelease.suffix=<value>` CLI flag

#### `additional_paths`

**Type**: Array of strings (optional)

**Default**: None

Additional directories to monitor for changes.

```toml
[[package]]
path = "packages/api"
release_type = "node"
additional_paths = ["shared/utils", "shared/types"]
```

#### `additional_manifest_files`

**Type**: Array of strings or objects (optional)

**Default**: None

Specifies additional files that should have their version strings updated during
a release. This is useful for:

- Custom version files (e.g., `VERSION`, `version.txt`)
- Documentation files with embedded version numbers
- Configuration files that reference the package version
- Any file with version strings that need to stay in sync

Accepts either simple string paths (recommended) or full configuration objects
with custom regex patterns for advanced use cases.

**Simple format** (recommended for most cases):

```toml
[[package]]
path = "."
release_type = "rust"
additional_manifest_files = ["VERSION", "README.md", "docs/installation.md"]
```

All paths are relative to the package path. The default regex pattern
automatically matches common version formats:

- `version = "1.0.0"`
- `version: "1.0.0"`
- `VERSION='1.0.0'`
- `"version": "1.0.0"`

**Full format** (for custom version patterns):

Use this when your files have non-standard version formats:

```toml
[[package]]
path = "."
release_type = "rust"
additional_manifest_files = [
    { path = "helm/Chart.yaml", version_regex = "appVersion:\\s*\"?(?<version>\\d+\\.\\d+\\.\\d+)\"?" },
    { path = "docker-compose.yml", version_regex = "image:.*:(?<version>\\d+\\.\\d+\\.\\d+)" }
]
```

**Important:** The regex must include a **named capture group** called
`version` to identify which part of the match should be replaced
(e.g., `(?<version>\d+\.\d+\.\d+)`). The surrounding text is automatically
preserved.

**Mixed format** (combine simple and custom):

```toml
[[package]]
path = "."
release_type = "rust"
additional_manifest_files = [
    "VERSION",                    # Uses default regex
    "README.md",                  # Uses default regex
    { path = "config.yml", version_regex = "app_version:\\s*(?<version>\\d+\\.\\d+\\.\\d+)" }
]
```

**Important notes:**

- Custom regex patterns **must** include a named capture group `(?<version>...)`
- Files that don't contain a version pattern are skipped automatically
- Invalid regex patterns will cause an error during configuration resolution
- Only the content within the `version` capture group is replaced
- Paths must be relative to the package path, not the repository root

#### `breaking_always_increment_major`

**Type**: Boolean (optional)

**Default**: Inherits global setting

Override global breaking change behavior.

```toml
[[package]]
path = "."
release_type = "node"
breaking_always_increment_major = false
```

#### `features_always_increment_minor`

**Type**: Boolean (optional)

**Default**: Inherits global setting

Override global feature commit behavior.

```toml
[[package]]
path = "."
release_type = "rust"
features_always_increment_minor = false
```

#### `custom_major_increment_regex`

**Type**: String (optional)

**Default**: Inherits global setting

Override global major version regex.

```toml
[[package]]
path = "."
release_type = "node"
custom_major_increment_regex = "\\[BREAKING\\]"
```

#### `custom_minor_increment_regex`

**Type**: String (optional)

**Default**: Inherits global setting

Override global minor version regex.

```toml
[[package]]
path = "."
release_type = "rust"
custom_minor_increment_regex = "\\[FEATURE\\]"
```

## Complete Example

```toml
# Global settings
base_branch = "main"
first_release_search_depth = 400
separate_pull_requests = false
auto_start_next = false
breaking_always_increment_major = true
features_always_increment_minor = true

# Global prerelease
[prerelease]
suffix = "beta"
strategy = "versioned"

# Changelog customization
[changelog]
skip_ci = true
skip_chore = true
skip_miscellaneous = false
include_author = false

# Package definitions
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

## Next Steps

- [Configuration Overview](./configuration.md) - Getting started guide
- [Prerelease Configuration](./configuration-prerelease.md) - Detailed
  prerelease guide
- [Changelog Configuration](./configuration-changelog.md) - Template
  customization
- [Monorepo Configuration](./configuration-monorepo.md) - Multi-package
  setup
