# Introduction

**Releasaurus** 🦕 automates releases across multiple languages and Git
forges. Point it at a repository and it analyzes your commit history,
generates a changelog, and publishes a tagged release — **no
configuration required**. Add a `releasaurus.toml` when you want version
file updates, monorepo support, or custom changelog formatting.

```bash
# 1. Open a release PR (analyzes commits, writes the changelog)
releasaurus release-pr --repo "https://github.com/your-org/your-repo"

# 2. After merging the PR, tag and publish the release
releasaurus release --repo "https://github.com/your-org/your-repo"
```

That two-command loop — **`release-pr`** to prepare, **`release`** to
publish — is the whole workflow. The pull request gives you a review
step; Releasaurus handles the tedious version and changelog work.

## Key Features

- **Zero config by default** — changelog generation and tagging work
  immediately. Configure only when you need more.
- **Multi-forge** — GitHub, GitLab, Gitea, Forgejo, and Azure DevOps
  (experimental), whether cloud-hosted or self-hosted.
- **Multi-language version updates** — Rust, Node.js, Python, Java, PHP,
  Ruby, Go, and a generic regex-based updater for anything else.
- **Monorepo ready** — multiple independently-versioned packages, with
  combined or separate release PRs.
- **Conventional-commit aware** — version bumps follow
  [conventional commits](https://www.conventionalcommits.org/) and
  [semver](https://semver.org/).
- **Forge API native** — runs entirely through forge APIs with no local
  clone required, ideal for CI/CD. An optional hybrid mode uses a local
  clone for git operations.
- **Command-line overrides** — change branch, tag prefix, and prerelease
  settings per run without editing your config.

## Optional Commands

- **`releasaurus start-next`** — bump patch versions right after a
  release to start the next development cycle.
- **`releasaurus get`** — query projected and published release data as
  JSON for automation, notifications, and debugging.

## Where to Go Next

- **[Getting Started](./getting-started.md)** — install and cut your
  first release.
- **[Commands](./commands.md)** — every command, flag, and mode.
- **[Configuration](./configuration.md)** — version file updates,
  monorepos, and prereleases.

## Credit and Inspiration

Releasaurus builds on the proven ideas of
[git-cliff](https://git-cliff.org/),
[release-please](https://github.com/googleapis/release-please), and
[release-plz](https://release-plz.ieni.dev/), extending them to a
broader set of languages, frameworks, and platforms.
