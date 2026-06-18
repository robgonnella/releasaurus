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

> ⚠️ **On Gitea and Forgejo runners (including Codeberg), set your
> token via `env: RELEASAURUS_FORGEJO_TOKEN` /
> `env: RELEASAURUS_GITEA_TOKEN` — not the bare `FORGEJO_TOKEN` /
> `GITEA_TOKEN`.**
>
> These runners automatically inject an ephemeral, limited per-job
> token into the environment under `GITHUB_TOKEN`, `GITEA_TOKEN`, and
> `FORGEJO_TOKEN`. A token you set via step `env:` under one of those
> bare names can be shadowed by the runner's value, so Releasaurus ends
> up using the limited token. It can read the repo (so the run starts
> fine) but cannot open a pull request on a **private** repo —
> Gitea/Forgejo return a `404 Not Found` on `.../pulls`. Public repos
> hide the issue.
>
> Releasaurus reads the `RELEASAURUS_`-prefixed variable before the
> bare name, and the runner doesn't inject the prefixed name, so it
> can't be shadowed. (Passing `--token ${{ secrets.RELEASE_TOKEN }}`
> works too — it beats every env var — but command args are more
> likely to surface in CI logs.) See the
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

On Gitea and Forgejo runners (including Codeberg), set the token via
`env: RELEASAURUS_FORGEJO_TOKEN` / `env: RELEASAURUS_GITEA_TOKEN`
rather than the bare `FORGEJO_TOKEN` / `GITEA_TOKEN` — see
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
            --local-path ${{ github.workspace }}
        env:
          RELEASAURUS_FORGEJO_TOKEN: ${{ secrets.RELEASE_TOKEN }}
```

## Documentation

Full documentation: [https://releasaurus.rgon.io](https://releasaurus.rgon.io)
