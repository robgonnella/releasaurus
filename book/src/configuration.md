# Configuration

Releasaurus works great out-of-the-box with zero configuration, but provides
extensive customization options through an optional `releasaurus.toml`
configuration file. This file allows you to customize changelog generation,
define multiple packages within a repository, and fine-tune the release process
to match your project's specific needs.

## Configuration File Location

Releasaurus looks for a `releasaurus.toml` file in your project's root
directory. If no configuration file is found, sensible defaults are used that
work for most single-package repositories.

```
my-project/
├── releasaurus.toml    # ← Configuration file (optional)
├── src/
├── README.md
└── ...
```

## Configuration Structure

The configuration file uses TOML format with these main sections:

- **`first_release_search_depth`** - (optional, default: 400) Controls commit
  history depth for initial release analysis
- **`separate_pull_requests`** - (optional, default: false) Create separate PRs
  for each package in monorepos
- **`prerelease`** - (optional, default: "") Sets prerelease suffix for all
  defined packages
- **`[changelog]`** - Customizes changelog generation and formatting
  - `body` - (optional) Tera template for changelog content
  - `skip_ci` - (optional, default: false) Exclude CI commits from changelog (optional, default: false)
  - `skip_chore` - (optional, default: false) Exclude chore commits from
    changelog (optional, default: false)
  - `skip_miscellaneous` - (optional, default: false) Exclude non-conventional
    commits from changelog (optional, default: false)
  - `include_author` - (optional, default: false) Include commit author names in
    changelog
- **`[[package]]`** - Defines packages within the repository with their
  release type (can have multiple)
  - `name` - (optional) The name for this package. This will be derived from
    the package path if not provided
  - `path` - (required) The path to the directory for this package relative to the
    repository root
  - `release_type`: (required) The release type for this package, see below for
    options
  - `tag_prefix`: (optional) The tag prefix to use for this package
  - `prerelease`: (optional) The prerelease suffix to use for this package

## Default Configuration

This is the default configuration that is used if there is no specific user
defined configuration provided.

```toml
# releasaurus.toml using default configuration

# Maximum commits to analyze for first release (default: 400)
first_release_search_depth = 400

# Create separate pull requests for each package (default: false)
separate_pull_requests = false

[changelog]
body = """# [{{ version  }}]({{ link }}) - {{ timestamp | date(format="%Y-%m-%d") }}
{% for group, commits in commits | filter(attribute="merge_commit", value=false) | group_by(attribute="group") %}
### {{ group | striptags | trim }}
{% for commit in commits %}
{% if commit.breaking -%}
{% if commit.scope %}_({{ commit.scope }})_ {% endif -%}[**breaking**]: {{ commit.message }} [_({{ commit.id | truncate(length=8, end="") }})_]({{ commit.link }})
{% if commit.body -%}
> {{ commit.body }}
{% endif -%}
{% if commit.breaking_description -%}
> {{ commit.breaking_description }}
{% endif -%}
{% else -%}
- {% if commit.scope %}_({{ commit.scope }})_ {% endif %}{{ commit.message }} [_({{ commit.id | truncate(length=8, end="") }})_]({{ commit.link }})
{% endif -%}
{% endfor %}
{% endfor %}"""

[[package]]
path = "."
release_type = "node"
tag_prefix = "v"
```

## First Release Search Depth

The `first_release_search_depth` setting controls how many commits Releasaurus
will analyze when determining the version for the first release (when no
previous tags exist).

```toml
# Optional: defaults to 400 if not specified
first_release_search_depth = 400
```

**When to adjust this value:**

- **Large repositories**: Decrease to `100` or `200` for faster analysis
- **Need comprehensive history**: Increase to `1000` or higher to analyze more commits
- **CI/CD environments**: Use smaller values like `100` for faster builds

**Note**: This setting only affects the first release. Once a tag exists,
subsequent releases automatically find all commits since the last tag, making
this setting unnecessary for ongoing releases.

## Separate Pull Requests

The `separate_pull_requests` setting controls whether Releasaurus creates a
single combined pull request for all packages or separate pull requests for
each package in a monorepo.

```toml
# Optional: defaults to false if not specified
separate_pull_requests = true
```

### Single PR Mode (Default)

When `separate_pull_requests` is `false` or not specified, Releasaurus creates
one pull request containing all package updates:

```toml
separate_pull_requests = false  # or omit this line

[[package]]
path = "./apps/frontend"
release_type = "node"

[[package]]
path = "./apps/backend"
release_type = "rust"
```

