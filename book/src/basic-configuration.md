# Basic Configuration

Releasaurus is designed to work out-of-the-box with zero configuration for
most projects. However, you may want to customize certain aspects of the
release process, such as changelog formatting or handling multiple packages
within a single repository.

## Do You Need Configuration?

### You DON'T need configuration if:

- You only need changelog generation and tagging (no version file updates)
- You're happy with the default changelog format
- You're happy with the default tag prefix "v" (e.g., `v1.0.0`, `v2.1.0`)

### You DO need configuration if:

- You want version file updates (requires specifying `release_type`)
- You want custom changelog templates or formatting
- You have multiple packages in one repository (monorepo)
- You want custom prefixed tags (e.g., `cli-v1.0.0` or `api-v1.0.0`)

## Creating Your First Configuration

If you need configuration, create a file called `releasaurus.toml` in your
project's root directory:

```
my-project/
├── releasaurus.toml    # ← Create this file
├── src/
├── README.md
└── ...
```

## Basic Configuration Examples

### Single Package with Version Updates

The most common setup specifies the release type for version file updates:

```toml
# releasaurus.toml
[[package]]
path = "."
release_type = "Node"  # Options: "Rust", "Node", "Python", "Java", "Php", "Ruby", "Generic"
tag_prefix = "v"
```

This creates tags like `v1.0.0`, `v1.1.0`, `v2.0.0` instead of `1.0.0`,
`1.1.0`, `2.0.0`.

### Simple Multi-Package Setup

For a repository with multiple independently-versioned components:

```toml
# releasaurus.toml
[[package]]
path = "./frontend"
release_type = "Node"
tag_prefix = "frontend-v"

[[package]]
path = "./backend"
release_type = "Rust"
tag_prefix = "backend-v"
```

This allows you to release the frontend and backend independently, with tags like:

- `frontend-v1.0.0`, `frontend-v1.1.0`
- `backend-v1.0.0`, `backend-v2.0.0`

## Configuration File Structure

The configuration file has these main components:

### `first_release_search_depth` (Optional)

Controls how many commits to analyze for the first release:

```toml
# Optional: defaults to 400 if not specified
# Set to 0 to analyze entire history for 1st release
first_release_search_depth = 400
```

### `[changelog]` Section (Optional)

Controls how changelogs are generated and which commits to include:

```toml
[changelog]
skip_ci = false              # Optional: exclude CI commits from changelog
skip_chore = false           # Optional: exclude chore commits from changelog
skip_miscellaneous = false   # Optional: exclude non-conventional commits
include_author = false       # Optional: show commit author names
# body template is available for advanced users
```

**Common changelog options:**

- `skip_ci`: Exclude CI/CD commits (e.g., "ci: update workflow")
- `skip_chore`: Exclude maintenance commits (e.g., "chore: update deps")
- `skip_miscellaneous`: Exclude commits without conventional type prefixes
- `include_author`: Add author names to changelog entries

### `[[package]]` Sections (Required)

Defines packages in your repository. You can have multiple `[[package]]` sections:

```toml
[[package]]
path = "."              # Required: path to package
release_type = "Node"   # Required: language/framework type
tag_prefix = "v"        # Optional: tag prefix

[[package]]
path = "./other-package"
release_type = "Rust"
tag_prefix = "other-v"
```

## Common Patterns

### Standard Single Project

```toml
# Most common setup
[[package]]
path = "."
release_type = "Node"
tag_prefix = "v"
```

### With Custom Search Depth

```toml
# For large repositories, limit initial commit analysis
first_release_search_depth = 200

[[package]]
path = "."
release_type = "Python"
tag_prefix = "v"
```

### Workspace with Shared Tag Prefix

```toml
# All packages use same prefix style
[[package]]
path = "./app"
release_type = "Node"
tag_prefix = "v"

[[package]]
path = "./lib"
release_type = "Rust"
tag_prefix = "v"
```

### Clear Component Separation

```toml
# Different prefix for each component
[[package]]
path = "./web-app"
release_type = "Node"
tag_prefix = "web-"

[[package]]
path = "./mobile-app"
release_type = "Node"
tag_prefix = "mobile-"

[[package]]
path = "./shared-lib"
release_type = "Rust"
tag_prefix = "lib-"
```

### Clean Changelog (Filtered Commits)

```toml
# Focus on user-facing changes only
[changelog]
skip_ci = true
skip_chore = true
skip_miscellaneous = true

[[package]]
path = "."
release_type = "Rust"
tag_prefix = "v"
```

### Changelog with all commits and author attribution

```toml
# Show who contributed each change
[changelog]
skip_ci = false
skip_chore = false
skip_miscellaneous = false
include_author = true

[[package]]
path = "."
release_type = "Python"
tag_prefix = "v"
```

## Testing Your Configuration

After creating your configuration file:

1. **Validate syntax**: Run any Releasaurus command with `--debug` to check
   for configuration errors
2. **Review output**: Check that tag names and changelog format match your
   expectations

Example validation:

```bash
# This will load and validate your configuration
releasaurus release-pr \
  --github-repo "https://github.com/owner/repo" \
  --debug
```

If there are configuration errors, you'll see clear error messages explaining
what needs to be fixed.

## Configuration Loading

Releasaurus looks for `releasaurus.toml` in your project root. If found:

- ✅ Your configuration is loaded and used
- ❌ Any errors will stop execution with helpful messages

If not found:

- ✅ Default configuration is used automatically
- ✅ Everything works with sensible defaults

## Default Values

When you don't specify values, these defaults are used:

```toml
# Implicit defaults (you don't need to write these)
first_release_search_depth = 400

[changelog]
skip_ci = false
skip_chore = false
skip_miscellaneous = false
include_author = false
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
release_type = "Node"
tag_prefix = "v"
```

## Environment-Specific Configuration

While the configuration file handles project settings, environment-specific settings use environment variables:

```bash
# Authentication tokens
export GITHUB_TOKEN="ghp_xxxxxxxxxxxx"
export GITLAB_TOKEN="glpat_xxxxxxxxxxxx"
export GITEA_TOKEN="xxxxxxxxxxxx"
```

These don't go in the configuration file for security reasons.

## Getting Help

If you're having trouble with configuration:

1. **Use debug mode**: Add `--debug` to see detailed configuration loading
2. **Start simple**: Begin with just tag prefixes, add complexity gradually
3. **Check examples**: Review the patterns above for similar use cases
4. **Validate early**: Test configuration changes before committing

## Next Steps

Once you have basic configuration working:

- **[Configuration](./configuration.md)** - Complete configuration reference
- **[Commands](./commands.md)** - Using Releasaurus with your configuration

Remember: start simple and add complexity as needed. The basic patterns above
handle most common requirements, and you can always enhance your configuration
as your project evolves.
