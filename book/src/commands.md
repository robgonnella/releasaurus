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

### `release`

**Purpose**: Create tags and publish the release after PR approval

This command finalizes the release:

- Validates that you're on a release commit
- Creates a Git tag for the new version
- Pushes the tag to the remote repository
- Creates a release on your forge platform
- Supports prerelease versions (alpha, beta, rc, etc.)

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
--debug
```

This provides verbose output including:

- Project detection logic
- File modification details
- API request/response information
- Git operations and status

### Prerelease Versions

Both `release-pr` and `release` commands support prerelease versions using the `--prerelease` flag:

```bash
# Create an alpha prerelease PR
releasaurus release-pr \
  --github-repo "https://github.com/owner/repo" \
  --prerelease alpha

# Publish the alpha prerelease (after merging PR)
releasaurus release \
  --github-repo "https://github.com/owner/repo" \
  --prerelease alpha

# Create a beta prerelease
releasaurus release-pr \
  --github-repo "https://github.com/owner/repo" \
  --prerelease beta

# Create a release candidate
releasaurus release-pr \
  --github-repo "https://github.com/owner/repo" \
  --prerelease rc
```

**Prerelease Behavior:**

- **Starting**: `v1.0.0` → `v1.1.0-alpha.1` (with feature commit)
- **Continuing**: `v1.1.0-alpha.1` → `v1.1.0-alpha.2` (same identifier)
- **Switching**: `v1.0.0-alpha.3` → `v1.1.0-beta.1` (different identifier)
- **Graduating**: `v1.0.0-alpha.5` → `v1.0.0` (no prerelease flag)

The `--prerelease` flag overrides any prerelease configuration in
`releasaurus.toml`, making it ideal for one-time prerelease versions or
testing different identifiers.

**Important**: When using `--prerelease` with the `release` command, make sure
to use the same identifier that was used when creating the release PR. This
ensures the version analysis produces the same tag that was proposed in the PR.

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

| Variable       | Description                          | Example            |
| -------------- | ------------------------------------ | ------------------ |
| `GITHUB_TOKEN` | GitHub personal access token         | `ghp_xxxxxxxxxxxx` |
| `GITLAB_TOKEN` | GitLab personal/project access token | `glpat_xxxxxxxxxx` |
| `GITEA_TOKEN`  | Gitea/Forgejo access token           | `xxxxxxxxxxxxxxxx` |

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
