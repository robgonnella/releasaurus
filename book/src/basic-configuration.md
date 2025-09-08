# Basic Configuration

Releasaurus is designed to work out-of-the-box with zero configuration for most projects. However, you may want to customize certain aspects of the release process, such as changelog formatting or handling multiple packages within a single repository.

## Do You Need Configuration?

### You DON'T need configuration if:

- You have a single package/project in your repository
- You're happy with the default changelog format
- You're happy with the default tag prefix "v" i.e. `v1.0.0`, `v2.1.0`
- You're using standard project structures for supported languages

### You DO need configuration if:

- You want custom changelog templates or formatting
- You have multiple packages in one repository (monorepo)
- You want custom prefixed tags (like `cli-v1.0.0` or `api-v1.0.0`)
- You need to customize the release process for your team's workflow

## Creating Your First Configuration

If you need configuration, create a file called `releasaurus.toml` in your project's root directory:

```
my-project/
├── releasaurus.toml    # ← Create this file
├── src/
├── README.md
└── ...
```

## Basic Configuration Examples

### Adding Tag Prefixes

The most common customization is adding a prefix to your Git tags:

```toml
# releasaurus.toml
[[package]]
path = "."
tag_prefix = "v"
```

This creates tags like `v1.0.0`, `v1.1.0`, `v2.0.0` instead of `1.0.0`, `1.1.0`, `2.0.0`.

### Custom Changelog Header

Add a custom header to your changelogs:

```toml
# releasaurus.toml
[changelog]
header = "# MyProject Changelog\n\nAll notable changes to MyProject are documented here.\n"

[[package]]
path = "."
tag_prefix = "v"
```

### Simple Multi-Package Setup

For a repository with multiple independently-versioned components:

```toml
# releasaurus.toml
[[package]]
path = "./frontend"
tag_prefix = "frontend-v"

[[package]]
path = "./backend"
tag_prefix = "backend-v"
```

This allows you to release the frontend and backend independently, with tags like:

- `frontend-v1.0.0`, `frontend-v1.1.0`
- `backend-v1.0.0`, `backend-v2.0.0`

## Configuration File Structure

The configuration file has two main sections:

### `[changelog]` Section (Optional)

Controls how changelogs are generated:

```toml
[changelog]
header = "Custom header text"    # Optional
footer = "Custom footer text"    # Optional
# body template is also available for advanced users
```

### `[[package]]` Sections (Required)

Defines packages in your repository. You can have multiple `[[package]]` sections:

```toml
[[package]]
path = "."              # Required: path to package
tag_prefix = "v"        # Optional: tag prefix

[[package]]
path = "./other-package"
tag_prefix = "other-v"
```

## Common Patterns

### Standard Single Project

```toml
# Most common setup
[[package]]
path = "."
tag_prefix = "v"
```

### Workspace with Shared Tag Prefix

```toml
# All packages use same prefix style
[[package]]
path = "./app"
tag_prefix = "v"

[[package]]
path = "./lib"
tag_prefix = "v"
```

### Clear Component Separation

```toml
# Different prefix for each component
[[package]]
path = "./web-app"
tag_prefix = "web-"

[[package]]
path = "./mobile-app"
tag_prefix = "mobile-"

[[package]]
path = "./shared-lib"
tag_prefix = "lib-"
```

## Testing Your Configuration

After creating your configuration file:

1. **Validate syntax**: Run any Releasaurus command with `--debug` to check for configuration errors
2. **Review output**: Check that tag names and changelog format match your expectations

Example validation:

```bash
# This will load and validate your configuration
releasaurus release-pr --github-repo "https://github.com/owner/repo" --debug
```

If there are configuration errors, you'll see clear error messages explaining what needs to be fixed.

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
[changelog]
body = "{% if version -%}
    # [{{ version | trim_start_matches(pat="v") }}]({{ extra.release_link_base }}/{{ version }}) - {{ timestamp | date(format="%Y-%m-%d") }}
{% else -%}
    # [unreleased]
{% endif -%}
{% for group, commits in commits | filter(attribute="merge_commit", value=false) | group_by(attribute="group") %}
    ### {{ group | striptags | trim | upper_first }}
    {% for commit in commits %}
      {% if commit.breaking -%}
        {% if commit.scope %}_({{ commit.scope }})_ {% endif -%}[**breaking**]: {{ commit.message | upper_first }} [_({{ commit.id | truncate(length=8, end="") }})_]({{ extra.commit_link_base }}/{{ commit.id }})
        {% if commit.body -%}
        > {{ commit.body }}
        {% endif -%}
        {% if commit.breaking_description -%}
        > {{ commit.breaking_description }}
        {% endif -%}
      {% else -%}
        - {% if commit.scope %}_({{ commit.scope }})_ {% endif %}{{ commit.message | upper_first }} [_({{ commit.id | truncate(length=8, end="") }})_]({{ extra.commit_link_base }}/{{ commit.id -}})
      {% endif -%}
    {% endfor %}
{% endfor %}"
footer = "Generated by Releasaurus 🦕"

[[package]]
path = "."
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

Remember: start simple and add complexity as needed. The basic patterns above handle most common requirements, and you can always enhance your configuration as your project evolves.
