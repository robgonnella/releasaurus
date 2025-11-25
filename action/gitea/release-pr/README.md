# 🦕 Releasaurus Release PR Action

Runs `releasaurus release-pr` to automate the creation and management of release pull requests. This action analyzes commits, determines version bumps, updates version files, and generates changelogs.

## Usage

```yaml
- uses: robgonnella/releasaurus/action/gitea/release-pr@vX.X.X
  with:
    token: ${{ secrets.GITEA_TOKEN }}
```

## Inputs

| Name    | Description                                           | Default                                           | Required |
| ------- | ----------------------------------------------------- | ------------------------------------------------- | -------- |
| `repo`  | The Gitea repository URL to affect                    | `${{ gitea.server_url }}/${{ gitea.repository }}` | No       |
| `token` | Gitea token with permissions to create PRs and labels |                                                   | Yes      |
| `debug` | Enable debug logs                                     | `false`                                           | No       |

## What This Action Does

- Analyzes commits since the last release
- Determines the appropriate version bump (patch, minor, major)
- Updates version files across the project
- Generates a changelog from commit history
- Creates or updates a release pull request
- Supports prerelease versions (alpha, beta, rc, etc.)
