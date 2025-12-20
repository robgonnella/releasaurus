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

- **Rust**: 1.70 or higher ([Install Rust](https://rustup.rs/))
- **Git**: For version control
- **A supported platform**: GitHub, GitLab, or Gitea account for testing

### Getting Started

1. **Fork and Clone**

   ```bash
   # Fork the repository on GitHub
   git clone https://github.com/your-username/releasaurus.git
   cd releasaurus
   ```

2. **Install Dependencies**

   ```bash
   # Install Rust dependencies
   cargo build

   # Install development tools
   cargo install cargo-watch
   cargo install cargo-nextest  # Optional: faster test runner
   ```

3. **Set Up Testing Environment**

   ```bash
   # Create test tokens (with minimal permissions)
   export GITHUB_TOKEN="ghp_test_token_here"
   export GITLAB_TOKEN="glpat_test_token_here"
   export GITEA_TOKEN="test_token_here"

   # Run tests
   cargo test
   ```

4. **Verify Installation**
   ```bash
   # Build and test the binary
   cargo build --release
   ./target/release/releasaurus --help
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
   `src/config/release_type.rs`
2. **Manifests module** - Defines manifest file targets in
   `src/updater/yourlanguage/manifests.rs`
3. **Updater module** - Language-specific implementation in
   `src/updater/yourlanguage/updater.rs`
4. **File parsers** - Version file format handlers (e.g.,
   `src/updater/yourlanguage/your_file_type.rs`)
5. **Tests** - Comprehensive test coverage for all modules
6. **Documentation** - User-facing documentation updates

#### Files to Update

**1. Add ReleaseType Variant**

- `src/config/release_type.rs` - Add your language to the `ReleaseType` enum

**2. Create Manifests Module**

- `src/updater/yourlanguage/manifests.rs` - Implement `ManifestTargets` trait
- `src/updater/manager.rs` - Register in `release_type_manifest_targets()`
  function

**3. Create Updater Implementation**

- `src/updater/yourlanguage.rs` - Module declaration file
- `src/updater/yourlanguage/updater.rs` - Implement `PackageUpdater` trait
- `src/updater/yourlanguage/your_file_type.rs` - File format parser(s)
- `src/updater.rs` - Add module declaration
- `src/updater/manager.rs` - Register in `updater()` function

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

- **PHP**: `src/updater/php/` - Single manifest file, straightforward JSON
  parsing
- **Python**: `src/updater/python/` - Multiple manifest formats (TOML, cfg, py)

**Complex Languages (Advanced features):**

- **Node**: `src/updater/node/` - Workspace support, multiple lock files
- **Rust**: `src/updater/rust/` - Workspace detection, dependency updates
- **Java**: `src/updater/java/` - Multiple build tools (Maven, Gradle),
  properties files

**Key Traits to Implement:**

- `ManifestTargets` in `manifests.rs` - Defines which files to load
- `PackageUpdater` in `updater.rs` - Coordinates version updates

#### Testing Guidelines

Follow the established testing patterns:

- **Test outcomes, not implementation** - Verify behavior, not how it's achieved
- **Minimal and concise** - Only test what provides value
- **Use helper functions** - Reduce duplication (see `test_helpers.rs`)
- **Descriptive names** - e.g., `returns_all_manifest_targets` not `test_1`

**Example test files to reference:**

- `src/updater/php/manifests.rs` - Simple manifest tests
- `src/updater/node/manifests.rs` - Workspace-aware manifest tests
- `src/updater/php/updater.rs` - Basic updater tests

#### Running Tests

```bash
# Run all tests for your language
cargo test yourlanguage

# Run specific module tests
cargo test updater::yourlanguage::manifests
cargo test updater::yourlanguage::updater

# Test with real repository
releasaurus release-pr \
  --forge local \
  --repo "/path/to/test/project" \
  --debug
```

#### Best Practices

- **Parse robustly** - Handle various file formats and edge cases
- **Preserve formatting** - Maintain original file structure when possible
- **Log clearly** - Use `info!`, `warn!`, `error!` macros appropriately
- **Follow patterns** - Study existing implementations before starting
- **Write tests first** - Define expected behavior through tests

#### Getting Help

- Review existing implementations in `src/updater/`
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
