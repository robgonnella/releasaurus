# Changelog Configuration

Customize changelog generation, filter commits, and format output with
Tera templates.

## Overview

Releasaurus automatically generates changelogs from commit history using
conventional commits. Customize what's included and how it's formatted
through the `[changelog]` section.

## Quick Configuration

### Filter Out Noise

Exclude CI, chore, and non-conventional commits:

```toml
[changelog]
skip_ci = true
skip_chore = true
skip_miscellaneous = true

[[package]]
path = "."
release_type = "node"
```

### Include Author Names

Show who made each commit:

```toml
[changelog]
include_author = true

[[package]]
path = "."
release_type = "rust"
```

## Commit Filtering Options

### `skip_ci` (default: false)

Exclude CI/CD related commits:

```toml
[changelog]
skip_ci = true  # Excludes "ci: update workflow"
```

### `skip_chore` (default: false)

Exclude chore commits:

```toml
[changelog]
skip_chore = true  # Excludes "chore: update deps"
```

### `skip_miscellaneous` (default: false)

Exclude non-conventional commits:

```toml
[changelog]
skip_miscellaneous = true  # Excludes commits without type prefix
```

### `skip_merge_commits` (default: true)

Exclude merge commits:

```toml
[changelog]
skip_merge_commits = false  # Include "Merge pull request #123"
```

### `skip_release_commits` (default: true)

Exclude release commits created by Releasaurus:

```toml
[changelog]
skip_release_commits = false  # Include "chore(main): release v1.0.0"
```

### `include_author` (default: false)

Include commit author names in changelog entries:

```toml
[changelog]
include_author = true
```

### `skip_shas` (default: none)

Skip specific commits by SHA prefix:

```toml
[changelog]
skip_shas = ["abc123d", "def456e"]

[[package]]
path = "."
release_type = "rust"
```

Use short SHA prefixes (7+ characters). Useful for excluding commits that
shouldn't affect versioning or appear in changelogs.

**CLI override:**

```bash
releasaurus release-pr --skip-sha "abc123d"
```

### `reword`

Rewrite commit messages in the changelog:

```toml
[[changelog.reword]]
sha = "abc123d"
message = "fix: corrected security vulnerability"

[[changelog.reword]]
sha = "def456e"
message = "feat: added user authentication"

[[package]]
path = "."
release_type = "node"
```

Use SHA prefixes to match commits. The reworded message affects both
changelog content and version calculation (e.g., changing `fix:` to `feat:`
bumps minor instead of patch).

**CLI override:**

```bash
releasaurus release-pr --skip-sha "abc123d" \
  --reword "def456e=feat: improved feature"
```

## Template Customization

### `body` Template

The main changelog content template using Tera syntax.

**Default template**

```toml
[changelog]
body = """# [{{ version  }}]{% if tag_compare_link %}({{ tag_compare_link }}){% else %}({{ link }}){% endif %} - {{ timestamp | date(format="%Y-%m-%d") }}
{% for group, commits in commits | filter(attribute="merge_commit", value=false) | sort(attribute="group") | group_by(attribute="group") %}
### {{ group | striptags | trim }}
{% for commit in commits %}
{% if commit.breaking -%}
{% if commit.scope %}_({{ commit.scope }})_ {% endif -%}[**breaking**]: {{ commit.title }} [_({{ commit.short_id }})_]({{ commit.link }}){% if include_author %} ({{ commit.author_name }}){% endif %}
{% if commit.body -%}
> {{ commit.body }}
{% endif -%}
{% if commit.breaking_description -%}
> {{ commit.breaking_description }}
{% endif -%}
{% else -%}
- {% if commit.scope %}_({{ commit.scope }})_ {% endif %}{{ commit.title }} [_({{ commit.short_id }})_]({{ commit.link }}){% if include_author %} ({{ commit.author_name }}){% endif %}
{% endif -%}
{% endfor %}
{% endfor %}"""
```

**Custom template example:**

```toml
[changelog]
body = """## Release v{{ version }}

Released on {{ timestamp | date(format="%Y-%m-%d") }}

{% for group, commits in commits | group_by(attribute="group") %}
### {{ group }}
{% for commit in commits %}
- {{ commit.message }} ({{ commit.short_id }})
{% endfor %}
{% endfor %}"""
```

## Template Variables

Available in the `body` template:

### Release Variables

- `version` - Semantic version string (e.g., "1.2.3")
- `tag_name` - Tag for this release including and prefixes and suffixes
- `link` - URL to the release
- `tag_compare_link` - URL to diff between this release tag and the previous
  release tag. Empty for the first release (use conditional rendering)
- `sha_compare_link` - URL to diff between this release commit SHA and the
  previous release tag. Empty for the first release
- `sha` - Git commit SHA
- `timestamp` - Unix timestamp
- `include_author` - Boolean flag for author names

### Commit Variables

The `commits` array contains objects with:

- `id` - Full commit SHA
- `short_id` - Abbreviated SHA
- `group` - Category (Features, Bug Fixes, etc.)
- `scope` - Optional scope from conventional commit
- `title` - Commit message without type/scope
- `body` - Optional extended description
- `link` - URL to the commit
- `breaking` - Boolean for breaking changes
- `breaking_description` - Breaking change details
- `merge_commit` - Boolean for merge commits
- `timestamp` - Commit timestamp
- `author_name` - Commit author name
- `author_email` - Author email
- `raw_title` - Original unprocessed title
- `raw_message` - Original full message

## Practical Examples

### Clean User-Facing Changelog

Focus only on features and fixes:

```toml
[changelog]
skip_ci = true
skip_chore = true
skip_miscellaneous = true
skip_merge_commits = true
skip_release_commits = true

[[package]]
path = "."
release_type = "node"
```

### Comprehensive Changelog

Include everything except merges:

```toml
[changelog]
skip_ci = false
skip_chore = false
skip_miscellaneous = false
skip_merge_commits = true
include_author = true

[[package]]
path = "."
release_type = "rust"
```

### Custom Template with Authors

```toml
[changelog]
include_author = true
body = """# {{ version }} - {{ timestamp | date(format="%Y-%m-%d") }}

{% for group, commits in commits | group_by(attribute="group") %}
## {{ group }}
{% for commit in commits %}
- {{ commit.title }}{% if include_author %} by {{
commit.author_name }}{% endif %}
{% endfor %}
{% endfor %}"""

[[package]]
path = "."
release_type = "python"
```

## Template Tips

### Conditional Author Display

```
{% if include_author %} <{{ commit.author_name }}>{% endif %}
```

### Filter Merge Commits

```
{% for commit in commits | filter(attribute="merge_commit",
value=false) %}
```

### Group by Category

```
{% for group, commits in commits | group_by(attribute="group") %}
### {{ group }}
{% endfor %}
```

### Highlight Breaking Changes

```
{% if commit.breaking %}
**BREAKING**: {{ commit.message }}
{% if commit.breaking_description %}
> {{ commit.breaking_description }}
{% endif %}
{% endif %}
```

## Testing Templates

Test your template locally before committing changes:

```bash
# See generated changelog with your template
releasaurus release-pr --forge local --repo "."
```

Review the output to verify formatting looks correct.

## Tera Template Resources

Releasaurus uses the [Tera](https://keats.github.io/tera/) templating
engine. See the Tera documentation for advanced filtering and
formatting options.

## Next Steps

- [Configuration Overview](./configuration.md) - Main configuration
  guide
- [Prerelease Configuration](./configuration-prerelease.md) -
  Alpha/beta releases
- [Configuration Reference](./configuration-reference.md) - All options
