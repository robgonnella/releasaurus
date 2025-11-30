# Why Releasaurus?

The software development landscape is rich with release automation tools, each
solving specific problems within their domains. So why create another one?
Releasaurus was born from real-world frustrations with existing tools and a
vision for what release automation could be:
**universal, intelligent, and effortless**.

## The Release Automation Challenge

Modern software development involves complex release processes that
traditionally require significant manual effort:

- **Version Management**: Updating version numbers across multiple files and formats
- **Changelog Generation**: Creating meaningful release notes from commit history
- **Platform Integration**: Working with different Git forge APIs (GitHub, GitLab, Gitea)
- **Multi-Language Support**: Handling different packaging systems and conventions
- **Review Workflows**: Ensuring releases go through proper review processes
- **Consistency**: Maintaining uniform practices across projects and teams

While several excellent tools address parts of this challenge, none provided a
complete, universal solution that works seamlessly across all environments.

## Quick Comparison Table

| Tool                 | Platforms             | Languages                                       | Config Required   | Process              | Best For                                 |
| -------------------- | --------------------- | ----------------------------------------------- | ----------------- | -------------------- | ---------------------------------------- |
| **Releasaurus**      | GitHub, GitLab, Gitea | Rust, Node.js, Python, Java, PHP, Ruby, Generic | Minimal           | Release PR → Publish | Universal, multi-platform, simple config |
| **release-please**   | GitHub only           | Many (via plugins)                              | Required for most | Release PR → Publish | GitHub-only orgs                         |
| **release-plz**      | GitHub, crates.io     | Rust only                                       | Minimal           | Release PR → Publish | Rust projects, Cargo ecosystems          |
| **git-cliff**        | Any Git host          | Language agnostic                               | Required          | Manual changelog     | Changelog generation only                |
| **semantic-release** | GitHub, GitLab, npm   | JavaScript-focused                              | Required          | Direct publish       | Node.js projects, automated releases     |

## The Existing Landscape

Before Releasaurus, developers had to choose between specialized tools, each
with significant limitations:

### release-please: GitHub-Only Excellence

