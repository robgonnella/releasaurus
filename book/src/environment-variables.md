# Environment Variables

Environment variables provide a secure and flexible way to configure Releasaurus without hardcoding sensitive information or platform-specific settings. This reference covers all supported environment variables and their usage patterns.

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

# Forgejo instance
releasaurus release-pr --gitea-repo "https://forgejo.example.com/user/project"
```

## Next Steps

- **[Basic Configuration](./basic-configuration.md)** - Configuration file setup
- **[Troubleshooting](./troubleshooting.md#authentication-issues)** - Resolving environment issues
