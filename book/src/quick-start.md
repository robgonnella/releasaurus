# Quick Start

This guide will get you up and running with Releasaurus in just a few minutes. We'll walk through releasing a simple project to demonstrate the core workflow.

## Prerequisites

Before starting, ensure you have:

1. **Releasaurus installed** - See [Installation](./installation.md) if you haven't already
2. **A Git repository** with some commits to release
3. **Access token** for your Git forge platform (GitHub, GitLab, or Gitea)
4. **Push access** to your repository

## Step 1: Prepare Your Access Token

Releasaurus needs an access token to create pull requests and releases on your behalf.

### GitHub

1. Go to [GitHub Settings → Personal Access Tokens](https://github.com/settings/tokens)
2. Generate a new token with these scopes:
   - `repo` (for private repositories)
   - `public_repo` (for public repositories)

### GitLab

1. Go to [GitLab User Settings → Access Tokens](https://gitlab.com/-/profile/personal_access_tokens)
2. Create a token with these scopes:
   - `api`
   - `read_repository`
   - `write_repository`

### Gitea

1. Go to your Gitea instance → User Settings → Applications
2. Generate a new token with repository read/write permissions

## Step 2: Identify Your Project Repository

You can run Releasaurus from any directory - it automatically clones your repository to a temporary location for analysis and updates. You just need the repository URL and appropriate access permissions.

Releasaurus works with any project structure and automatically detects:

- **Rust**: Projects with `Cargo.toml`
- **Node.js**: Projects with `package.json`
- **Python**: Projects with `pyproject.toml`, `setup.py`, or `setup.cfg`
- **Java**: Projects with `pom.xml` or `build.gradle`
- **PHP**: Projects with `composer.json`
- **Ruby**: Projects with `Gemfile` or `.gemspec` files

## Step 3: Create a Release PR

**Important**: You can run this command from any directory. Releasaurus will automatically clone your repository to analyze it and create the release PR.

Run the release-pr command with your repository information:

```bash
# GitHub example
releasaurus release-pr \
  --github-repo "https://github.com/owner/repo" \
  --github-token "ghp_your_token_here"

# GitLab example
releasaurus release-pr \
  --gitlab-repo "https://gitlab.com/owner/repo" \
  --gitlab-token "glpat_your_token_here"

# Gitea example
releasaurus release-pr \
  --gitea-repo "https://git.example.com/owner/repo" \
  --gitea-token "your_token_here"
```

This command will:

1. **Analyze your commits** since the last release using conventional commit patterns
2. **Determine the next version** based on the changes (patch, minor, or major)
3. **Update version files** in your project automatically
4. **Generate a changelog** from your commit history
5. **Create a pull request** with all the changes ready for review

## Step 4: Review and Merge

The release PR is created in your repository regardless of where you ran the command from.

1. **Review the pull request** that was created
2. **Check the changelog** and version updates
3. **Make any necessary adjustments** by pushing additional commits to the PR branch
4. **Merge the pull request** when you're satisfied

## Step 5: Publish the Release

After merging the release PR, publish the actual release:

```bash
# Use the same platform and credentials as before
releasaurus release \
  --github-repo "https://github.com/owner/repo" \
  --github-token "ghp_your_token_here"
```

This will:

1. **Create a Git tag** for the new version
2. **Push the tag** to your repository
3. **Create a release** on your forge platform with the generated changelog

## Environment Variables (Alternative)

Instead of passing tokens as command-line arguments, you can use environment variables:

```bash
# Set your token
export GITHUB_TOKEN="ghp_your_token_here"
# or
export GITLAB_TOKEN="glpat_your_token_here"
# or
export GITEA_TOKEN="your_token_here"

# Then run commands without --*-token flags
releasaurus release-pr --github-repo "https://github.com/owner/repo"
releasaurus release --github-repo "https://github.com/owner/repo"
```

## What Just Happened?

Congratulations! You've just completed a full release cycle with Releasaurus:

1. ✅ **Automated version detection** - No manual version bumping
2. ✅ **Automatic file updates** - All version files updated consistently
3. ✅ **Generated changelog** - Beautiful changelog from your commit history
4. ✅ **Safe review process** - Changes reviewed via pull request
5. ✅ **Published release** - Tagged and published to your forge platform

## Next Steps

This quick start used all defaults, but Releasaurus is highly customizable:

- **[Configuration](./configuration.md)** - Customization options and advanced setup
- **[Troubleshooting](./troubleshooting.md)** - Common issues and solutions

## Common Patterns

### Workflow Integration

Many teams integrate Releasaurus into their development workflow:

```bash
# 1. Develop and commit using conventional commits (in your project directory)
git commit -m "feat: add user authentication"
git commit -m "fix: resolve login validation issue"
git commit -m "docs: update API documentation"

# 2. When ready to release (can be run from anywhere)
releasaurus release-pr --github-repo "https://github.com/owner/repo"

# 3. Review, merge, then publish (can be run from anywhere)
releasaurus release --github-repo "https://github.com/owner/repo"
```

### Debug Mode

If something isn't working as expected, enable debug logging:

```bash
releasaurus release-pr --debug \
  --github-repo "https://github.com/owner/repo"
```

This provides detailed information about detection logic, API calls, and file operations.

Ready to dive deeper? Check out the [Commands](./commands.md) section for detailed information about all available options and features.
