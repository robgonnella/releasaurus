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

## Template Customization

### `body` Template

The main changelog content template using Tera syntax.

**Default template** creates entries starting with `# [version](link) -
date`.

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
- `link` - URL to the release
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
