# Releasaurus Action

Run [Releasaurus](https://releasaurus.rgon.io) commands in GitHub
Actions and Gitea Actions workflows.

## Inputs

| Input          | Required | Description                      |
| -------------- | -------- | -------------------------------- |
| `command`      | Yes      | The releasaurus command to run   |
| `command_args` | No       | Arguments to pass to the command |

## Examples

### Basic Release Workflow

```yaml
name: Release
on:
  push:
    branches: [main]

jobs:
  release:
    runs-on: ubuntu-latest
    permissions:
      contents: write
      pull-requests: write
    steps:
      # Run release before release-pr to ensure pending releases are
      # published first
      - name: Publish Release
        uses: robgonnella/releasaurus/action@vX.X.X
        with:
          command: release
          command_args: >-
            --forge github
            --repo ${{ github.server_url }}/${{ github.repository }}
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

      - name: Create Release PR
        uses: robgonnella/releasaurus/action@vX.X.X
        with:
          command: release-pr
          command_args: >-
            --forge github
            --repo ${{ github.server_url }}/${{ github.repository }}
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

## Documentation

Full documentation: [https://releasaurus.rgon.io](https://releasaurus.rgon.io)
