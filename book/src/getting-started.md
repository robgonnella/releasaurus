# Getting Started

Install Releasaurus and cut your first release in a few minutes.

## Install

### Pre-built binary (recommended)

The fastest option, via
[cargo-binstall](https://github.com/cargo-bins/cargo-binstall):

```bash
cargo binstall releasaurus
```

### From crates.io

Compiles from source:

```bash
cargo install releasaurus
```

### Docker

```bash
docker pull rgonnella/releasaurus:latest
docker run --rm rgonnella/releasaurus:latest --help
```

You can also download a binary directly from the
[releases page](https://github.com/robgonnella/releasaurus/releases), or
build from source — see [Contributing](./contributing.md). Confirm the
install with `releasaurus --version`.

## Preview Without Any Risk

Run Releasaurus against a local checkout to see what it would do — no
token, no config, no changes:

```bash
cd /path/to/your/repo
releasaurus release-pr --forge local --repo "."
```

The output shows the next version and the generated changelog without
touching your repository. See
[Local & Dry-Run Modes](./commands.md#testing-modes) for more.

## Cut Your First Release

### 1. Set an access token

Releasaurus picks the right variable from the `--forge` you use:

```bash
export GITHUB_TOKEN="ghp_your_token_here"    # GitHub
export GITLAB_TOKEN="glpat_your_token_here"  # GitLab
export GITEA_TOKEN="your_token_here"         # Gitea
```

Every token variable and its required scopes are listed in the
[Configuration Reference](./configuration-reference.md#environment-variables).

### 2. Open a release PR

```bash
releasaurus release-pr --repo "https://github.com/your-org/your-repo"
```

This analyzes your commits, picks the next version, generates a
changelog, and opens a pull request. (`--forge` is inferred for known
hosts like `github.com`; pass it explicitly for self-hosted instances.)

### 3. Merge, then publish

After reviewing and merging the PR:

```bash
releasaurus release --repo "https://github.com/your-org/your-repo"
```

This tags the release commit and publishes the release on your forge.

## Add Version File Updates (Optional)

By default Releasaurus only writes changelogs and tags. To also bump
versions in your manifests (`package.json`, `Cargo.toml`, etc.), add a
`releasaurus.toml` at the repository root:

```toml
[[package]]
path = "."
release_type = "node"  # or rust, python, java, php, ruby, go, generic
```

See [Configuration](./configuration.md) for monorepos, prereleases,
changelog customization, and the full option list.

## Next Steps

- **[Commands](./commands.md)** — all commands, overrides, and the
  `start-next` and `get` commands.
- **[CI/CD Integration](./ci-cd-integration.md)** — automate with GitHub
  Actions, GitLab CI, and more.
- **[Troubleshooting](./troubleshooting.md)** — common issues and fixes.
