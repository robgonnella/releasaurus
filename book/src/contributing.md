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
