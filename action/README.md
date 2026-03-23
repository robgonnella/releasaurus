# Releasaurus Action

Run [Releasaurus](https://releasaurus.rgon.io) commands in GitHub
Actions and Gitea Actions workflows.

## Inputs

| Input          | Required | Description                      |
| -------------- | -------- | -------------------------------- |
| `command`      | Yes      | The releasaurus command to run   |
| `command_args` | No       | Arguments to pass to the command |

## Known Limitations

### Gitea: Closed Release PRs on Repeated Runs (Gitea < 1.26)

Gitea versions prior to **1.26** do not support force-pushing a branch via
the API. As a workaround, releasaurus deletes the release branch and
re-creates it on each run. Gitea automatically closes any open pull request
targeting a deleted branch, so each run produces a new PR and leaves a
closed one behind.

**Workaround**: Use `--local-path` (hybrid mode) to perform git operations
locally. This bypasses the branch-deletion workaround and avoids accumulating
closed PRs. See the [Using `--local-path`](#using---local-path-hybrid-mode)
example below, substituting `--forge gitea` and your Gitea repository URL.

This limitation will be resolved once Gitea 1.26 is released and a
corresponding releasaurus update ships. See
[PR #200](https://github.com/robgonnella/releasaurus/pull/200).

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

### Using `--local-path` (Hybrid Mode)

When passing `--local-path` to use a local clone for git operations,
the checkout must include the full commit history and all tags back
to the previous release. Use `fetch-depth: 0` on the checkout step:

```yaml
name: Release
on:
  push:
    branches: [main]

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
        with:
          fetch-depth: 0 # required for --local-path
      - name: Create Release PR
        uses: robgonnella/releasaurus/action@vX.X.X
        with:
          command: release-pr
          command_args: >-
            --forge github
            --repo ${{ github.server_url }}/${{ github.repository }}
            --local-path ${{ github.workspace }}
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

## Documentation

Full documentation: [https://releasaurus.rgon.io](https://releasaurus.rgon.io)
