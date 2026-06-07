# Contributing

Thanks for your interest in contributing to Releasaurus! Bug reports,
feature requests, code, docs, tests, and community support are all
welcome. Bugs and feature requests go through
[GitHub Issues](https://github.com/robgonnella/releasaurus/issues);
general questions through GitHub Discussions.

## Development Setup

**Prerequisites:** Rust 1.92+ ([rustup](https://rustup.rs/)), Git, and a
GitHub/GitLab/Gitea account for testing.

This project uses [Mise](https://mise.jdx.dev/) to manage the Rust
version and dev tools (see
[mise.toml](https://github.com/robgonnella/releasaurus/blob/main/mise.toml)).
After [installing and activating](https://mise.jdx.dev/installing-mise.html)
mise:

```bash
git clone https://github.com/your-username/releasaurus.git
cd releasaurus
mise trust && mise install
```

This installs the correct Rust toolchain and tools (including `just`),
switches to them whenever you `cd` into the repo, and auto-loads any
variables from a local `.env`.

A `Justfile` provides common recipes:

```bash
just build              # build (add --release for a release build)
just run --help         # = cargo run -p releasaurus -- --help
just help               # list all recipes
```

To build and install from source directly:

```bash
cargo install --path crates/cli
```

## Running Tests

There are two kinds of tests:

**Unit tests** use mocks and never touch a real forge:

```bash
just test           # run unit tests
just test-cov       # with coverage
```

**Integration tests** run against real forges and require per-forge
environment variables (`*_TEST_REPO`, `*_TEST_TOKEN`, `*_RESET_SHA` for
`GITHUB`, `GITLAB`, `GITEA`, `FORGEJO`, and `AZURE_DEVOPS`). You can put
them in `.env` for mise to load automatically.

> ⚠️ **The configured test repositories WILL be overwritten.** All PRs,
> tags, releases, and branches are deleted and the repo is hard-reset to
> the configured reset SHA at the start of the suite. Use dedicated,
> disposable repositories with minimal-permission tokens.

```bash
just test-all                      # all tests, including integration
just test-github-integration       # a single forge's integration tests
# (also: gitlab, gitea, forgejo, azure-devops)
```

> **Azure DevOps test setup:** the test repo's default branch must have
> no branch policies (the reset routine force-resets history via a
> temporary branch swap). The PAT needs `Code: Read & Write` and
> `Pull Request Threads: Read & Write`.

## Coding Standards

- Format with `cargo fmt` and lint with `cargo clippy`.
- Write documentation comments for public APIs.
- Test outcomes, not implementation; keep tests minimal and use the
  existing `test_helpers.rs` patterns; name tests descriptively
  (`returns_all_manifest_targets`, not `test_1`).

## Adding a New Language Updater

Each language updater lives under `crates/core/src/updater/<lang>/` and
consists of a `ReleaseType` variant, a manifests module, an updater
module, file parsers, tests, and docs. To add one:

1. **Add the `ReleaseType` variant** in
   `crates/core/src/config/release_type.rs`.
2. **Create the manifests module**
   (`crates/core/src/updater/<lang>/manifests.rs`, implementing
   `ManifestTargets`) and register it in
   `crates/core/src/updater/manager.rs` under
   `release_type_manifest_targets()`.
3. **Create the updater** (`crates/core/src/updater/<lang>/updater.rs`
   implementing `PackageUpdater`, plus per-format file parsers), declare
   the module in `crates/core/src/updater.rs`, and register it in the
   `updater()` function in `manager.rs`.
4. **Add tests** for manifest generation, updater integration, and each
   file parser.
5. **Update the docs** — add the language to the Supported Languages
   table in `book/src/configuration-reference.md`.

**Reference implementations:** PHP and Python are good simple starting
points; Node, Rust, and Java show workspace support, lock files, and
multiple build tools. Verify your work end-to-end with the local and
hybrid modes:

```bash
just run release-pr --forge local --repo "/path/to/test/project" --debug
```

## Code of Conduct

This project follows the
[Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).
Report unacceptable behavior to the project maintainers.

Thank you for contributing to Releasaurus!
