# Contributing

Welcome to Releasaurus! We appreciate your interest in contributing to making software releases effortless for everyone. This guide will help you get started with contributing to the project, whether you're fixing bugs, adding features, improving documentation, or helping with community support.

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
│   ├── analyzer/         # Commit analysis and changelog generation
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
        .with_context(|| format!("Failed to write version file: {}", path.display()))?;
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
       fn update(&self, root_path: &Path, packages: Vec<Package>) -> Result<()> {
           // Implementation
       }
   }
   ```

2. **Add Detection Logic**

   ```rust
   // src/updater/detection/newlang.rs
   use crate::updater::detection::traits::FrameworkDetector;

   pub struct NewLangDetector;

   impl FrameworkDetector for NewLangDetector {
     fn detect(&self, path: &Path) -> Result<FrameworkDetection> {
         let pattern = DetectionPattern {
             manifest_files: vec!["<some_manifest_file>"],
             support_files: vec!["<some_support_file>", "<another_support_file>"],
             content_patterns: vec![
                 "[package]",
                 "[workspace]",
                 "[dependencies]",
             ],
             base_confidence: 0.8,
         };

         DetectionHelper::analyze_with_pattern(
             path,
             pattern.clone(),
             |support_evidence| FrameworkDetection {
                 framework: Framework::NewLang,
                 confidence: DetectionHelper::calculate_confidence(
                     &pattern,
                     &support_evidence,
                 ),
                 evidence: support_evidence,
             },
         )
     }
   }
   ```

3. **Register in Framework System**

   ```rust
   // src/updater/framework.rs
   pub enum Framework {
       // ... existing frameworks
       NewLang,
   }

   impl Framework {
       pub fn detection_manager(root_path: PathBuf) -> DetectionManager {
           let detectors: Vec<Box<dyn FrameworkDetector>> = vec![
               // ... existing detectors
               Box::new(NewLangDetector::new()),
           ];
           DetectionManager::new(root_path, detectors)
       }
   }
   ```

4. **Add Tests**

   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       use tempfile::TempDir;

       #[test]
       fn test_newlang_detection() {
           let temp_dir = TempDir::new().unwrap();
           let path = temp_dir.path();

           // Create test files
           std::fs::write(path.join("package.newlang"), "version = \"1.0.0\"").unwrap();

           let detector = NewLangDetector::new();
           let result = detector.detect(path);

           assert!(matches!(result, DetectionResult::Detected { .. }));
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
       async fn create_pull_request(&self, pr: &PullRequest) -> Result<String> {
           // Implementation
       }

       async fn create_release(&self, release: &Release) -> Result<String> {
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
   pub fn determine_forge_client(args: &Args) -> Result<Box<dyn ForgeClient>> {
       if !args.newforge_repo.is_empty() {
           return Ok(Box::new(NewForgeClient::new(/* config */)?));
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

#### Integration Tests

```bash
# Run integration tests (requires test tokens)
cargo test --test integration

# Run specific integration test
cargo test --test integration test_github_workflow
```

#### End-to-End Tests

```bash
# Run e2e tests with special feature flag
cargo test --features _internal_e2e_tests

# These tests create actual repositories and releases
# Use with caution and dedicated test accounts
```

### Writing Tests

#### Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_version_update() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_file = temp_dir.path().join("Cargo.toml");

        std::fs::write(&cargo_file, r#"
[package]
name = "test"
version = "1.0.0"
"#).unwrap();

        let updater = CargoUpdater::new();
        let result = updater.update_version(&cargo_file, "1.1.0");

        assert!(result.is_ok());

        let content = std::fs::read_to_string(&cargo_file).unwrap();
        assert!(content.contains(r#"version = "1.1.0""#));
    }
}
```

#### Integration Test Example

```rust
#[tokio::test]
#[ignore] // Only run with --ignored flag
async fn test_github_integration() {
    let token = std::env::var("GITHUB_TOKEN")
        .expect("GITHUB_TOKEN required for integration tests");

    let client = GitHubClient::new(/* config with test repo */);

    // Test API functionality
    let result = client.create_pull_request(&test_pr).await;
    assert!(result.is_ok());

    // Clean up test data
    // ...
}
```

### Test Data Management

Create test fixtures in the `tests/fixtures` directory:

```
tests/
├── fixtures/
│   ├── rust-project/
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   ├── node-project/
│   │   ├── package.json
│   │   └── index.js
│   └── multi-package/
│       ├── releasaurus.toml
│       ├── frontend/package.json
│       └── backend/Cargo.toml
└── integration/
    ├── github_tests.rs
    └── gitlab_tests.rs
```

## Documentation Contributions

### Documentation Structure

The documentation is built with [mdBook](https://rust-lang.github.io/mdBook/):

```bash
# Install mdBook
cargo install mdbook

# Serve documentation locally
cd book
mdbook serve

# Build documentation
mdbook build
```

### Writing Documentation

#### Style Guidelines

- Use clear, concise language
- Include practical examples
- Add code snippets for complex concepts
- Test all code examples

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
- [ ] Breaking change (fix or feature causing existing functionality to change)
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

### Our Standards

- **Be respectful**: Treat everyone with respect and kindness
- **Be inclusive**: Welcome people of all backgrounds and experience levels
- **Be constructive**: Provide helpful feedback and suggestions
- **Be patient**: Remember that everyone is learning
- **Be collaborative**: Work together toward common goals

### Unacceptable Behavior

- Harassment, discrimination, or offensive comments
- Personal attacks or trolling
- Spam or off-topic discussions
- Sharing private information without permission

### Reporting

Report any unacceptable behavior to the project maintainers. All reports will be handled confidentially.

## Community and Communication

### Channels

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: General questions and community support
- **Pull Requests**: Code review and collaboration

### Getting Help

- **Documentation**: Start with this book
- **Search Issues**: Check if your question has been asked
- **Ask Questions**: Create a discussion or issue
- **Debug Mode**: Use `--debug` for troubleshooting

### Contributor Recognition

Contributors are recognized through:

- Changelog entries for significant contributions
- Contributor list in repository
- Social media acknowledgments for major features

## Questions?

Don't hesitate to ask questions! We're here to help:

- **General questions**: GitHub Discussions
- **Specific issues**: GitHub Issues
- **Code questions**: Comments in pull requests

Thank you for contributing to Releasaurus!
