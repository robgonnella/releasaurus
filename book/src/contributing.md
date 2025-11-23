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

### Project Structure

```
releasaurus/
├── src/
│   ├── analyzer/         # Commit analysis
│   ├── command/          # CLI command implementations
│   ├── config.rs         # Configuration file handling
│   ├── forge/            # Git forge platform integrations
│   │   ├── github.rs     # GitHub API client
│   │   ├── gitlab.rs     # GitLab API client
│   │   └── gitea.rs      # Gitea API client
│   ├── repo.rs           # Git repository operations
│   ├── updater/          # Version file update logic
│   │   ├── rust.rs       # Rust/Cargo support
│   │   ├── node.rs       # Node.js support
│   │   ├── python.rs     # Python support
│   │   └── ...           # Other language implementations
│   └── main.rs           # Application entry point
├── tests/                # Integration tests
├── book/                 # Documentation (mdBook)
├── Cargo.toml            # Rust project configuration
└── README.md
```

## Code Contribution Guidelines

### Coding Standards

#### Rust Style

- Use `cargo fmt` for consistent formatting
- Use `cargo clippy` for linting
- Write comprehensive documentation comments

#### Error Handling

```rust
// Use the project's Result type
use crate::result::Result;

// Provide context for errors
fn update_version_file(path: &Path) -> Result<()> {
    fs::write(path, content)
        .with_context(|| {
            format!("Failed to write version file: {}", path.display())
        })?;
    Ok(())
}
```

### Adding Language Support

To add support for a new programming language:

1. **Create Language Module**

   ```rust
   // src/updater/newlang.rs
   use crate::updater::traits::PackageUpdater;

   pub struct NewLangUpdater;

   impl PackageUpdater for NewLangUpdater {
       fn update(
           &self,
           packages: Vec<Package>,
           loader: &dyn FileLoader
       ) -> Result<Option<Vec<FileChange>>> {
           // Implementation for updating version files
           // Return Some(file_changes) if updates were made
           // Return None if no updates needed
       }
   }
   ```

2. **Register in Framework System**

   ```rust
   // src/updater/framework.rs
   pub enum Framework {
       // ... existing frameworks
       NewLang,
   }

   impl Framework {
       pub fn updater(&self) -> Box<dyn PackageUpdater> {
           match self {
               // ... existing updaters
               Framework::NewLang => Box::new(NewLangUpdater::new()),
           }
       }
   }
   ```

3. **Add to ReleaseType Enum**

   ```rust
   // src/config.rs
   pub enum ReleaseType {
       // ... existing types
       NewLang,
   }

   impl From<ReleaseType> for Framework {
       fn from(value: ReleaseType) -> Self {
           match value {
               // ... existing mappings
               ReleaseType::NewLang => Framework::NewLang,
           }
       }
   }
   ```

4. **Add Tests**

   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[tokio::test]
       async fn test_newlang_update() {
           let package = Package {
               name: "test".to_string(),
               path: ".".to_string(),
               framework: Framework::NewLang,
               next_version: Tag {
                   sha: "abc123".to_string(),
                   name: "v1.0.0".to_string(),
                   semver: SemVer::parse("1.0.0").unwrap(),
               },
           };

           let updater = NewLangUpdater::new();
           let mock_loader = MockFileLoader::new();
           let result = updater.update(vec![package], &mock_loader).await;

           assert!(result.is_ok());
       }
   }
   ```

### Adding Forge Platform Support

To add support for a new Git forge platform:

1. **Create Platform Module**

   ```rust
   // src/forge/newforge.rs
   use crate::forge::traits::ForgeClient;
   use crate::forge::types::{PullRequest, Release};

   pub struct NewForgeClient {
       config: RemoteConfig,
       client: reqwest::Client,
   }

   #[async_trait]
   impl ForgeClient for NewForgeClient {
       async fn create_pull_request(
           &self,
           pr: &PullRequest
       ) -> Result<String> {
           // Implementation
       }

       async fn create_release(
           &self,
           release: &Release
       ) -> Result<String> {
           // Implementation
       }
   }
   ```

2. **Add CLI Integration**

   ```rust
   // src/cli.rs
   pub struct Args {
       // ... existing fields

       #[arg(long, default_value = "", global = true)]
       /// NewForge repository URL
       pub newforge_repo: String,

       #[arg(long, default_value = "", global = true)]
       /// NewForge authentication token
       pub newforge_token: String,
   }
   ```

3. **Update Command Logic**
   ```rust
   // src/command/common.rs
   pub fn determine_forge_client(
       args: &Args
   ) -> Result<Box<dyn ForgeClient>> {
       if !args.newforge_repo.is_empty() {
           return Ok(Box::new(
               NewForgeClient::new(/* config */)?
           ));
       }
       // ... existing logic
   }
   ```

## Testing

### Test Categories

#### Unit Tests

```bash
# Run all unit tests
cargo test

# Run tests for specific module
cargo test updater::rust

# Run with output
cargo test -- --nocapture
```

#### E2E Tests

These tests can be found in
[./src/command/tests](../../src/command/tests)

```bash
# Run e2e tests (requires test tokens)
cargo test --features _internal_e2e_tests

# Run specific integration test
cargo test --features _internal_e2e_tests github_e2e_test

# These tests create actual repositories and releases
# Use with caution and dedicated test accounts
```

#### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```bash
# Features
feat: add support for Go projects
feat(forge): add Codeberg integration

# Bug fixes
fix: resolve GitHub API rate limiting
fix(updater): handle malformed package.json files

# Documentation
docs: add troubleshooting guide for Windows
docs(config): clarify multi-package setup

# Tests
test: add integration tests for GitLab
test(updater): improve Python detection tests

# Chores
chore: update dependencies
chore(ci): add automated security scanning
```

#### Pull Request Description Template

```markdown
## Description

Brief description of changes.

## Type of Change

- [ ] Bug fix (non-breaking change fixing an issue)
- [ ] New feature (non-breaking change adding functionality)
- [ ] Breaking change (fix or feature causing existing functionality to
      change)
- [ ] Documentation update

## Testing

- [ ] Unit tests pass
- [ ] Integration tests pass (if applicable)
- [ ] Manual testing completed
- [ ] Documentation tested (if applicable)

## Related Issues

Fixes #123
Relates to #456

## Additional Notes

Any additional context or screenshots.
```

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
- **Debug Mode**: Use `--debug` flag or `RELEASAURUS_DEBUG` environment variable for troubleshooting

Thank you for contributing to Releasaurus!

[Rust Code of Conduct]: https://www.rust-lang.org/policies/code-of-conduct
