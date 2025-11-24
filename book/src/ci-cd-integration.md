# CI/CD Integration

Releasaurus provides seamless integration with popular CI/CD platforms through official actions and components that automate your release workflow, eliminating the need to run Releasaurus commands manually.

## Overview

All Releasaurus CI/CD integrations follow the same workflow:

1. **Automatic Release PRs**: When you push to your main branch, a release PR is automatically created (or updated) with version bumps and changelog
2. **Review and Merge**: Review the generated release PR and merge when ready
3. **Automatic Publishing**: After merge, the release is automatically tagged and published

This hands-off approach means you focus on writing code with conventional commits, and Releasaurus handles the rest.

## GitHub Actions

Releasaurus provides official GitHub Actions that integrate directly with your GitHub repository workflows.

### Available Actions

Three GitHub Actions are available:

- **[Workflow Action](https://github.com/robgonnella/releasaurus/tree/main/action/github)** - Composite action that runs both `release-pr` and `release` (recommended for most users)
- **[Release PR Action](https://github.com/robgonnella/releasaurus/tree/main/action/github/release-pr)** - Creates and manages release pull requests
- **[Release Action](https://github.com/robgonnella/releasaurus/tree/main/action/github/release)** - Publishes releases after PR merge

### Quick Setup

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
    permissions:
      contents: write
      pull-requests: write
      issues: write
    steps:
      - name: Release
        uses: robgonnella/releasaurus/action/github@vX.X.X
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

For detailed input options and advanced usage, see the [Workflow Action documentation](https://github.com/robgonnella/releasaurus/tree/main/action/github) for all available inputs and configuration options.

## GitLab CI/CD

Releasaurus provides official GitLab CI components that integrate seamlessly with GitLab pipelines.

### Available Components

Three GitLab CI components are available:

- **[Workflow Component](https://github.com/robgonnella/releasaurus/tree/main/templates/workflow)** - Composite component that includes both `release-pr` and `release` (recommended for most users)
- **[Release PR Component](https://github.com/robgonnella/releasaurus/tree/main/templates/release-pr)** - Creates and manages release merge requests
- **[Release Component](https://github.com/robgonnella/releasaurus/tree/main/templates/release)** - Publishes releases after MR merge

### Quick Setup

Create `.gitlab-ci.yml` in your repository:

```yaml
include:
  - component: gitlab.com/rgon/releasaurus/workflow@vX.X.X
    inputs:
      token: $RELEASE_TOKEN
```

For detailed configuration options and advanced usage, see the [Workflow Component documentation](https://github.com/robgonnella/releasaurus/tree/main/templates/workflow).

## Gitea Actions

Releasaurus provides official Gitea Actions that integrate with Gitea Actions workflows.

### Available Actions

Three Gitea Actions are available:

- **[Workflow Action](https://github.com/robgonnella/releasaurus/tree/main/action/gitea)** - Composite action that runs both `release-pr` and `release` (recommended for most users)
- **[Release PR Action](https://github.com/robgonnella/releasaurus/tree/main/action/gitea/release-pr)** - Creates and manages release pull requests
- **[Release Action](https://github.com/robgonnella/releasaurus/tree/main/action/gitea/release)** - Publishes releases after PR merge

### Quick Setup

Create `.gitea/workflows/release.yml` in your repository:

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
      - name: Run Releasaurus
        uses: rgon/releasaurus/action/gitea@vX.X.X
        with:
          token: ${{ secrets.GITEA_TOKEN }}
```

For detailed input options and advanced usage, see the [Workflow Action documentation](https://github.com/robgonnella/releasaurus/tree/main/action/gitea).

## Configuration

All CI/CD integrations work with your existing `releasaurus.toml` configuration file. No additional CI/CD-specific configuration is required.

If you need to customize the release workflow (e.g., skip certain commit types, use prerelease versions, handle monorepos), simply update your `releasaurus.toml` file. See the [Configuration](./configuration.md) guide for all available options.

## Benefits of CI/CD Integration

- **Fully Automated**: Push commits and releases happen automatically
- **Consistent Process**: Same release workflow across all your projects
- **Safe Reviews**: Release PRs/MRs provide review opportunities before publishing
- **Zero Maintenance**: No need to remember release commands or procedures
- **Team Friendly**: Anyone can trigger releases by merging the release PR/MR

## Next Steps

- Review [Configuration](./configuration.md) to customize your release process
- Check [Commands](./commands.md) for manual release options
- See [Troubleshooting](./troubleshooting.md) if you encounter issues
