# 🦕 Releasaurus GitHub Action

Automates the complete release workflow by running both `release-pr` and
`release` commands. This is a composite action that manages release pull requests and publishes releases for GitHub repositories.

## Usage

```yaml
- uses: robgonnella/releasaurus/action/github@vX.X.X
  with:
    token: ${{ secrets.GITHUB_TOKEN }}
```

## Inputs

| Name             | Description                                                             | Default                                             | Required |
| ---------------- | ----------------------------------------------------------------------- | --------------------------------------------------- | -------- |
| `repo`           | The GitHub repository URL to affect                                     | `${{ github.server_url }}/${{ github.repository }}` | No       |
| `token`          | GitHub token with permissions to create PRs, tags, labels, and releases |                                                     | Yes      |
| `debug`          | Enable debug logs                                                       | `false`                                             | No       |
| `git_user_name`  | Git user name for commits and tags                                      | `ReleasaurusCI`                                     | No       |
| `git_user_email` | Git user email for commits and tags                                     | `releasaurus-ci@noreply.com`                        | No       |

## What This Action Does

This composite action runs both:

1. [`release-pr`](./release-pr) - Creates or updates release pull requests
2. [`release`](./release) - Publishes releases when on a release commit

For more granular control, use the individual sub-actions directly.
