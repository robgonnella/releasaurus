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

Releasaurus supports multiple programming languages through its updater system. Here's how to add support for a new language or framework.

#### Overview

Each language updater consists of:

1. **Framework enum variant** - Identifies the language/framework
2. **ReleaseType enum variant** - Configuration option for users
3. **Updater module** - Language-specific implementation
4. **File parsers** - Handlers for version file formats
5. **Tests** - Comprehensive test coverage

#### Step-by-Step Guide

**1. Add the Framework Variant**

Edit `src/updater/framework.rs`:

```rust
pub enum Framework {
    Generic,
    // ... existing variants ...
    YourLanguage,  // Add your new language here
}
```

Update the `Display` implementation:

```rust
impl Display for Framework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // ... existing cases ...
            Framework::YourLanguage => f.write_str("yourlanguage"),
        }
    }
}
```

**2. Add the ReleaseType Configuration**

Edit `src/config.rs`:

```rust
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseType {
    // ... existing variants ...
    YourLanguage,
}
```

Update the `From<ReleaseType>` implementation in `src/updater/framework.rs`:

```rust
impl From<ReleaseType> for Framework {
    fn from(value: ReleaseType) -> Self {
        match value {
            // ... existing cases ...
            ReleaseType::YourLanguage => Framework::YourLanguage,
        }
    }
}
```

**3. Create the Updater Module**

Create `src/updater/yourlanguage.rs`:

```rust
//! YourLanguage package updater supporting YourPackageManager projects.

pub mod your_file_type;
pub mod updater;
```

Create `src/updater/yourlanguage/updater.rs`:

```rust
use async_trait::async_trait;
use crate::{
    forge::request::FileChange,
    result::Result,
    updater::{
        framework::UpdaterPackage,
        yourlanguage::your_file_type::YourFileType,
        traits::PackageUpdater,
    },
};

pub struct YourLanguageUpdater {
    your_file: YourFileType,
}

impl YourLanguageUpdater {
    pub fn new() -> Self {
        Self {
            your_file: YourFileType::new(),
        }
    }
}

#[async_trait]
impl PackageUpdater for YourLanguageUpdater {
    async fn update(
        &self,
        package: &UpdaterPackage,
        workspace_packages: Vec<UpdaterPackage>,
    ) -> Result<Option<Vec<FileChange>>> {
        self.your_file.process_package(package).await
    }
}
```

**4. Implement File Parsers**

Create `src/updater/yourlanguage/your_file_type.rs`:

```rust
use log::*;
use crate::{
    forge::request::{FileChange, FileUpdateType},
    result::Result,
    updater::framework::UpdaterPackage,
};

pub struct YourFileType {}

impl YourFileType {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn process_package(
        &self,
        package: &UpdaterPackage,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.file_basename != "your-manifest-file.ext" {
                continue;
            }

            // Parse the file content
            // Update version fields
            // Create FileChange objects

            file_changes.push(FileChange {
                path: manifest.file_path.clone(),
                content: updated_content,
                update_type: FileUpdateType::Replace,
            });
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }
}
```

**5. Register the Updater**

In `src/updater/framework.rs`, add to the `updater()` method:

```rust
fn updater(&self) -> Box<dyn PackageUpdater> {
    match self {
        // ... existing cases ...
        Framework::YourLanguage => Box::new(YourLanguageUpdater::new()),
    }
}
```

Add manifest file detection in the `manifest_files()` method:

```rust
Framework::YourLanguage => {
    vec![
        ManifestFile {
            content: "".to_string(),
            file_basename: "your-manifest.ext".into(),
            file_path: gen_package_path("your-manifest.ext"),
            is_workspace: false,
        },
    ]
}
```

**6. Add Module Declaration**

In `src/updater.rs`, add:

```rust
mod yourlanguage;
```

**7. Write Tests**

Add comprehensive tests in your updater module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_helpers::create_test_tag,
        updater::framework::{Framework, ManifestFile, UpdaterPackage},
    };

    #[tokio::test]
    async fn processes_yourlanguage_project() {
        let updater = YourLanguageUpdater::new();
        let content = r#"version = "1.0.0""#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "manifest.ext".to_string(),
            file_basename: "manifest.ext".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::YourLanguage,
        };

        let result = updater.update(&package, vec![]).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert!(changes[0].content.contains("2.0.0"));
    }
}
```

**8. Update Documentation**

Add your language to:

- `book/src/supported-languages.md` - Add documentation and examples
- `book/src/configuration.md` - Update release_type options
- `README.md` - Add to supported languages list

#### Testing Your Updater

```bash
# Run your specific tests
cargo test yourlanguage

# Test with a real repository (use --local-repo for safety)
releasaurus release-pr --local-repo "/path/to/test/project" --debug
```

#### Best Practices

- **Parse robustly**: Handle various file formats and edge cases
- **Preserve formatting**: Maintain the original file structure when possible
- **Log clearly**: Use `info!`, `warn!`, and `error!` macros appropriately
- **Test thoroughly**: Cover success cases, edge cases, and error conditions
- **Document well**: Add doc comments explaining behavior
- **Follow patterns**: Look at existing updaters (e.g., `php`, `ruby`) for examples

#### Common Patterns

**JSON-based manifests**: See `src/updater/node/package_json.rs`
**TOML-based manifests**: See `src/updater/rust/cargo_toml.rs`
**XML-based manifests**: See `src/updater/java/maven.rs`
**Line-based files**: See `src/updater/ruby/version_rb.rs`

#### Getting Help

- Review existing updater implementations in `src/updater/`
- Ask questions in GitHub Discussions
- Reference the `PackageUpdater` trait documentation
- Look at test files for usage examples

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
