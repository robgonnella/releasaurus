# Introduction

**Releasaurus** 🦕 is a comprehensive release automation tool that streamlines the software release process across multiple programming languages and Git forge platforms. Designed with simplicity and flexibility in mind, Releasaurus works out-of-the-box with little to no configuration while providing powerful customization options for advanced use cases.

## What is Releasaurus?

Releasaurus automates the entire release workflow, from version detection and changelog generation to creating pull requests and publishing releases. It intelligently detects your project's language and framework, automatically handles version updates across multiple file types, and seamlessly integrates with your preferred Git hosting platform.

## Key Features

### 🚀 **Zero Configuration Start**

Get started immediately without complex setup files or extensive configuration. Releasaurus intelligently detects your project structure and applies sensible defaults.

### 🔍 **Intelligent Detection**

Automatically detects your project's programming language, framework, and version files. No manual specification required for most common project structures.

### 🌍 **Multi-Platform Support**

Works with GitHub, GitLab, and Gitea—whether hosted or self-hosted instances. One tool for all your repositories, regardless of where they're hosted.

### 🤖 **CI/CD Integration**

Complete automation through official integrations:

- **GitHub Actions**: [robgonnella/releasaurus-action]
- **GitLab CI/CD**: [releasaurus-component]
- **Gitea Actions**: [releasaurus-gitea-action]

Automatically create release PRs on push and publish releases when merged—no
manual intervention required.

### 📁 **Remote Repository Operations**

Works from any directory by automatically cloning repositories to temporary
locations for analysis. No need to navigate to project directories or maintain
local checkouts—just point at any repository URL.

### 📦 **Multi-Language Support**

Native support for:

- **Rust** (Cargo.toml)
- **Node.js** (package.json, package-lock.json)
- **Python** (pyproject.toml, setup.py, requirements files)
- **Java** (Maven pom.xml, Gradle build files)
- **PHP** (composer.json)
- **Ruby** (Gemfile, .gemspec files)
- **Generic** projects (changelog and tagging only - no version file updates)

### 📝 **Smart Changelog Generation**

Inspired by [git-cliff](https://git-cliff.org/), automatically generates beautiful changelogs from your commit history with conventional commit support.

### 🏢 **Monorepo Ready**

Handle multiple independently-versioned packages within a single repository with per-package configuration and release cycles.

### 🔧 **Flexible Configuration**

While it works great with defaults, customize every aspect of the release process through an optional `releasaurus.toml` configuration file.

## How It Works

Releasaurus follows a simple two-step release process:

1. **`releasaurus release-pr`** - Analyzes your commits, determines the next version, updates version files, generates a changelog, and creates a pull request for review.

2. **`releasaurus release`** - After the release PR is merged, creates a Git tag and publishes the release to your forge platform.

This workflow provides a safety net through pull request reviews while automating all the tedious version management tasks.

## Why Another Release Tool?

Releasaurus was inspired by excellent tools like [git-cliff](https://git-cliff.org/), [release-please](https://github.com/googleapis/release-please), and [release-plz](https://release-plz.ieni.dev/), each of which excels in their specific domain. However, we identified key limitations that Releasaurus addresses:

- **release-please** only works with GitHub, limiting teams using other platforms
- **release-plz** focuses exclusively on Rust projects
- Existing tools often require extensive configuration for non-standard projects

Releasaurus brings together the best ideas from these tools while providing:

- **Universal platform support** - GitHub, GitLab, and Gitea
- **Multi-language support** - Works with any programming language or framework
- **Minimal configuration** - Intelligent defaults that work immediately
- **Consistent experience** - Same workflow regardless of language or platform

## Credit and Inspiration

We gratefully acknowledge the inspiration and foundation provided by:

- **[git-cliff](https://git-cliff.org/)**
- **[release-please](https://github.com/googleapis/release-please)**
- **[release-plz](https://release-plz.ieni.dev/)**

Releasaurus builds upon these proven concepts while extending support to a broader ecosystem of languages, frameworks, and platforms.

## Getting Started

Ready to automate your releases? Head over to the [Installation](./installation.md) guide to get started, or jump straight into the [Quick Start](./quick-start.md) tutorial to see Releasaurus in action.

Whether you're maintaining a single-language project or a complex monorepo, Releasaurus adapts to your workflow while maintaining the reliability and safety that production releases demand.

[robgonnella/releasaurus-action]: https://github.com/robgonnella/releasaurus-action
[releasaurus-component]: https://gitlab.com/rgon/releasaurus-component
[releasaurus-gitea-action]: https://gitea.com/rgon/releasaurus-gitea-action
