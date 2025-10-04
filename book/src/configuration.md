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

- **`first_release_search_depth`** - Controls commit history depth for initial
  release analysis (optional)
- **`[changelog]`** - Customizes changelog generation and formatting
  (optional)
  - `body` - Tera template for changelog content
  - `skip_ci` - Exclude CI commits from changelog (optional, default: false)
  - `skip_chore` - Exclude chore commits from changelog (optional, default: false)
  - `skip_miscellaneous` - Exclude non-conventional commits from changelog (optional, default: false)
  - `include_author` - Include commit author names in changelog (optional, default: false)
- **`[[package]]`** - Defines packages within the repository with their
  release type (required, can have multiple)

## Default Configuration

This is the default configuration that is used if there is no specific user
defined configuration provided.

```toml
# releasaurus.toml using default configuration

# Maximum commits to analyze for first release (default: 400)
first_release_search_depth = 400

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
release_type = "Node"  # Options: "Rust", "Node", "Python", "Java", "Php", "Ruby", "Generic"
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

The directory path to the package, relative to the repository root.

```toml
[[package]]
path = "."  # Root of repository
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
release_type = "Node"
```

### `tag_prefix`

Optional prefix for Git tags. Defaults to `"v"` if not specified.

```toml
[[package]]
path = "."
release_type = "Rust"
tag_prefix = "v"  # Creates tags like v1.0.0, v1.1.0
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

```toml
# Frontend package
[[package]]
path = "./apps/frontend"
release_type = "Node"
tag_prefix = "frontend-v"

# Backend API
[[package]]
path = "./apps/api"
release_type = "Rust"
tag_prefix = "api-v"

# Shared library
[[package]]
path = "./packages/shared"
release_type = "Python"
tag_prefix = "shared-v"

# CLI tool
[[package]]
path = "./packages/cli"
release_type = "Rust"
tag_prefix = "cli-v"
```

## Next Steps

- Check [Environment Variables](./environment-variables.md) for runtime
  configuration options
- Review [Troubleshooting](./troubleshooting.md) for common configuration
  issues
