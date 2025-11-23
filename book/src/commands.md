# Commands

Releasaurus provides two main commands that work together to create a safe,
reviewable release process. This two-stage approach ensures that all changes
are reviewed before publication while automating the tedious aspects of version
management and changelog generation.

**Important**: Releasaurus operates entirely through forge platform APIs
without requiring local repository clones. You can run these commands from any
location with network access to your forge platform.

## Command Overview

### `release-pr`

**Purpose**: Analyze commits, update versions, generate changelog, and create
a pull request

This command does the heavy lifting of release preparation:

- Analyzes commits since the last release
- Determines the appropriate version bump (patch, minor, major)
- Updates version files across your project
- Generates a changelog from commit history
- Creates a pull request with all changes
- Supports prerelease versions (alpha, beta, rc, etc.)
- Supports dry-run mode for testing

### `release`

**Purpose**: Create tags and publish the release after PR approval

This command finalizes the release:

- Validates that you're on a release commit
- Creates a Git tag for the new version
- Pushes the tag to the remote repository
- Creates a release on your forge platform
- Supports prerelease versions (alpha, beta, rc, etc.)
- Supports dry-run mode for testing

## Basic Usage Pattern

The typical Releasaurus workflow follows this pattern (can be run from any
directory):

```bash
# Step 1: Create release preparation PR (run from anywhere)
releasaurus release-pr --github-repo "https://github.com/owner/repo"

# Step 2: Review and merge the PR (done via web interface)

# Step 3: Publish the release (run from anywhere)
releasaurus release --github-repo "https://github.com/owner/repo"
```

**Note**: These commands work by accessing your repository through the forge
API, analyzing commits and files, creating branches with updates, and managing
pull requests—all without requiring a local clone.

## Global Options

All commands support these global options:

### Platform Selection

Choose your Git forge platform by specifying the repository URL:

```bash
# GitHub
--github-repo "https://github.com/owner/repository"

# GitLab
--gitlab-repo "https://gitlab.com/group/project"

# Gitea (or Forgejo)
--gitea-repo "https://git.example.com/owner/repo"
```

### Dry Run Mode

Test your release workflow without making any actual changes to your repository.
Dry-run mode performs all analysis and validation steps while logging what
actions would be taken, but prevents any modifications to your forge platform.

**Note:** Dry-run mode automatically enables debug logging for maximum
visibility into what would happen.

**What dry-run mode does:**

- ✅ Analyzes commit history since the last release
- ✅ Determines version bumps based on conventional commits
- ✅ Generates changelog content
- ✅ Validates configuration and file formats
- ✅ Logs detailed information about what would be created/modified

**What dry-run mode prevents:**

- ❌ Creating or updating branches
- ❌ Creating or updating pull requests
- ❌ Creating Git tags
- ❌ Publishing releases
- ❌ Modifying repository labels

**Usage:**

```bash
# Via command line flag
releasaurus release-pr --dry-run --github-repo "https://github.com/owner/repo"
releasaurus release --dry-run --github-repo "https://github.com/owner/repo"

# Via environment variable
export RELEASAURUS_DRY_RUN=true
releasaurus release-pr --github-repo "https://github.com/owner/repo"
releasaurus release --github-repo "https://github.com/owner/repo"
```

**Output:** Dry-run mode produces detailed debug logs prefixed with `dry_run:`
that show exactly what operations would be performed, including PR titles,
version numbers, file changes, and release notes. Debug mode is automatically
enabled to provide maximum visibility.

### Authentication

Provide access tokens for API authentication:

```bash
# Via command line
--github-token "ghp_xxxxxxxxxxxxxxxxxxxx"
--gitlab-token "glpat_xxxxxxxxxxxxxxxxxxxx"
--gitea-token "xxxxxxxxxxxxxxxxxx"

# Via environment variables (recommended)
export GITHUB_TOKEN="ghp_xxxxxxxxxxxxxxxxxxxx"
export GITLAB_TOKEN="glpat_xxxxxxxxxxxxxxxxxxxx"
export GITEA_TOKEN="xxxxxxxxxxxxxxxxxx"
```

### Debug Logging

Enable detailed logging for troubleshooting:

```bash
# Via command line flag
--debug

# Via environment variable
export RELEASAURUS_DEBUG=true
```