[release-please](https://github.com/googleapis/release-please) pioneered the
release-PR workflow and remains excellent for GitHub-centric organizations.
However:

- **Platform Lock-in**: Only works with GitHub, excluding GitLab and Gitea users

### release-plz: Rust Ecosystem Specialist

[release-plz](https://release-plz.ieni.dev/) provides outstanding Rust support
with deep Cargo integration. But:

- **Single Language**: Only supports Rust projects
- **Ecosystem Specific**: Designed exclusively for the Rust/Cargo ecosystem

### git-cliff: Powerful but Manual

[git-cliff](https://git-cliff.org/) excels at changelog generation with
extensive customization and inspired our approach to commit parsing and
changelog formatting. However, as a standalone tool it has some limitations for
complete release automation:

- **Changelog Only**: Doesn't handle version updates or release automation
- **Manual Process**: Requires additional tooling for complete release workflows
- **No Platform Integration**: Doesn't create releases or pull requests

## Releasaurus

Releasaurus was designed to combine the best aspects of existing tools while
eliminating their limitations. Our core principles:

### 🌍 Universal Platform Support

**Problem**: Teams using GitLab, Gitea, or mixed environments were left behind
by GitHub-only tools.

**Solution**: First-class support for GitHub, GitLab, and Gitea with identical
workflows across all platforms. Whether your repositories are hosted on
github.com, self-hosted GitLab, or Gitea instances, Releasaurus works
seamlessly.

```bash
# Same workflow, different platforms
releasaurus release-pr --github-repo "https://github.com/team/project"
releasaurus release-pr --gitlab-repo "https://gitlab.company.com/team/project"
releasaurus release-pr --gitea-repo "https://git.company.com/team/project"
```

### 🔍 Simple Configuration

**Problem**: Complex setup requirements create barriers to adoption and
maintenance overhead.

**Solution**: Straightforward configuration that gets you started quickly.
Specify your project's `release_type` once in `releasaurus.toml` and
Releasaurus handles all version file updates with sensible defaults.

- **Clear Language Specification**: Configure `release_type` for each package
  (Rust, Node, Python, Java, Php, Ruby, or Generic)
- **Version File Management**: Updates all relevant version files for the
  specified language (see
  [`additional_manifest_files`](./configuration.md#`additional_manifest_files`)
  for generic projects)
- **Framework Integration**: Handles language-specific packaging and build
  systems
- **Sensible Defaults**: Provides beautiful changelogs and workflows
  out-of-the-box

### 🚀 Multi-Language Native Support

**Problem**: Polyglot projects and organizations needed different tools for
different languages.

**Solution**: Native support for major programming languages with deep
understanding of each ecosystem's manifest files

| Language    | Package Files                | Lock Files                       |
| ----------- | ---------------------------- | -------------------------------- |
| **Rust**    | `Cargo.toml`                 | `Cargo.lock`                     |
| **Node.js** | `package.json`               | `package-lock.json`, `yarn.lock` |
| **Python**  | `pyproject.toml`, `setup.py` | Various                          |
| **Java**    | `pom.xml`, `build.gradle`    | Maven/Gradle locks               |
| **PHP**     | `composer.json`              | `composer.lock`                  |
| **Ruby**    | `*.gemspec`, `Gemfile`       | `Gemfile.lock`                   |

## The Philosophy

Releasaurus embodies several key philosophical principles:

### Simplicity Over Complexity

Releasaurus requires minimal configuration—just specify your project's
`release_type` and let the tool handle the rest. Version file patterns,
changelog generation, and release workflows all work with sensible defaults.

### Universal Over Specialized

While specialized tools excel in narrow domains, Releasaurus prioritizes
universal applicability. This enables consistent workflows across diverse
projects and teams.

### Simplicity Over Features

Every feature must justify its existence by solving a real problem without
adding unnecessary complexity. Power users have customization options, but they
don't interfere with simple use cases.

## Credit to existing projects

Releasaurus wouldn't exist without the pioneering work of existing tools:

- **[git-cliff](https://git-cliff.org/)**
- **[release-please](https://github.com/googleapis/release-please)**
- **[release-plz](https://release-plz.ieni.dev/)**

We're grateful for these tools and the problems they solved. Releasaurus builds
upon their innovations while implementing our own solutions to address the gaps
that remained.

## When to Choose Releasaurus

**Choose Releasaurus if you:**

- Work with multiple programming languages or frameworks
- Use GitLab, Gitea, or mixed Git hosting platforms
- Want release automation that works immediately without complex setup
- Need consistent workflows across diverse projects
- Value safety and review processes in your release workflow
- Want to minimize time spent on release mechanics

**Consider alternatives if you:**

- Are deeply committed to GitHub-only workflows and love release-please
- Need features specific to a single language ecosystem
- Require extensive customization beyond what templates provide
- Have unique release requirements not covered by conventional patterns

## The Future

Releasaurus continues to evolve based on real-world usage and community
feedback. Our roadmap includes:

- **Additional Language Support**: Expanding to more programming languages and frameworks
- **Enhanced CI/CD Integration**: Deeper integration with popular CI/CD platforms
- **Advanced Monorepo Features**: More sophisticated dependency and release coordination
- **Custom Workflow Support**: Extensibility for unique release requirements

But our core mission remains constant: making software releases effortless,
safe, and universal.

## Getting Started

Ready to experience effortless releases? The [Quick Start](./quick-start.md)
guide will have you releasing software in minutes, not hours. Or dive into
the [Installation](./installation.md) instructions to get Releasaurus set up in
your environment.

Remember: great software deserves great releases. Let Releasaurus handle the
mechanics so you can focus on building amazing things. 🦕
