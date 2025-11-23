# GitHub Actions Integration

Releasaurus provides seamless integration with GitHub Actions through official actions that automate your release workflow directly in your GitHub repository, eliminating the need to run Releasaurus commands manually.

## Available Actions

Releasaurus provides three GitHub Actions:

- **[Workflow Action](https://github.com/robgonnella/releasaurus/tree/main/action/github)** - Composite action that runs both `release-pr` and `release` (recommended for most users)
- **[Release PR Action](https://github.com/robgonnella/releasaurus/tree/main/action/github/release-pr)** - Creates and manages release pull requests
- **[Release Action](https://github.com/robgonnella/releasaurus/tree/main/action/github/release)** - Publishes releases after PR merge

For detailed input options and advanced usage, see the individual action READMEs linked above.
