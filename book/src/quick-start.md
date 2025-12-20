# Quick Start

This guide will get you up and running with Releasaurus in just a few
minutes. We'll walk through releasing a simple project to demonstrate the
core workflow.

## Prerequisites

Before starting, ensure you have:

1. **Releasaurus installed** - See [Installation](./installation.md) if you
   haven't already
2. **A Git repository** with some commits to release
3. **Access token** for your Git forge platform (GitHub, GitLab, or Gitea)
4. **Push access** to your repository

## Step 1: Prepare Your Access Token

Releasaurus needs an access token to create pull requests and releases on
your behalf.

### GitHub

1. Go to [GitHub Settings → Personal Access Tokens](https://github.com/settings/tokens)
2. Choose either **Classic** or **Fine-grained** token type:

**Classic Token - Required Scopes:**

- `repo` (full control of private repositories)

**Fine-grained Token - Required Permissions:**

- **Contents**: Read and write
- **Issues**: Read and write
- **Pull requests**: Read and write

See the [Environment Variables](./environment-variables.md#github_token)
guide for complete permission details.

### GitLab

1. Go to [GitLab User Settings → Access Tokens](https://gitlab.com/-/profile/personal_access_tokens)
2. Create a token with these scopes:
   - `api`
   - `read_repository`
   - `write_repository`

### Gitea

1. Go to your Gitea instance → User Settings → Applications
2. Generate a new token with repository read/write permissions

## Step 2: Configure Your Project (Optional)

Releasaurus works with zero configuration for changelog generation and
tagging. However, if you want version file updates, you'll need to create a
`releasaurus.toml` file in your repository root specifying your project's
`release_type`.

**Supported release types:**

- **Rust**: For projects with `Cargo.toml`
- **Node**: For projects with `package.json`
- **Python**: For projects with `pyproject.toml`, `setup.py`, or `setup.cfg`
- **Java**: For projects with `pom.xml` or `build.gradle`
- **Php**: For projects with `composer.json`
- **Ruby**: For projects with `Gemfile` or `.gemspec` files
- **Generic**: For projects without specific language support (see
  [`additional_manifest_files`](./configuration.md#additional_manifest_files)
  for version updates)

**Minimal configuration example:**

```toml
# releasaurus.toml
[[package]]
path = "."
release_type = "node"
```

For more configuration options including changelog filtering, monorepo
support, prerelease versions, and custom version increment patterns, see the
[Configuration](./configuration.md) guide.

## Step 3: Create a Release PR

Run the release-pr command with your repository information:

```bash
# GitHub example
releasaurus release-pr \
  --forge github \
  --repo "https://github.com/owner/repo" \
  --token "ghp_your_token_here"

# GitLab example
releasaurus release-pr \
  --forge gitlab \
  --repo "https://gitlab.com/owner/repo" \
  --token "glpat_your_token_here"

# Gitea example
releasaurus release-pr \
  --forge gitea \
  --repo "https://git.example.com/owner/repo" \
  --token "your_token_here"
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
3. **Make any necessary adjustments** by pushing additional commits to the
   PR branch
4. **Merge the pull request** when you're satisfied

## Step 5: Publish the Release

After merging the release PR, publish the actual release:

```bash
# Use the same platform and credentials as before
releasaurus release \
  --forge github \
  --repo "https://github.com/owner/repo" \
  --token "ghp_your_token_here"
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

# Then run commands without --token flag
releasaurus release-pr \
  --forge github \
  --repo "https://github.com/owner/repo"

releasaurus release \
  --forge github \
  --repo "https://github.com/owner/repo"
```

## Configuration

To enable version file updates, create a `releasaurus.toml` file specifying
your project's release type:

```toml
[[package]]
path = "."
release_type = "node"
```

For more configuration options including changelog filtering, tag prefixes,
monorepo setups, and prerelease versions, see the
[Configuration](./configuration.md) guide.

## What Just Happened?

Congratulations! You've just completed a full release cycle with Releasaurus:

1. ✅ **Automated version calculation** - Determines version bump from commits
2. ✅ **Automatic file updates** - All version files updated consistently
3. ✅ **Generated changelog** - Beautiful changelog from your commit history
4. ✅ **Safe review process** - Changes reviewed via pull request
5. ✅ **Published release** - Tagged and published to your forge platform

## Next Steps

This quick start used all defaults, but Releasaurus is highly customizable:

- **[Configuration](./configuration.md)** - Customization options and
  advanced setup
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
releasaurus release-pr \
  --forge github \
  --repo "https://github.com/owner/repo"

# 3. Review, merge, then publish (can be run from anywhere)
releasaurus release \
  --forge github \
  --repo "https://github.com/owner/repo"
```

### Debug Mode

If something isn't working as expected, enable debug logging:

```bash
# Via command line flag
releasaurus release-pr \
  --debug \
  --forge github \
  --repo "https://github.com/owner/repo"

# Or via environment variable
export RELEASAURUS_DEBUG=true
releasaurus release-pr \
  --forge github \
  --repo "https://github.com/owner/repo"
```

See the [Environment Variables](./environment-variables.md#releasaurus_debug)
guide for more details.

### Dry Run Mode

Before making actual changes to your repository, test your release workflow
with dry-run mode:

```bash
# Via command line flag
releasaurus release-pr \
  --dry-run \
  --forge github \
  --repo "https://github.com/owner/repo"

# Via environment variable
export RELEASAURUS_DRY_RUN=true
releasaurus release-pr \
  --forge github \
  --repo "https://github.com/owner/repo"

# Review the logs, then run for real
releasaurus release-pr \
  --forge github \
  --repo "https://github.com/owner/repo"
```

Dry-run mode performs all analysis (commit history, version calculation,
changelog generation) and logs what would be created or modified, but
prevents any actual changes to your repository. **Note:** Dry-run mode
automatically enables debug logging for maximum visibility into the release
process.

This is especially useful for:

- **Testing configuration** - Verify your `releasaurus.toml` settings produce
  expected results
- **Troubleshooting** - Diagnose issues without affecting your repository
- **CI/CD setup** - Test automation workflows before going live

See the [Commands](./commands.md#dry-run-mode) guide for complete details on
dry-run mode.

### Local Repository Testing

Before pushing configuration changes to your remote forge, test them against
your local repository. Local repository mode validates your `releasaurus.toml`
settings without requiring authentication or making remote changes:

```bash
# Test your config locally first
releasaurus release-pr --forge local --repo "."

# Review the output, then run against remote
releasaurus release-pr \
  --forge github \
  --repo "https://github.com/owner/repo"
```

**What gets tested:**

- Configuration validation
- Version detection and bumping logic
- Changelog generation from local commits
- Tag prefix matching

**What doesn't happen:**

- No remote API calls or authentication required
- No branches, PRs, tags, or releases created

This is perfect for:

- **Testing configuration changes** - Verify your `releasaurus.toml` updates
  before committing
- **Learning Releasaurus** - Experiment safely without affecting remote
  repositories
- **Quick validation** - Check version detection and changelog output instantly

See the [Commands](./commands.md#local-repository-mode) guide for complete
details.

### Automation with CI/CD

While the manual workflow above works great, you can fully automate your
releases using CI/CD platforms:

#### GitHub Actions

Releasaurus provides an official GitHub Action that automatically creates
release PRs when you push to your main branch and publishes releases when
those PRs are merged. See the
[CI/CD Integration](./ci-cd-integration.md#github-actions) guide for all
available options.

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
        uses: robgonnella/releasaurus/action/github@vX.X.X
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

With this setup, your releases become completely hands-off:

1. **Push commits** using conventional commit format to your main branch
2. **GitHub Action automatically creates** a release PR with version updates
   and changelog
3. **Review and merge** the PR when ready
4. **GitHub Action automatically publishes** the release with tags and release
   notes

#### GitLab CI/CD

For GitLab projects, Releasaurus provides an official component that
integrates seamlessly with GitLab CI/CD pipelines. Create `.gitlab-ci.yml` in
your repository:

```yaml
include:
  - component: gitlab.com/rgon/releasaurus/workflow@vX.X.X
    inputs:
      token: $RELEASE_TOKEN
```

See the [CI/CD Integration](./ci-cd-integration.md#gitlab-cicd) guide for
complete setup instructions.

#### Gitea Actions

For Gitea repositories, Releasaurus provides an official action that
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
        uses: https://gitea.com/rgon/releasaurus/action/gitea@vX.X.X
```

See the [CI/CD Integration](./ci-cd-integration.md#gitea-actions) guide for
complete setup instructions.

Ready to dive deeper? Check out the [Commands](./commands.md) section for
detailed information about all available options and features.