**Benefits:**

- Single review process for all changes
- All packages released together atomically
- Simpler workflow for tightly coupled packages

**Best for:**

- Packages that are always released together
- Small monorepos with few packages
- Teams that prefer coordinated releases

### Separate PR Mode

When `separate_pull_requests` is `true`, Releasaurus creates individual pull
requests for each package that has changes:

```toml
separate_pull_requests = true

[[package]]
path = "./apps/frontend"
release_type = "node"
tag_prefix = "frontend-v"

[[package]]
path = "./apps/backend"
release_type = "rust"
tag_prefix = "backend-v"
```

**Benefits:**

- Independent release cycles for each package
- Parallel review and merging of changes
- Teams can release packages at different cadences
- Easier to track changes per package

**Best for:**

- Large monorepos with many packages
- Independently versioned packages
- Packages maintained by different teams
- When packages have different release schedules

### Branch Naming

The branch names created differ based on this setting:

**Single PR mode:**

```
releasaurus-release-main
```

**Separate PR mode:**

```
releasaurus-release-main-frontend
releasaurus-release-main-backend
releasaurus-release-main-shared
```

### Example: Independent Package Releases

This example shows a monorepo where packages can be released independently:

```toml
# Allow independent releases
separate_pull_requests = true

# Frontend app - released frequently
[[package]]
path = "./apps/web"
release_type = "node"
tag_prefix = "web-v"

# Mobile app - released independently
[[package]]
path = "./apps/mobile"
release_type = "node"
tag_prefix = "mobile-v"

# Core library - released less frequently
[[package]]
path = "./packages/core"
release_type = "rust"
tag_prefix = "core-v"

# CLI tool - released on demand
[[package]]
path = "./packages/cli"
release_type = "rust"
tag_prefix = "cli-v"
```

With this configuration:

- Each package gets its own pull request when it has changes
- Teams can merge and release packages independently
- No need to coordinate releases across all packages
- Each package maintains its own version history

## Changelog Configuration