This provides verbose output including:

- Project detection logic
- File modification details
- API request/response information
- Git operations and status

**Note:** Debug mode is automatically enabled when using `--dry-run` or
`RELEASAURUS_DRY_RUN=true`.

See the [Environment Variables](./environment-variables.md#releasaurus_debug) guide for more details on `RELEASAURUS_DEBUG`.

### Prerelease Versions

Both `release-pr` and `release` commands support prerelease versions configured
in your `releasaurus.toml` file:

```toml
# Global prerelease for all packages
prerelease = "alpha"

[[package]]
path = "."
release_type = "node"
```

Or configure per-package:

```toml
[[package]]
path = "./packages/stable"
release_type = "rust"
# No prerelease - stable releases

[[package]]
path = "./packages/experimental"
release_type = "rust"
prerelease = "beta"  # Beta releases for this package
```

**Prerelease Behavior:**

- **Starting**: `v1.0.0` → `v1.1.0-alpha.1` (with feature commit and `prerelease = "alpha"`)
- **Continuing**: `v1.1.0-alpha.1` → `v1.1.0-alpha.2` (same identifier in config)
- **Switching**: `v1.0.0-alpha.3` → `v1.1.0-beta.1` (change `prerelease = "beta"` in config)
- **Graduating**: `v1.0.0-alpha.5` → `v1.0.0` (remove `prerelease` from config)

To change prerelease identifiers or graduate to stable, update your
configuration file and create a new release PR.

See the [Configuration](./configuration.md) guide for complete prerelease
configuration details.

## Platform-Specific Examples

### GitHub

```bash
# With explicit token
releasaurus release-pr \
  --github-repo "https://github.com/myorg/myproject" \
  --github-token "ghp_xxxxxxxxxxxxxxxxxxxx"

# With environment variable
export GITHUB_TOKEN="ghp_xxxxxxxxxxxxxxxxxxxx"
releasaurus release-pr --github-repo "https://github.com/myorg/myproject"
```

### GitLab

```bash
# GitLab.com
releasaurus release-pr \
  --gitlab-repo "https://gitlab.com/mygroup/myproject" \
  --gitlab-token "glpat_xxxxxxxxxxxxxxxxxxxx"

# Self-hosted GitLab
releasaurus release-pr \
  --gitlab-repo "https://gitlab.company.com/team/project" \
  --gitlab-token "glpat_xxxxxxxxxxxxxxxxxxxx"
```

### Gitea

```bash
# Self-hosted Gitea
releasaurus release-pr \
  --gitea-repo "https://git.company.com/team/project" \
  --gitea-token "xxxxxxxxxxxxxxxxxx"

# Works with Forgejo too
releasaurus release-pr \
  --gitea-repo "https://forgejo.example.com/org/repo" \
  --gitea-token "xxxxxxxxxxxxxxxxxx"
```

## Environment Variables

For security and convenience, use environment variables instead of
command-line tokens:

| Variable              | Description                              | Example            |
| --------------------- | ---------------------------------------- | ------------------ |
| `GITHUB_TOKEN`        | GitHub personal access token             | `ghp_xxxxxxxxxxxx` |
| `GITLAB_TOKEN`        | GitLab personal/project access token     | `glpat_xxxxxxxxxx` |
| `GITEA_TOKEN`         | Gitea/Forgejo access token               | `xxxxxxxxxxxxxxxx` |
| `RELEASAURUS_DEBUG`   | Enable debug logging                     | `true`             |
| `RELEASAURUS_DRY_RUN` | Enable dry-run mode (auto-enables debug) | `true`             |

When environment variables are set, you can omit the `--*-token` flags:

```bash
# Set once
export GITHUB_TOKEN="ghp_xxxxxxxxxxxxxxxxxxxx"

# Use in multiple commands
releasaurus release-pr --github-repo "https://github.com/org/repo1"
releasaurus release-pr --github-repo "https://github.com/org/repo2"
```

## Help and Documentation

Get help for any command:

```bash
# General help
releasaurus --help

# Command-specific help
releasaurus release-pr --help
releasaurus release --help

# Version information
releasaurus --version
```

## Next Steps

For integration and automation:

- **[Troubleshooting](./troubleshooting.md)** - Common issues and
  solutions
