# Releasaurus Action

Run [Releasaurus](https://releasaurus.rgon.io) commands in GitHub
Actions and Gitea Actions workflows.

## Inputs

| Input          | Required | Description                      |
| -------------- | -------- | -------------------------------- |
| `command`      | Yes      | The releasaurus command to run   |
| `command_args` | No       | Arguments to pass to the command |

## Required token scopes

| Forge                     | Scopes / permissions                                                                                                                      |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| **GitHub** (classic)      | `repo`                                                                                                                                    |
| **GitHub** (fine-grained) | Contents, Issues, Pull requests — all read & write. Add Actions/Workflows read & write only if using the Action to modify workflow files. |

## Authentication on Gitea / Forgejo Actions

> ⚠️ **Pass your token with `--token`, not `env: FORGEJO_TOKEN` /
> `env: GITEA_TOKEN`, on Gitea and Forgejo runners (including
> Codeberg).**
>
> These runners automatically inject an ephemeral, limited per-job
> token into the environment under `GITHUB_TOKEN`, `GITEA_TOKEN`, and
> `FORGEJO_TOKEN`. A token you set via step `env:` under one of those
> names can be shadowed by the runner's value, so Releasaurus ends up
> using the limited token. It can read the repo (so the run starts
> fine) but cannot open a pull request on a **private** repo —
> Gitea/Forgejo return a `404 Not Found` on `.../pulls`. Public repos
> hide the issue.
>
> Passing `--token ${{ secrets.RELEASE_TOKEN }}` takes precedence over
> any `*_TOKEN` environment variable and avoids the collision. See the
> [example below](#gitea--forgejo-actions).

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

### Gitea / Forgejo Actions

On Gitea and Forgejo runners (including Codeberg), pass the token with
`--token` rather than `env: FORGEJO_TOKEN` / `env: GITEA_TOKEN` — see
[the authentication note above][auth-note] for why.

[auth-note]: #authentication-on-gitea--forgejo-actions

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
            --forge forgejo
            --repo ${{ github.server_url }}/${{ github.repository }}
            --token ${{ secrets.RELEASE_TOKEN }}
            --local-path ${{ github.workspace }}
```

## Documentation

Full documentation: [https://releasaurus.rgon.io](https://releasaurus.rgon.io)
