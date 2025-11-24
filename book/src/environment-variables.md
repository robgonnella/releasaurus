# Environment Variables

Environment variables provide a secure and flexible way to configure
Releasaurus without hardcoding sensitive information or platform-specific
settings. This reference covers all supported environment variables and their
usage patterns.

## Authentication Tokens

### Primary Token Variables

#### `GITHUB_TOKEN`

**Purpose**: Authentication token for GitHub API access

**Required Scopes**:

- `repo` (for private repositories)
- `public_repo` (for public repositories)
- `write:packages` (if publishing packages)

**Example**:

```bash
export GITHUB_TOKEN="ghp_xxxxxxxxxxxxxxxxxxxx"
```

**Usage**:

```bash
# With environment variable set
releasaurus release-pr --github-repo "https://github.com/owner/repo"

# Without environment variable (less secure)
releasaurus release-pr \
  --github-repo "https://github.com/owner/repo" \
  --github-token "ghp_xxxxxxxxxxxxxxxxxxxx"
```

#### `GITLAB_TOKEN`

**Purpose**: Authentication token for GitLab API access

**Required Scopes**:

- `api` (full API access)
- `read_repository` (repository read access)
- `write_repository` (repository write access)

**Token Types**:

- Personal Access Tokens

**Example**:

```bash
export GITLAB_TOKEN="glpat_xxxxxxxxxxxxxxxxxxxx"
```

**Usage**:

```bash
# GitLab.com
releasaurus release-pr --gitlab-repo "https://gitlab.com/group/project"

# Self-hosted GitLab
releasaurus release-pr --gitlab-repo "https://gitlab.company.com/team/repo"
```

#### `GITEA_TOKEN`

**Purpose**: Authentication token for Gitea/Forge API access

**Required Permissions**:

- Repository read/write access
- Issue/PR management permissions

**Example**:

```bash
export GITEA_TOKEN="xxxxxxxxxxxxxxxxxx"
```

**Usage**:

```bash
# Self-hosted Gitea
releasaurus release-pr --gitea-repo "https://git.company.com/org/repo"

# Forge instance
releasaurus release-pr --gitea-repo "https://forgejo.example.com/user/project"
```

## Debug Configuration

### `RELEASAURUS_DEBUG`

**Purpose**: Enable detailed debug logging for troubleshooting

**Values**:

- Any value (including `true`, `false`, `1`, `0`, etc.) - Enable debug mode
- Unset or empty - Disable debug mode (default)

**Example**:

```bash
export RELEASAURUS_DEBUG=true
```

**Usage**:

```bash
# Enable debug mode via environment variable
export RELEASAURUS_DEBUG=true
releasaurus release-pr --github-repo "https://github.com/owner/repo"

# Alternative: use the --debug flag
releasaurus release-pr --github-repo "https://github.com/owner/repo" --debug
```

**Note**: Debug mode is enabled whenever `RELEASAURUS_DEBUG` is set to any
value. To disable debug mode, the variable must be unset or empty. The `--debug`
flag will always enable debug mode regardless of the environment variable value.

### `RELEASAURUS_DRY_RUN`

**Purpose**: Enable dry-run mode to test release workflows without making actual
changes

**Values**:

- Any value (including `true`, `false`, `1`, `0`, etc.) - Enable dry-run mode
- Unset or empty - Disable dry-run mode (default)

**Behavior**:

- Performs all analysis and validation steps
- Logs detailed information about what would happen
- Prevents any modifications to your forge platform (branches, PRs, tags, releases)
- **Automatically enables debug mode** for maximum visibility

**Example**:

```bash
export RELEASAURUS_DRY_RUN=true
```

**Usage**:

```bash
# Enable dry-run mode via environment variable
export RELEASAURUS_DRY_RUN=true
releasaurus release-pr --github-repo "https://github.com/owner/repo"
releasaurus release --github-repo "https://github.com/owner/repo"

# Alternative: use the --dry-run flag
releasaurus release-pr --dry-run --github-repo "https://github.com/owner/repo"
```

**Output**: Produces detailed debug logs prefixed with `dry_run:` showing
exactly what operations would be performed, including PR titles, version
numbers, file changes, and release notes.

**Note**: Dry-run mode automatically enables debug logging regardless of the
`RELEASAURUS_DEBUG` setting. This ensures you have maximum visibility into what
would happen during the release process.

## Next Steps

- **[Commands](./commands.md)** - Command-line options and usage patterns
- **[Configuration](./configuration.md)** - Configuration file setup
- **[Troubleshooting](./troubleshooting.md#authentication-issues)** -
  Resolving environment issues
