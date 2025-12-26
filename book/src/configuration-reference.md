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
- `"rust"` - Cargo.toml, Cargo.lock
- `"node"` - package.json, lock files
- `"python"` - pyproject.toml, setup.py, setup.cfg
- `"java"` - pom.xml, build.gradle
- `"php"` - composer.json
- `"ruby"` - .gemspec, Gemfile
- `"generic"` - Changelog/tagging only

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

**Type**: Array of strings (optional)

**Default**: None

Additional files to receive generic version updates.

```toml
[[package]]
path = "."
release_type = "rust"
additional_manifest_files = ["VERSION", "README.md"]
```

Files updated with regex pattern matching `version = "x.y.z"`.

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
