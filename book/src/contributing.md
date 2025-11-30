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

Edit `src/config/release_type.rs`:

```rust
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
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

**3. Create the Manifest Loader**

Create `src/config/manifest/yourlanguage.rs`:

```rust
use crate::{
    Result,
    config::{
        manifest::{ManifestFile, gen_package_path},
        package::PackageConfig,
    },
    forge::manager::ForgeManager,
};

pub struct YourLanguageManifestLoader {}

impl YourLanguageManifestLoader {
    pub async fn load_manifests(
        forge: &ForgeManager,
        pkg: &PackageConfig,
    ) -> Result<Option<Vec<ManifestFile>>> {
        let files = vec!["your-manifest.ext"];
        let mut manifests = vec![];

        for file in files {
            let full_path = gen_package_path(pkg, file);
            if let Some(content) = forge.get_file_content(&full_path).await? {
                manifests.push(ManifestFile {
                    file_path: full_path,
                    file_basename: file.to_string(),
                    is_workspace: false,
                    content,
                });
            }
        }

        if manifests.is_empty() {
            return Ok(None);
        }

        Ok(Some(manifests))
    }
}
```

Register it in `src/config/manifest.rs`:

```rust
mod yourlanguage;
use yourlanguage::YourLanguageManifestLoader;

// In load_release_type_manifests_for_package function:
Some(ReleaseType::YourLanguage) => {
    YourLanguageManifestLoader::load_manifests(forge, pkg).await
}
```

**4. Create the Updater Module**

Create `src/updater/yourlanguage/` directory with the following files:

`src/updater/yourlanguage.rs`:

```rust
//! YourLanguage package updater supporting YourPackageManager projects.

pub mod your_file_type;
pub mod updater;
```

`src/updater/yourlanguage/updater.rs`:

```rust
use crate::{
    Result,
    forge::request::FileChange,
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

impl PackageUpdater for YourLanguageUpdater {
    fn update(
        &self,
        package: &UpdaterPackage,
        workspace_packages: Vec<UpdaterPackage>,
    ) -> Result<Option<Vec<FileChange>>> {
        self.your_file.process_package(package)
    }
}
```

**5. Implement File Parsers**

`src/updater/yourlanguage/your_file_type.rs`:

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

    pub fn process_package(
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

**6. Register the Updater**

Add module declaration in `src/updater.rs`:

```rust
pub mod yourlanguage;
```

In `src/updater/framework.rs`, add to the `updater()` method:

```rust
fn updater(&self) -> Box<dyn PackageUpdater> {
    match self {
        // ... existing cases ...
        Framework::YourLanguage => Box::new(YourLanguageUpdater::new()),
    }
}
```

**7. Write Tests**

Add tests following the established patterns.

In `src/config/manifest/yourlanguage.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        forge::traits::MockForge,
        test_helpers::create_test_remote_config,
    };

    // ===== Test Helpers =====

    fn package_config(path: &str, workspace_root: &str) -> PackageConfig {
        PackageConfig {
            name: "my-package".to_string(),
            path: path.to_string(),
            workspace_root: workspace_root.to_string(),
            ..Default::default()
        }
    }

    fn mock_forge_with_file(path: &str, content: &str) -> ForgeManager {
        let mut mock = MockForge::new();
        let path = path.to_string();
        let content = content.to_string();
        mock.expect_get_file_content().returning(move |p| {
            if p == path {
                Ok(Some(content.clone()))
            } else {
                Ok(None)
            }
        });
        mock.expect_remote_config()
            .returning(create_test_remote_config);
        ForgeManager::new(Box::new(mock))
    }

    // ===== Manifest Loading Tests =====

    #[tokio::test]
    async fn loads_manifest_file() {
        let pkg = package_config(".", ".");
        let forge = mock_forge_with_file("your-manifest.ext", "version = 1.0.0");

        let result = YourLanguageManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests[0].file_basename, "your-manifest.ext");
    }

    #[tokio::test]
    async fn returns_none_when_not_found() {
        let pkg = package_config(".", ".");
        let mut mock = MockForge::new();
        mock.expect_get_file_content().returning(|_| Ok(None));
        mock.expect_remote_config()
            .returning(create_test_remote_config);
        let forge = ForgeManager::new(Box::new(mock));

        let result = YourLanguageManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_none());
    }
}
```

In `src/updater/yourlanguage/updater.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::manifest::ManifestFile,
        test_helpers::create_test_tag,
        updater::framework::{Framework, UpdaterPackage},
    };

    // ===== Test Helpers =====

    fn create_manifest(content: &str) -> ManifestFile {
        ManifestFile {
            is_workspace: false,
            file_path: "your-manifest.ext".to_string(),
            file_basename: "your-manifest.ext".to_string(),
            content: content.to_string(),
        }
    }

    // ===== Update Tests =====

    #[test]
    fn updates_version_field() {
        let updater = YourLanguageUpdater::new();
        let manifest = create_manifest(r#"version = "1.0.0""#);
        let package = UpdaterPackage {
            package_name: "test-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::YourLanguage,
        };

        let result = updater.update(&package, vec![]).unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert!(changes[0].content.contains("2.0.0"));
        assert!(!changes[0].content.contains("1.0.0"));
    }

    #[test]
    fn returns_none_when_no_matching_files() {
        let updater = YourLanguageUpdater::new();
        let manifest = create_manifest("# no version here");
        let package = UpdaterPackage {
            package_name: "test-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::YourLanguage,
        };

        let result = updater.update(&package, vec![]).unwrap();

        assert!(result.is_none());
    }
}
```

**8. Update Documentation**

Add your language to:

- `book/src/supported-languages.md` - Add documentation and examples
- `book/src/configuration.md` - Update release_type options
- `README.md` - Add to supported languages list

#### Testing Principles

Follow these key principles when writing tests:

- **Test outcomes, not implementation**: Focus on what the code does, not how
- **Use helper functions**: Reduce duplication with reusable test helpers
- **Clear test names**: Use descriptive names like `loads_manifest_file` instead of `test_loader_1`
- **Minimize redundancy**: Avoid overlapping test conditions
- **Test all enum variants**: For conversions and Display implementations, test every variant

#### Running Tests

```bash
# Run all tests for your language
cargo test yourlanguage

# Run just manifest loader tests
cargo test config::manifest::yourlanguage

# Run just updater tests
cargo test updater::yourlanguage

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

#### Example Implementations to Reference

Look at these existing implementations for patterns:

- **JSON manifests**: `src/updater/node/package_json.rs` and `src/config/manifest/node.rs`
- **TOML manifests**: `src/updater/rust/cargo_toml.rs` and `src/config/manifest/rust.rs`
- **XML manifests**: `src/updater/java/maven.rs` and `src/config/manifest/java.rs`
- **Line-based files**: `src/updater/ruby/version_rb.rs` and `src/config/manifest/ruby.rs`
- **Simple loaders**: `src/config/manifest/php.rs` and `src/config/manifest/python.rs`

Each implementation includes comprehensive tests demonstrating the testing patterns.

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
