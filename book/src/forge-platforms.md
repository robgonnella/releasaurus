# Git Forge Platforms

Releasaurus provides unified support for multiple Git forge platforms, allowing you to use the same workflow regardless of where your repositories are hosted. Whether you're using GitHub, GitLab, Gitea, or self-hosted instances, Releasaurus adapts seamlessly to your platform's API and conventions.

## Supported Platforms

### GitHub

**Authentication**:

- Personal Access Tokens (classic)
- Fine-grained Personal Access Tokens
- Environment variable: `GITHUB_TOKEN`

**Example Usage**:

```bash
releasaurus release-pr --github-repo "https://github.com/owner/repo"
releasaurus release --github-repo "https://github.com/owner/repo"
```

### GitLab

**Authentication**:

- Personal Access Tokens
- Environment variable: `GITLAB_TOKEN`

**Example Usage**:

```bash
# GitLab.com
releasaurus release-pr --gitlab-repo "https://gitlab.com/group/project"

# Self-hosted GitLab
releasaurus release-pr --gitlab-repo "https://gitlab.company.com/team/repo"
```

**CI/CD Integration**:

For automated releases with GitLab CI/CD pipelines, see the dedicated
[GitLab CI/CD](./gitlab-ci.md) integration guide.

### Gitea

**Authentication**:

- Access Tokens
- Environment variable: `GITEA_TOKEN`

**Example Usage**:

```bash
releasaurus release-pr --gitea-repo "https://git.company.com/team/project"
releasaurus release --gitea-repo "https://git.company.com/team/project"
```

**Actions Integration**:

For automated releases with Gitea Actions workflows, see the dedicated
[Gitea Actions](./gitea-actions.md) integration guide.

## Authentication Methods

All platforms use token-based authentication for security and API access:

### Command-Line Tokens

Pass tokens directly as command-line arguments:

```bash
# GitHub
releasaurus release-pr \
  --github-repo "https://github.com/owner/repo" \
  --github-token "ghp_xxxxxxxxxxxxxxxxxxxx"

# GitLab
releasaurus release-pr \
  --gitlab-repo "https://gitlab.com/owner/repo" \
  --gitlab-token "glpat_xxxxxxxxxxxxxxxxxxxx"

# Gitea
releasaurus release-pr \
  --gitea-repo "https://git.example.com/owner/repo" \
  --gitea-token "xxxxxxxxxxxxxxxxxx"
```

### Environment Variables

Store tokens in environment variables for security and convenience:

```bash
# Set tokens
export GITHUB_TOKEN="ghp_xxxxxxxxxxxxxxxxxxxx"
export GITLAB_TOKEN="glpat_xxxxxxxxxxxxxxxxxxxx"
export GITEA_TOKEN="xxxxxxxxxxxxxxxxxx"

# Use without --*-token flags
releasaurus release-pr --github-repo "https://github.com/owner/repo"
releasaurus release-pr --gitlab-repo "https://gitlab.com/owner/repo"
releasaurus release-pr --gitea-repo "https://git.example.com/owner/repo"
```

### URL Formats

Always use complete HTTP / HTTPS URLs for repository specification:

```bash
# Correct formats
--github-repo "https://github.com/owner/repo"
--gitlab-repo "https://gitlab.com/group/subgroup/project"
--gitea-repo "https://git.example.com/organization/repository"

# Incorrect formats (won't work)
--github-repo "github.com/owner/repo"           # Missing protocol
--github-repo "git@github.com:owner/repo.git"  # SSH format not supported
```

## Next Steps

- Refer to the platform-specific sections above for detailed information about supported features
- Check [Troubleshooting](./troubleshooting.md) for common issues

The unified forge platform support in Releasaurus means you can focus on your release process rather than platform-specific differences, enabling consistent workflows across your entire organization's repositories.