The `[changelog]` section allows you to customize how changelogs are generated
using [Tera](https://keats.github.io/tera/) templating engine.

### Commit Filtering Options

Control which commit types are included in your changelog:

#### `skip_ci` (Optional)

Excludes CI/CD related commits from the changelog. When set to `true`, commits
with the `ci:` type will not appear in generated changelogs.

```toml
[changelog]
skip_ci = true  # Exclude commits like "ci: update workflow"
```

**Default**: `false`

#### `skip_chore` (Optional)

Excludes chore commits from the changelog. When set to `true`, commits with
the `chore:` type will not appear in generated changelogs.

```toml
[changelog]
skip_chore = true  # Exclude commits like "chore: update dependencies"
```

**Default**: `false`

#### `skip_miscellaneous` (Optional)

Excludes non-conventional commits from the changelog. When set to `true`,
commits that don't follow the conventional commit format will not appear in
generated changelogs.

```toml
[changelog]
skip_miscellaneous = true  # Exclude commits without a type prefix
```

**Default**: `false`

**Example use case**: Use this option to keep your changelog focused on
conventional commits only, filtering out commits that don't follow the
`type: description` format.

#### `include_author` (Optional)

Includes the commit author's name in the changelog entries. When set to `true`,
the `author_name` field will be used in the default `body` template that
generates the changelog. If you wish to use this field in your own custom
template, you can access it as part of the commit object `commit.author_name`
or `commit.author_email`.

```toml
[changelog]
include_author = true  # Show author names like "feat: add feature <John Doe>"
```

**Default**: `false`

### Available Template

#### `body` (Required)

The main changelog content template. This defines how each release section is
formatted.

## Package Configuration

Each `[[package]]` section defines a package in your repository and requires
specifying the `release_type` for version file management.

### `path`

The directory path to the package, relative to the `workspace_root` path
(or relative to the repository root if `workspace_root` is not specified).

```toml
[[package]]
path = "."
```

```toml
[[package]]
workspace_root = "rust-workspace"
path = "packages/api"  # Relative to rust-workspace directory
```

### `workspace_root` (Optional)

The directory path to the workspace root for this package, relative to the
repository root. This allows you to define workspace packages that are in
subdirectories of the repository.

```toml
[[package]]
workspace_root = "."  # Default: repository root
path = "packages/api"
release_type = "rust"
```

**When to use:**

- **Multi-workspace repositories**: When you have multiple independent
  workspaces in one repository i.e. A rust workspace in a subdirectory

**Default**: If not specified, defaults to `"."` (the repository root).

**How it works:**

The `workspace_root` and `path` are combined to locate package files:

- Version files are located at: `workspace_root + path + <version-file>`
- Workspace-level files are located at: `workspace_root + <workspace-file>`

**Example: Rust workspace in subdirectory**

```toml
[[package]]
name = "api-server-1"
workspace_root = "backend"  # Workspace is in backend/ directory
path = "packages/api1"      # Package is in backend/packages/api1/
release_type = "rust"
tag_prefix = "api1-v"

name = "api-server-2"
workspace_root = "backend"  # Workspace is in backend/ directory
path = "packages/api2"      # Package is in backend/packages/api2/
release_type = "rust"
tag_prefix = "api2-v"
```

This configuration will:

- Update `backend/packages/api1/Cargo.toml`
- Update `backend/packages/api2/Cargo.toml`
- Update `backend/Cargo.lock` (workspace-level lock file)
- Update `backend/packages/api1/Cargo.lock` (if it exists)
- Update `backend/packages/api2/Cargo.lock` (if it exists)

**Example: Multiple workspaces in one repository**

```toml
# Rust workspace
[[package]]
name = "rust-core"
workspace_root = "rust-workspace"
path = "."
release_type = "rust"
tag_prefix = "rust-v"

# Node workspace
[[package]]
name = "web-app"
workspace_root = "node-workspace"
path = "packages/web"
release_type = "node"
tag_prefix = "web-v"
```

### `name` (Optional)

An explicit name for the package. If not specified, Releasaurus automatically
derives the package name from the `path`:

- For root packages (`path = "."`), uses the repository name
- For nested packages, uses the directory name (e.g., `"packages/api"` becomes `"api"`)

```toml
[[package]]
name = "my-custom-name"  # Optional: override derived name
path = "packages/backend"
release_type = "rust"
```

**When to use:**

- Override the automatically derived package name
- Provide a more descriptive name for releases and tags
- Maintain consistent naming when package directories change

**Note**: The package name is used in PR titles, branch names, and when
generating tag prefixes in monorepos.

**Example: Custom naming for clearer releases**

```toml
# Without explicit names, these would derive as "api" and "web"
[[package]]
name = "backend-api"      # Clearer than just "api"
path = "services/api"
release_type = "rust"
tag_prefix = "backend-api-v"

[[package]]
name = "frontend-web"     # Clearer than just "web"
path = "apps/web"
release_type = "node"
tag_prefix = "frontend-web-v"
```

### `release_type`

Specifies which language/framework updater to use for version files. This is
**required** for version file updates.

**Available options:**

- **`"Rust"`** - Updates `Cargo.toml` and `Cargo.lock`
- **`"Node"`** - Updates `package.json`, `package-lock.json`, `yarn.lock`
- **`"Python"`** - Updates `pyproject.toml`, `setup.py`, `setup.cfg`
- **`"Java"`** - Updates `pom.xml` or `build.gradle`
- **`"Php"`** - Updates `composer.json`
- **`"Ruby"`** - Updates gemspec files and version files
- **`"Generic"`** - Changelog and tagging only (no version file updates)

```toml
[[package]]
path = "."
release_type = "node"
```

### `tag_prefix`

Optional prefix for Git tags. Defaults to `"v"` if not specified.

```toml
[[package]]
path = "."
release_type = "rust"
tag_prefix = "v"  # Creates tags like v1.0.0, v1.1.0
```

### `prerelease`

Optional prerelease identifier for creating pre-release versions
(e.g., alpha, beta, rc). Can be configured globally or per-package, and can be
overridden via CLI flag.

**Configuration Priority:** CLI flag > Package config > Global config

#### Global Prerelease Configuration

Set a prerelease identifier for all packages:

```toml
# All packages will use alpha prereleases
prerelease = "alpha"

[[package]]
path = "."
release_type = "node"
```

With this configuration, version `1.0.0` would become `1.1.0-alpha.1` for a
feature commit.

#### Per-Package Prerelease Configuration

Override the global setting for specific packages:

```toml
# Global default is beta
prerelease = "beta"

[[package]]
path = "./apps/web"
release_type = "node"
# Uses global beta prerelease

[[package]]
path = "./apps/api"
release_type = "rust"
prerelease = "rc"  # Override: this package uses rc instead
```

#### CLI Override

Override configuration with the `--prerelease` flag:

```bash
# Override config file settings for this release
releasaurus release-pr --prerelease alpha
```

#### Prerelease Version Behavior

**Starting a Prerelease:**

- Current: `v1.0.0`
- With `feat:` commit and `prerelease = "alpha"`
- Result: `v1.1.0-alpha.1`

**Continuing a Prerelease:**

- Current: `v1.1.0-alpha.1`
- With `fix:` commit and `prerelease = "alpha"`
- Result: `v1.1.0-alpha.2`

**Switching Prerelease Identifier:**

- Current: `v1.0.0-alpha.3`
- With `feat:` commit and `prerelease = "beta"`
- Result: `v1.1.0-beta.1` (calculates next version and switches identifier)

**Graduating to Stable:**

- Current: `v1.0.0-alpha.5`
- With `fix:` commit and no prerelease configured
- Result: `v1.0.0` (removes prerelease suffix)

#### Common Prerelease Identifiers

- **`alpha`** - Early testing phase, expect significant changes
- **`beta`** - Feature complete, testing and bug fixes
- **`rc`** - Release candidate, final testing before stable release
- **`preview`** - Preview release for gathering feedback
- **`dev`** - Development/nightly builds

#### Example: Monorepo with Mixed Prerelease States

```toml
# Most packages are stable
separate_pull_requests = true

[[package]]
path = "./packages/core"
release_type = "rust"
tag_prefix = "core-v"
# No prerelease - stable releases only

[[package]]
path = "./packages/experimental"
release_type = "rust"
tag_prefix = "experimental-v"
prerelease = "alpha"  # Experimental features in alpha

[[package]]
path = "./apps/web"
release_type = "node"
tag_prefix = "web-v"
prerelease = "beta"  # Web app in beta testing
```

## Changelog Body Template Variables

The variables / fields available in the tera template to construct the
changelog for a release are as follows:

- **version** - The semantic version string (e.g., "1.2.3")
- **link** - URL link to the release
- **sha** - Git commit SHA for the release
- **timestamp** - Unix timestamp of the release
- **include_author** - Boolean flag indicating if author names should be included
- **commits**: `List<Commit>` - Array of commit objects with the following fields:
  - **id** - Commit SHA
  - **group** - Commit category (e.g., "Features", "Bug Fixes", "Chore", "CI/CD")
  - **scope** - Optional scope from conventional commit (e.g., "api", "ui")
  - **message** - Commit description
  - **body** - Optional extended commit body
  - **link** - URL link to the commit
  - **breaking** - Boolean indicating if this is a breaking change
  - **breaking_description** - Optional description of breaking changes
  - **merge_commit** - Boolean indicating if this is a merge commit
  - **timestamp** - Unix timestamp of the commit
  - **author_name** - Name of the commit author
  - **author_email** - Email of the commit author
  - **raw_message** - Original unprocessed commit message

### Using the `include_author` Flag in Templates

The `include_author` flag can be used in conditional statements within your
template to show or hide author information:

```
{% if include_author %} <{{ commit.author_name }}>{% endif %}
```

This allows you to control whether author names appear in the changelog without
modifying the template when you toggle the `include_author` configuration option.

### Monorepo with Multiple Packages

For monorepos, you can choose between coordinated releases (single PR) or
independent releases (separate PRs):

#### Coordinated Releases (Single PR)

```toml
# All packages released together in one PR (default)
separate_pull_requests = false

[[package]]
path = "./apps/frontend"
release_type = "node"
tag_prefix = "frontend-v"

[[package]]
path = "./apps/api"
release_type = "rust"
tag_prefix = "api-v"

[[package]]
path = "./packages/shared"
release_type = "python"
tag_prefix = "shared-v"
```

#### Independent Releases (Separate PRs)

```toml
# Each package gets its own PR for independent releases
separate_pull_requests = true

[[package]]
path = "./apps/frontend"
release_type = "node"
tag_prefix = "frontend-v"

[[package]]
path = "./apps/api"
release_type = "rust"
tag_prefix = "api-v"

[[package]]
path = "./packages/shared"
release_type = "python"
tag_prefix = "shared-v"

[[package]]
path = "./packages/cli"
release_type = "rust"
tag_prefix = "cli-v"
```

## Next Steps

- Check [Environment Variables](./environment-variables.md) for runtime
  configuration options
- Review [Troubleshooting](./troubleshooting.md) for common configuration
  issues
