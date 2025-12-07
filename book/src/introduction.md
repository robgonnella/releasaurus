# Introduction

**Releasaurus** 🦕 is a comprehensive release automation tool that streamlines
the software release process across multiple programming languages and Git forge
platforms. Designed with simplicity and flexibility in mind, Releasaurus
requires minimal configuration through a `releasaurus.toml` file to specify
your project type and provides powerful customization options for advanced use
cases.

## What is Releasaurus?

Releasaurus automates the entire release workflow, from version detection and
changelog generation to creating pull requests and publishing releases. Based
on your configuration, it handles version updates across multiple file types
for various programming languages and seamlessly integrates with your preferred
Git hosting platform.

## Key Features

### 🚀 **Simple Configuration**

Get started quickly with straightforward configuration. Specify your project's
language type in `releasaurus.toml` and Releasaurus handles the rest with
sensible defaults. Or even skip configuration if you don't care about updating
any version files and only care about tagging and releasing.

### 🔍 **Multi-Language Support**

Handles version file updates for Rust, Node.js, Python, Java, PHP, and Ruby
projects. Configure your project's release type once and Releasaurus manages all
version files consistently.

### 🌍 **Multi-Platform Support**

Works with GitHub, GitLab, and Gitea—whether hosted or self-hosted instances.
One tool for all your repositories, regardless of where they're hosted.

### 🤖 **CI/CD Integration**

Complete automation through official integrations:

- **GitHub Actions**: [GitHub Actions Integration](./ci-cd-integration.md#github-actions)
- **GitLab CI/CD**: [GitLab CI/CD Integration](./ci-cd-integration.md#gitlab-cicd)
- **Gitea Actions**: [Gitea Actions Integration](./ci-cd-integration.md#gitea-actions)

Automatically create release PRs on push and publish releases when merged—no
manual intervention required.

### 📁 **Forge API Integration**

Works entirely through forge platform APIs—no local repository cloning
required. Analyzes commits, creates branches, and manages releases directly via
API calls, making it ideal for CI/CD environments and remote automation.

### 📦 **Multi-Language Support**

Native support for:

- **Rust** (Cargo.toml)
- **Node.js** (package.json, package-lock.json, yarn.lock)
- **Python** (pyproject.toml, setup.py, setup.cfg, requirements files)
- **Java** (Maven pom.xml, Gradle build files)
- **PHP** (composer.json)
- **Ruby** (Gemfile, .gemspec files)
- **Generic** projects (see
  [`additional_manifest_files`](./configuration.md#`additional_manifest_files`)
  for version updates)

### 📝 **Smart Changelog Generation**

Inspired by [git-cliff](https://git-cliff.org/), automatically generates
beautiful changelogs from your commit history with conventional commit support.

### 🏢 **Monorepo Ready**

Handle multiple independently-versioned packages within a single repository
with per-package configuration and release cycles.

### 🔧 **Flexible Configuration**

While it works great with defaults, customize every aspect of the release
process through an optional `releasaurus.toml` configuration file.

## How It Works

Releasaurus provides a simple two-step release workflow:

1. **`releasaurus release-pr`** - Analyzes your commits, determines the next
   version, updates version files, generates a changelog, and creates a pull
   request for review.

2. **`releasaurus release`** - After the release PR is merged, creates a Git
   tag and publishes the release to your forge platform.

Additionally, **`releasaurus projected-release`** outputs projected release
information as JSON for automation and CI/CD pipelines without making any
changes.

This workflow provides a safety net through pull request reviews while
automating all the tedious version management tasks.

## Why Another Release Tool?

Releasaurus was inspired by excellent tools like
[git-cliff](https://git-cliff.org/),
[release-please](https://github.com/googleapis/release-please), and
[release-plz](https://release-plz.ieni.dev/), each of which excels in their
specific domain. However, we identified key limitations that Releasaurus
addresses:

- **release-please** only works with GitHub, limiting teams using other
  platforms
- **release-plz** focuses exclusively on Rust projects
- Existing tools often require extensive configuration for non-standard
  projects

Releasaurus brings together the best ideas from these tools while providing:

- **Universal platform support** - GitHub, GitLab, and Gitea
- **Multi-language support** - Works with any programming language or
  framework
- **Minimal configuration** - Intelligent defaults that work immediately
- **Consistent experience** - Same workflow regardless of language or
  platform
- **Flexible commit format** - While conventional commits enable version
  detection, non-conventional commits can still be included in changelogs
- **Non-linear history support** - Works with merge-based workflows by
  filtering merge commits when needed

## Credit and Inspiration

We gratefully acknowledge the inspiration and foundation provided by:

- **[git-cliff](https://git-cliff.org/)**
- **[release-please](https://github.com/googleapis/release-please)**
- **[release-plz](https://release-plz.ieni.dev/)**

Releasaurus builds upon these proven concepts while extending support to a
broader ecosystem of languages, frameworks, and platforms.

## Getting Started

Ready to automate your releases? Head over to the
[Installation](./installation.md) guide to get started, or jump straight into
the [Quick Start](./quick-start.md) tutorial to see Releasaurus in action.

Whether you're maintaining a single-language project or a complex monorepo,
Releasaurus adapts to your workflow while maintaining the reliability and
safety that production releases demand.
