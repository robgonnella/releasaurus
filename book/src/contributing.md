# Contributing

We appreciate your interest in contributing to Releasaurus! This guide will
help you get started with contributing to the project, whether you're fixing
bugs, adding features, improving documentation, or helping with community
support.

## Ways to Contribute

### 🐛 Bug Reports

- Report bugs and issues you encounter
- Provide detailed reproduction steps
- Share your environment details

### 💡 Feature Requests

- Suggest new language/framework support
- Propose workflow improvements
- Request platform integrations

### 🔧 Code Contributions

- Fix bugs and implement features
- Add support for new languages
- Improve performance and reliability

### 📚 Documentation

- Improve existing documentation
- Add examples and tutorials
- Translate documentation

### 🎯 Testing

- Write and improve tests
- Test on different platforms
- Validate new features

### 💬 Community Support

- Help other users in discussions
- Answer questions in issues
- Share your experience and best practices

## Development Environment Setup

### Prerequisites

- **Rust**: 1.92 or higher ([Install Rust](https://rustup.rs/))
- **Git**: For version control
- **A supported platform**: GitHub, GitLab, or Gitea account for testing

See below section for managing rust version with mise.

### Install Mise

Mise is used to manage rust version for local development. Refer to
[mise.toml](https://github.com/robgonnella/releasaurus/blob/main/mise.toml) for
the list of tools and versions managed by mise.

- Installing: https://mise.jdx.dev/installing-mise.html
- Activating: https://mise.jdx.dev/installing-mise.html#shells

### Getting Started

1. **Fork and Clone**

   ```bash
   # Fork the repository on GitHub
   git clone https://github.com/your-username/releasaurus.git
   cd releasaurus
   ```

2. **Install Dependencies**

Assuming mise is installed and activated, run the following from the root of
this repo.

```bash
mise trust
mise install
```

This will install the correct version of rust as well as other dependencies
used for local development. It will also ensure any time you `cd` to this
directory the proper versions of these tools are selected and added to the front
of your PATH. It will also ensure that any environment variables you define in
`.env` are automatically loaded into your environment, similar to `direnv`.

3. **Using just commands**

A Justfile is provided for quick access to a number of commands for developing
locally. This tool is automatically installed and managed via `mise`.

```bash
just build # builds the project
just build --release # builds a release version
just run # builds and run the releasaurus cli in one command
# For example
just run --help
# Is equivalent to `cargo run -p releasaurus -- --help`
just help # show all available just recipes and their descriptions
```

4. **Running tests**

There are two types of tests included in this repository, unit and integration.
Unit tests use mocks without ever interacting with real forges. Integration
tests run against real forges and require setting up proper environment
variables to point at real repositories used for testing.

**Unit tests**

```bash
# run unit tests
just test
# run unit tests with coverage
just test-cov
```

**Integration tests**

The following environment variables are used when running integration tests.

- `GITHUB_TEST_REPO`
- `GITHUB_TEST_TOKEN`
- `GITHUB_RESET_SHA`

- `GITLAB_TEST_REPO`
- `GITLAB_TEST_TOKEN`
- `GITLAB_RESET_SHA`

- `GITEA_TEST_REPO`
- `GITEA_TEST_TOKEN`
- `GITEA_RESET_SHA`

⚠️ Whatever you configure for these repositories WILL have their histories
overwritten. All PRs, tags, releases, and branches will be deleted and the
repository will be hard reset back to the configured reset sha at the start
of the test suite.

```bash
# Create test tokens (with minimal permissions)
export GITHUB_TEST_REPO="https://github.com/your/test/repo"
export GITHUB_TEST_TOKEN="gh_test_token"
export GITHUB_RESET_SHA="abc123"

export GITLAB_TEST_REPO="https://gitlab.com/your/test/repo"
export GITLAB_TEST_TOKEN="gl_test_token"
export GITLAB_RESET_SHA="abc123"

export GITEA_TEST_REPO="https://gitea.com/your/test/repo"
export GITEA_TEST_TOKEN="gt_test_token"
export GITEA_RESET_SHA="abc123"

# Or you can add these to a .env file and mise will automatically load them
# .env
# GITHUB_TEST_REPO="https://github.com/your/test/repo"
# GITHUB_TEST_TOKEN="gh_test_token"
# GITHUB_RESET_SHA="abc123"

# GITLAB_TEST_REPO="https://gitlab.com/your/test/repo"
# GITLAB_TEST_TOKEN="gl_test_token"
# GITLAB_RESET_SHA="abc123"

# GITEA_TEST_REPO="https://gitea.com/your/test/repo"
# GITEA_TEST_TOKEN="gt_test_token"
# GITEA_RESET_SHA="abc123"

just test-all # run all tests including integration tests

# target just github integration tests
just test-github-integration
# target just gitlab integration tests
just test-gitlab-integration
# target just gitea integration tests
just test-gitea-integration
```

## Code Contribution Guidelines

### Coding Standards

#### Rust Style

- Use `cargo fmt` for consistent formatting
- Use `cargo clippy` for linting
- Write comprehensive documentation comments

### Adding New Language/Framework Updaters

Releasaurus supports multiple programming languages through its updater system.
Adding support for a new language requires updating several files across the
codebase.

#### Overview

Each language updater consists of:

1. **ReleaseType enum variant** - Configuration option in
   `crates/releasaurus-core/src/config/release_type.rs`
2. **Manifests module** - Defines manifest file targets in
   `crates/releasaurus-core/src/updater/yourlanguage/manifests.rs`
3. **Updater module** - Language-specific implementation in
   `crates/releasaurus-core/src/updater/yourlanguage/updater.rs`
4. **File parsers** - Version file format handlers (e.g.,
   `crates/releasaurus-core/src/updater/yourlanguage/your_file_type.rs`)
5. **Tests** - Comprehensive test coverage for all modules
6. **Documentation** - User-facing documentation updates

#### Files to Update

**1. Add ReleaseType Variant**

- `crates/releasaurus-core/src/config/release_type.rs` - Add your
  language to the `ReleaseType` enum

**2. Create Manifests Module**

- `crates/releasaurus-core/src/updater/yourlanguage/manifests.rs` -
  Implement `ManifestTargets` trait
- `crates/releasaurus-core/src/updater/manager.rs` - Register in
  `release_type_manifest_targets()` function

**3. Create Updater Implementation**

- `crates/releasaurus-core/src/updater/yourlanguage.rs` - Module
  declaration file
- `crates/releasaurus-core/src/updater/yourlanguage/updater.rs` -
  Implement `PackageUpdater` trait
- `crates/releasaurus-core/src/updater/yourlanguage/your_file_type.rs` -
  File format parser(s)
- `crates/releasaurus-core/src/updater.rs` - Add module declaration
- `crates/releasaurus-core/src/updater/manager.rs` - Register in
  `updater()` function

**4. Add Tests**

- Tests in `manifests.rs` - Test manifest target generation
- Tests in `updater.rs` - Test updater integration
- Tests in file parser modules - Test version updates

**5. Update Documentation**

- `book/src/supported-languages.md` - Add language section
- `book/src/configuration.md` - Update ReleaseType options

#### Reference Implementations

Use existing language implementations as templates:

**Simple Languages (Good starting points):**

- **PHP**: `crates/releasaurus-core/src/updater/php/` - Single
  manifest file, straightforward JSON parsing
- **Python**: `crates/releasaurus-core/src/updater/python/` -
  Multiple manifest formats (TOML, cfg, py)

**Complex Languages (Advanced features):**

- **Node**: `crates/releasaurus-core/src/updater/node/` - Workspace
  support, multiple lock files
- **Rust**: `crates/releasaurus-core/src/updater/rust/` - Workspace
  detection, dependency updates
- **Java**: `crates/releasaurus-core/src/updater/java/` - Multiple
  build tools (Maven, Gradle, version catalogs), properties files

**Key Traits to Implement:**

- `ManifestTargets` in `manifests.rs` - Defines which files to load
- `PackageUpdater` in `updater.rs` - Coordinates version updates

#### Testing Guidelines

Follow the established testing patterns:

- **Test outcomes, not implementation** - Verify behavior, not how
  it's achieved
- **Minimal and concise** - Only test what provides value
- **Use helper functions** - Reduce duplication (see `test_helpers.rs`)
- **Descriptive names** - e.g., `returns_all_manifest_targets` not
  `test_1`

**Example test files to reference:**

- `crates/releasaurus-core/src/updater/php/manifests.rs` - Simple
  manifest tests
- `crates/releasaurus-core/src/updater/node/manifests.rs` -
  Workspace-aware manifest tests
- `crates/releasaurus-core/src/updater/php/updater.rs` - Basic
  updater tests

#### Running Tests

```bash
# Run all tests for your language
just test yourlanguage

# Run specific module tests
just test updater::yourlanguage::manifests
just test updater::yourlanguage::updater

# Test with real repository
just run release-pr \
  --forge local \
  --repo "/path/to/test/project" \
  --debug

# Test with real remote forge and local path to clone
just run release-pr \
  --forge github \
  --repo "https://github.com/your/repo" \
  --local-path "/path/to/your/repo" \
  --debug \
  --dry-run
```

#### Best Practices

- **Parse robustly** - Handle various file formats and edge cases
- **Preserve formatting** - Maintain original file structure when possible
- **Log clearly** - Use `info!`, `warn!`, `error!` macros appropriately
- **Follow patterns** - Study existing implementations before starting
- **Write tests first** - Define expected behavior through tests

#### Getting Help

- Review existing implementations in
  `crates/releasaurus-core/src/updater/`
- Check test files for usage patterns
- Ask questions in GitHub Discussions
- Reference the `PackageUpdater` and `ManifestTargets` trait documentation

## Code of Conduct

This repository adheres the [Rust Code of Conduct]

### Reporting

Report any unacceptable behavior to the project maintainers.

## Community and Communication

### Channels

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: General questions and community support
- **Pull Requests**: Code review and collaboration

### Getting Help

- **Documentation**: Start with this book
- **Search Issues**: Check if your question has been asked
- **Ask Questions**: Create a discussion or issue
- **Debug Mode**: Use `--debug` flag or `RELEASAURUS_DEBUG` environment
  variable for troubleshooting

Thank you for contributing to Releasaurus!

[Rust Code of Conduct]: https://www.rust-lang.org/policies/code-of-conduct
