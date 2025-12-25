# Introduction

**Releasaurus** 🦕 is a comprehensive release automation tool that works
out-of-the-box with **zero configuration required**. Simply point it at
your repository to get automated changelog generation and git tagging, or
add a minimal config file to enable version file updates across multiple
programming languages and Git forge platforms.

## Get Started in Seconds

Releasaurus works immediately without any setup:

```bash
# Create a release PR (no config file needed!)
releasaurus release-pr \
  --forge github \
  --repo "https://github.com/your-org/your-repo"

# After merging the PR, publish the release
releasaurus release \
  --forge github \
  --repo "https://github.com/your-org/your-repo"
```

**That's it!** Releasaurus analyzes your commit history, generates a
changelog, and creates a release—no configuration file required.

## When You Need More

Add a `releasaurus.toml` file when you want:

- **Version file updates** - Automatically update package.json,
  Cargo.toml, pom.xml, etc.
- **Monorepo support** - Manage multiple independently-versioned packages
- **Custom changelog templates** - Tailor formatting to your needs
- **Custom tag prefixes** - Use tags like `api-v1.0.0` or `cli-v2.1.0`

See the [Configuration](./configuration.md) guide for details.

## Key Features

### 🚀 **Zero Configuration by Default**

Works immediately for changelog generation and git tagging. Add
configuration only when you need version file updates or custom settings.

### 🌍 **Multi-Platform Support**

Works with GitHub, GitLab, and Gitea—whether hosted or self-hosted
instances. One tool for all your repositories, regardless of where
they're hosted.

### 📦 **Multi-Language Version Updates**

When configured, updates version files for:

- **Rust** (Cargo.toml)
- **Node.js** (package.json, package-lock.json, yarn.lock)
- **Python** (pyproject.toml, setup.py, setup.cfg, requirements files)
- **Java** (Maven pom.xml, Gradle build files)
- **PHP** (composer.json)
- **Ruby** (Gemfile, .gemspec files)
- **Generic** projects (custom version file patterns)

### 🎯 **Command-Line Overrides**

Test configurations, create emergency releases, or use different
prerelease settings without modifying your config file. Override base
branch, prerelease settings, and per-package configurations directly from
the command line—perfect for CI/CD environments with multiple deployment
targets.

See [Configuration Overrides](./commands.md#configuration-overrides) for
details.

### 📁 **Forge API Integration**

Works entirely through forge platform APIs—no local repository cloning
required. Analyzes commits, creates branches, and manages releases
directly via API calls, making it ideal for CI/CD environments and remote
automation.

### 📝 **Smart Changelog Generation**

Inspired by [git-cliff](https://git-cliff.org/), automatically generates
beautiful changelogs from your commit history with conventional commit
support.

### 🏢 **Monorepo Ready**

Handle multiple independently-versioned packages within a single
repository with per-package configuration and release cycles.

### 🤖 **CI/CD Integration**

Complete automation through official integrations:

- **GitHub Actions**: [GitHub Actions
  Integration](./ci-cd-integration.md#github-actions)
- **GitLab CI/CD**: [GitLab CI/CD
  Integration](./ci-cd-integration.md#gitlab-cicd)
- **Gitea Actions**: [Gitea Actions
  Integration](./ci-cd-integration.md#gitea-actions)

Automatically create release PRs on push and publish releases when
merged—no manual intervention required.

## How It Works

### Core Workflow (2 Steps)

1. **`releasaurus release-pr`** - Analyzes your commits, determines the
   next version, updates version files (if configured), generates a
   changelog, and creates a pull request for review.

2. **`releasaurus release`** - After the release PR is merged, creates a
   Git tag and publishes the release to your forge platform.

This workflow provides a safety net through pull request reviews while
automating all the tedious version management tasks.

### Optional Enhancements

- **`releasaurus start-next`** - Automatically bump patch versions after
  release to start the next development cycle. Perfect for continuous
  development workflows.

- **`releasaurus show`** - Query release information for automation,
  CI/CD pipelines, and custom notifications without making any changes.

## What Problems Does Releasaurus Solve?

### For Teams Across Multiple Platforms

- Stuck on GitHub but want to migrate to GitLab or self-hosted Gitea?
- Managing repositories across different forge platforms?
- Need a single release tool that works everywhere?

### For Multi-Language Projects

- Managing releases across Rust, Node.js, Python, and Java projects?
- Want consistent release workflows regardless of language?
- Need automatic version file updates across different ecosystems?

### For Flexibility and Control

- Need zero-config changelog generation without setup files?
- Want to test prerelease configurations before committing changes?
- Need different release settings for dev/staging/prod environments?
- Looking for command-line overrides for emergency releases?

### For Monorepo Management

- Managing multiple packages with independent version numbers?
- Need separate or combined release PRs for different packages?
- Want per-package prerelease configurations?

Releasaurus addresses these challenges by providing universal platform
support, multi-language compatibility, minimal configuration, and
flexible command-line overrides.

## Credit and Inspiration

Releasaurus was inspired by excellent tools like
[git-cliff](https://git-cliff.org/),
[release-please](https://github.com/googleapis/release-please), and
[release-plz](https://release-plz.ieni.dev/). We're grateful for the
foundation these projects provided and have built upon their proven
concepts while extending support to a broader ecosystem of languages,
frameworks, and platforms.

## Getting Started

Ready to automate your releases?

- **Quick start**: Jump to [Quick Start](./quick-start.md) to see
  Releasaurus in action
- **Installation**: See [Installation](./installation.md) for setup
  instructions
- **Configuration**: Check [Configuration](./configuration.md) when you
  need version file updates or custom settings

Whether you're maintaining a single-language project or a complex
monorepo, Releasaurus adapts to your workflow while maintaining the
reliability and safety that production releases demand.
