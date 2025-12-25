# Quick Start

Get Releasaurus running in under 2 minutes with zero configuration.

## Test Locally (No Setup Required)

Try Releasaurus on any git repository without tokens or configuration:

```bash
# 1. Navigate to any git repository
cd /path/to/your/repo

# 2. Run Releasaurus locally
releasaurus release-pr --forge local --repo "."

# 3. Review the output
# - See what version would be released
# - View the generated changelog
# - No changes made to your repository
```

**That's it!** You've just seen what Releasaurus would do without any risk.

## Go Live in 3 Steps

Ready to create an actual release? Here's the full workflow:

### 1. Set Your Access Token

```bash
# GitHub
export GITHUB_TOKEN="ghp_your_token_here"

# GitLab
export GITLAB_TOKEN="glpat_your_token_here"

# Gitea
export GITEA_TOKEN="your_token_here"
```

See [Environment Variables](./environment-variables.md) for token setup
details.

### 2. Create a Release PR

```bash
releasaurus release-pr \
  --forge github \
  --repo "https://github.com/your-org/your-repo"
```

This analyzes commits, generates a changelog, and creates a pull request.

### 3. Merge PR, Then Publish

After merging the pull request:

```bash
releasaurus release \
  --forge github \
  --repo "https://github.com/your-org/your-repo"
```

This creates a git tag and publishes the release.

## Need Version File Updates?

By default, Releasaurus only creates changelogs and tags. To update
version files (package.json, Cargo.toml, etc.), add a
`releasaurus.toml`:

```toml
[[package]]
path = "."
release_type = "node"  # or rust, python, java, php, ruby
```

See [Configuration](./configuration.md) for all options.

## Next Steps

- **[Commands](./commands.md)** - All commands and options including
  dry-run mode, CLI overrides, and the `start-next` command
- **[CI/CD Integration](./ci-cd-integration.md)** - Automate with GitHub
  Actions, GitLab CI, or Gitea Actions
- **[Configuration](./configuration.md)** - Monorepo support, custom
  changelogs, prerelease versions, and more
- **[Troubleshooting](./troubleshooting.md)** - Common issues and
  solutions
