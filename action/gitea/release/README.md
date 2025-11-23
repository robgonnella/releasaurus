# 🦕 Releasaurus Release Action

Runs `releasaurus release` to automate Git tag creation and Gitea release publication. This action should run after a release PR has been merged.

## Usage

```yaml
- uses: robgonnella/releasaurus/action/gitea/release@vX.X.X
  with:
    token: ${{ secrets.GITEA_TOKEN }}
```

## Inputs

| Name             | Description                                              | Default                                           | Required |
| ---------------- | -------------------------------------------------------- | ------------------------------------------------- | -------- |
| `repo`           | The Gitea repository URL to affect                       | `${{ gitea.server_url }}/${{ gitea.repository }}` | No       |
| `token`          | Gitea token with permissions to create tags and releases |                                                   | Yes      |
| `debug`          | Enable debug logs                                        | `false`                                           | No       |
| `git_user_name`  | Git user name for creating tags                          | `ReleasaurusCI`                                   | No       |
| `git_user_email` | Git user email for creating tags                         | `releasaurus-ci@noreply.com`                      | No       |

## What This Action Does

- Validates that the current commit is a release commit
- Creates a Git tag for the new version
- Pushes the tag to the repository
- Creates a Gitea release with generated changelog
