# Quick Start

This guide will get you up and running with Releasaurus in just a few minutes.
We'll walk through releasing a simple project to demonstrate the core workflow.

## Prerequisites

Before starting, ensure you have:

1. **Releasaurus installed** - See [Installation](./installation.md) if you
   haven't already
2. **A Git repository** with some commits to release
3. **Access token** for your Git forge platform (GitHub, GitLab, or Gitea)
4. **Push access** to your repository

## Step 1: Prepare Your Access Token

Releasaurus needs an access token to create pull requests and releases on your
behalf.

### GitHub

1. Go to [GitHub Settings → Personal Access Tokens]
   (https://github.com/settings/tokens)
2. Generate a new token with these scopes:
   - `repo` (for private repositories)
   - `public_repo` (for public repositories)

### GitLab

1. Go to [GitLab User Settings → Access Tokens]
   (https://gitlab.com/-/profile/personal_access_tokens)
2. Create a token with these scopes:
   - `api`
   - `read_repository`
   - `write_repository`

### Gitea

1. Go to your Gitea instance → User Settings → Applications
2. Generate a new token with repository read/write permissions

## Step 2: Identify Your Project Repository

Releasaurus works entirely through forge platform APIs—no local repository
required. You just need the repository URL and appropriate access permissions.

For version file updates, you'll need to specify your project's `release_type`
in `releasaurus.toml`. Supported types include:

- **Rust**: For projects with `Cargo.toml`
- **Node**: For projects with `package.json`
- **Python**: For projects with `pyproject.toml`, `setup.py`, or `setup.cfg`
- **Java**: For projects with `pom.xml` or `build.gradle`
- **Php**: For projects with `composer.json`
- **Ruby**: For projects with `Gemfile` or `.gemspec` files
- **Generic**: (default) For changelog and tagging only (no version file updates)

## Step 3: Create a Release PR

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

1. **Analyze your commits** since the last release using conventional commit
   patterns
2. **Determine the next version** based on the changes (patch, minor, or
   major)
3. **Update version files** in your project automatically
4. **Generate a changelog** from your commit history
5. **Create a pull request** with all the changes ready for review

## Step 4: Review and Merge

1. **Review the pull request** that was created
2. **Check the changelog** and version updates
3. **Make any necessary adjustments** by pushing additional commits to the PR
   branch
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

Instead of passing tokens as command-line arguments, you can use environment
variables:

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

## Basic Configuration

To enable version file updates, create a `releasaurus.toml` file specifying
your project's release type:

```toml
[[package]]
path = "."
release_type = "node"
```

For repositories with extensive commit history, you can also control how many
commits are analyzed when determining the first release version:

```toml
# Limit commit history search for first release (default: 400)
first_release_search_depth = 200

[[package]]
path = "."
release_type = "node"
```

This setting only affects the first release when no previous tags exist.
Subsequent releases automatically find commits since the last tag. Adjust this
if:

- **Your repository is very large** - Use a smaller depth like `100` for
  faster analysis
- **You need comprehensive history** - Increase to `1000` or more for deeper
  analysis
- **You're in a CI/CD environment** - Use a smaller depth for faster builds

### Filtering Changelog Commits

You can also control which commits appear in your changelog by adding filtering
options to the `[changelog]` section:

```toml
[changelog]
skip_ci = true                # Exclude CI/CD commits
skip_chore = true             # Exclude chore/maintenance commits
skip_miscellaneous = true     # Exclude non-conventional commits
skip_merge_commits = true     # Exclude merge commits (default: true)
skip_release_commits = true   # Exclude release commits (default: true)
include_author = true         # Show commit author names

[[package]]
path = "."
release_type = "node"
```

These options help you:

- **`skip_ci`** - Remove CI/CD related commits (e.g., "ci: update workflow")
- **`skip_chore`** - Remove maintenance commits (e.g., "chore: update deps")
- **`skip_miscellaneous`** - Remove commits without conventional type prefixes
- **`skip_merge_commits`** - Remove merge commits (default: true)
- **`skip_release_commits`** - Remove automated release commits (default: true)
- **`include_author`** - Add author attribution to each changelog entry

This keeps your changelog focused on user-facing changes. See the
[Configuration](./configuration.md) guide for more details and examples.

## Common Patterns

### Workflow Integration

Many teams integrate Releasaurus into their development workflow:

1. **Develop and commit** using conventional commits to your repository
2. **When ready to release**, run `releasaurus release-pr` to create a release PR
3. **Review and merge** the PR when ready
4. **Publish the release** with `releasaurus release`

Example release workflow:

```bash
# Create release PR
releasaurus release-pr --github-repo "https://github.com/owner/repo"

# Review, merge, then publish
releasaurus release --github-repo "https://github.com/owner/repo"
```

## What Just Happened?

Congratulations! You've just completed a full release cycle with Releasaurus:

1. ✅ **Automated version calculation** - Determines version bump from commits
2. ✅ **Automatic file updates** - All version files updated consistently
3. ✅ **Generated changelog** - Beautiful changelog from your commit history
4. ✅ **Safe review process** - Changes reviewed via pull request
5. ✅ **Published release** - Tagged and published to your forge platform

## Next Steps

This quick start used all defaults, but Releasaurus is highly customizable:

- **[Configuration](./configuration.md)** - Customization options and advanced
  setup
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

This provides detailed information about configuration loading, API calls, and
file operations.

### Automation with CI/CD

While the manual workflow above works great, you can fully automate your
releases using CI/CD platforms:

#### GitHub Actions

The official [robgonnella/releasaurus-action] automatically creates release
PRs when you push to your main branch and publishes releases when those PRs
are merged. See full action [documentation][robgonnella/releasaurus-action]
for all available options.

Create `.github/workflows/release.yml` in your repository:

```yaml
name: Release
on:
  workflow_dispatch:
  push:
    branches: [main]

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - name: Release
        uses: robgonnella/releasaurus-action@v1
        with:
          github-token: ${{ secrets.GITHUB_TOKEN }}
```

With this setup, your releases become completely hands-off:

1. **Push commits** using conventional commit format to your main branch
2. **GitHub Action automatically creates** a release PR with version updates
   and changelog
3. **Review and merge** the PR when ready
4. **GitHub Action automatically publishes** the release with tags and release
   notes

#### GitLab CI/CD

For GitLab projects, use the official [releasaurus-component] that integrates
seamlessly with GitLab CI/CD pipelines. Create `.gitlab-ci.yml` in your
repository:

```yaml
include:
  - component: gitlab.com/rgon/releasaurus-component/releasaurus@~latest
```

See the [GitLab CI/CD](./gitlab-ci.md) integration guide for complete setup
instructions.

#### Gitea Actions

For Gitea repositories, use the official [releasaurus-gitea-action] that
integrates seamlessly with Gitea Actions workflows. Create
`.gitea/workflows/release.yml` in your repository:

```yaml
name: Release
on:
  workflow_dispatch:
  push:
    branches: [main]

jobs:
  release:
    runs-on: ubuntu-latest
    permissions:
      contents: write
      pull-requests: write
      issues: write
    steps:
      - name: Checkout
        uses: actions/checkout@v5
      - name: Run Releasaurus
        uses: https://gitea.com/rgon/releasaurus-gitea-action@v1
```

See the [Gitea Actions](./gitea-actions.md) integration guide for complete setup
instructions.

Ready to dive deeper? Check out the [Commands](./commands.md) section for
detailed information about all available options and features.

[robgonnella/releasaurus-action]: https://github.com/robgonnella/releasaurus-action
[releasaurus-component]: https://gitlab.com/rgon/releasaurus-component
[releasaurus-gitea-action]: https://gitea.com/rgon/releasaurus-gitea-action
