# GitLab CI/CD Integration

Releasaurus provides seamless integration with GitLab CI/CD through official components that automate your release workflow directly in your GitLab project, eliminating the need to run Releasaurus commands manually.

## Available Components

Releasaurus provides three GitLab CI components:

- **[Workflow Component](https://github.com/robgonnella/releasaurus/tree/main/templates/workflow)** - Composite component that includes both `release-pr` and `release` (recommended for most users)
- **[Release PR Component](https://github.com/robgonnella/releasaurus/tree/main/templates/release-pr)** - Creates and manages release merge requests
- **[Release Component](https://github.com/robgonnella/releasaurus/tree/main/templates/release)** - Publishes releases after MR merge

For detailed input options and advanced usage, see the individual component READMEs linked above.
