# Commands

Releasaurus operates entirely through forge platform APIs — no local
clone required — so every command can run from any machine with network
access to your forge. An optional
[hybrid mode](#hybrid-mode-local-git--remote-forge) uses a local clone
for git operations.

The core workflow is two commands:

```bash
# 1. Prepare: analyze commits, bump versions, write changelog, open a PR
releasaurus release-pr --repo "https://github.com/owner/repo"

# 2. Review and merge the PR in your forge's UI, then publish:
releasaurus release --repo "https://github.com/owner/repo"
```

`start-next` and `get` are optional helpers covered below.

## `release-pr`

Analyzes commits since the last release, determines the version bump
(patch/minor/major) from conventional commits, updates version files (if
a `release_type` is configured), generates the changelog, and creates or
updates a release pull request.

```bash
# All packages
releasaurus release-pr --repo "https://github.com/owner/repo"

# A single package in a monorepo
releasaurus release-pr --package my-pkg \
  --repo "https://github.com/owner/repo"
```

Supports prereleases, dry-run, and the [overrides](#configuration-overrides)
below.

## `release`

Run after the release PR is merged. Validates the release commit, creates
and pushes the git tag, and publishes the release on your forge. Reads
the release notes directly from the merged PR body (see
[Editing Release Notes](./release-notes-editing.md)).

```bash
# All packages with merged release PRs
releasaurus release --repo "https://github.com/owner/repo"

# A single package
releasaurus release --package my-pkg \
  --repo "https://github.com/owner/repo"
```

## `start-next`

Bumps the patch version for each previously-tagged package and commits
the manifest changes **directly to the base branch** as a `chore` commit.
It does not open PRs or create tags, and skips packages that have never
been tagged. Use it right after a release to keep manifest versions ahead
of the last release.

```bash
# All previously-tagged packages
releasaurus start-next --repo "https://github.com/owner/repo"

# Specific packages only
releasaurus start-next --repo "https://github.com/owner/repo" \
  --packages pkg-a,pkg-b
```

> **Note:** This commits directly to your base branch. Ensure your branch
> protection rules permit it. It can also run automatically after
> `release` — see
> [`auto_start_next`](./configuration-reference.md#global-settings).

## `get`

Queries release information as JSON without making any changes — useful
for debugging version detection and for building custom notifications.
(`show` is kept as an alias.)

### `get next-release`

Projects the next release for each package as JSON.

```bash
releasaurus get next-release --repo "https://github.com/owner/repo"

# Single package, or write to a file
releasaurus get next-release --package my-pkg --out-file releases.json \
  --repo "https://github.com/owner/repo"
```

### `get current-release`

Returns the most recent release for each package (packages without a
release are omitted).

```bash
releasaurus get current-release --repo "https://github.com/owner/repo"
```

### `get release`

Returns the data for an existing tag — `tag`, `sha`, and `notes`.

```bash
releasaurus get release --tag v1.0.0 \
  --repo "https://github.com/owner/repo"
```

### `get notes`

Re-renders release notes from a `get next-release` JSON file using your
configured Tera template. This lets you transform the data (for example,
replacing author names with Slack IDs) before producing final notes.
(`recompiled-notes` is kept as an alias.)

```bash
# 1. Capture release data
releasaurus get next-release --out-file releases.json \
  --repo "https://github.com/owner/repo"

# 2. Transform it however you like (custom script), then re-render:
releasaurus get notes --file releases.json \
  --repo "https://github.com/owner/repo"
```

Output is a JSON array of `{ name, notes }` objects.

## Global Options & Forge Selection

These apply to every command:

| Flag                     | Env fallback                                 | Description                 |
| ------------------------ | -------------------------------------------- | --------------------------- |
| `--repo <url>`           | `RELEASAURUS_REPO`                           | Repository URL              |
| `--forge <forge>`        | `RELEASAURUS_FORGE`                          | Forge type (see below)      |
| `--token <token>`        | `RELEASAURUS_<FORGE>_TOKEN`, `<FORGE>_TOKEN` | Auth token                  |
| `--local-path <path>`    | `RELEASAURUS_LOCAL_PATH`                     | Local clone for hybrid mode |
| `--base-branch <branch>` | —                                            | Override the base branch    |
| `--debug`                | `RELEASAURUS_DEBUG`                          | Verbose logging             |
| `--config`               | `RELEASAURUS_CONFIG`                         | Custom file path location   |

Available forge types: `github`, `gitlab`, `gitea`, `forgejo`,
`azure-devops` (experimental), and `local` (testing). For the full list
of token variables and required scopes, see the
[Configuration Reference](./configuration-reference.md#environment-variables).

### Automatic forge inference

When `--repo` points at a recognized cloud host, `--forge` can be
omitted:

| Host            | Inferred forge |
| --------------- | -------------- |
| `github.com`    | `github`       |
| `gitlab.com`    | `gitlab`       |
| `gitea.com`     | `gitea`        |
| `codeberg.org`  | `forgejo`      |
| `dev.azure.com` | `azure-devops` |

Self-hosted instances (e.g. `https://gitlab.company.com/...`) and
`--forge local` always require the flag, since the host alone can't
identify the forge software.

## Testing Modes

Three ways to run safely or against a local checkout.

### Dry-Run Mode

Performs all analysis and validation and logs exactly what _would_
happen, but makes no changes — no branches, PRs, tags, or releases.
Dry-run automatically enables debug logging (output is prefixed
`dry_run:`).

```bash
releasaurus release-pr --dry-run --repo "https://github.com/owner/repo"

# Or via environment variable
export RELEASAURUS_DRY_RUN=true
```

### Local Repository Mode

`--forge local` reads commits, tags, and files from your working
directory and never contacts a remote forge — ideal for validating a
`releasaurus.toml` change before pushing. No token required.

```bash
releasaurus release-pr --forge local --repo "."

# Or from a specific path
releasaurus release-pr --forge local --repo "/path/to/repo"
```

### Hybrid Mode (Local Git + Remote Forge)

`--local-path` performs git operations (reading commits/tags/files,
creating branches, committing, pushing) against a local clone, while
still creating real PRs and releases via the forge API. Use it when you
already have a checkout and want to avoid repeated API calls for data
gathering. A forge token is still required.

```bash
releasaurus release-pr \
  --repo "https://github.com/owner/repo" \
  --token "$GITHUB_TOKEN" \
  --local-path /path/to/checkout
```

> **CI fetch depth:** in hybrid mode the local checkout must include full
> history and all tags back to the previous release. Most CI systems
> shallow-clone by default — set `fetch-depth: 0` (GitHub/Gitea Actions)
> or `GIT_DEPTH: 0` (GitLab CI), or run `git fetch --unshallow`. See
> [CI/CD Integration](./ci-cd-integration.md) for per-platform setup.

## Configuration Overrides

Override config from the command line without editing
`releasaurus.toml` — handy for testing, one-off releases, and per-branch
CI settings.

| Flag                                        | Effect                                         |
| ------------------------------------------- | ---------------------------------------------- |
| `--base-branch <branch>`                    | Override the base branch                       |
| `--tag-prefix <prefix>`                     | Global tag prefix for all packages             |
| `--prerelease-suffix <suffix>`              | Global prerelease suffix (empty `""` disables) |
| `--prerelease-strategy <versioned\|static>` | Global prerelease strategy                     |
| `--skip-sha <sha>`                          | Skip a commit by SHA prefix (repeatable)       |
| `--reword <sha>=<message>`                  | Rewrite a commit message (repeatable)          |
| `--set-package <pkg>.<property>=<value>`    | Per-package override (repeatable)              |

`--set-package` takes precedence over all other overrides and config.
Supported properties: `tag_prefix`, `prerelease.suffix`,
`prerelease.strategy`. Setting an unsupported property prints an error
listing valid values.

**Precedence (highest to lowest):** `--set-package` → global CLI
overrides → package config → global config → defaults.

```bash
# Override base branch and global prerelease suffix
releasaurus release-pr --base-branch develop --prerelease-suffix beta \
  --repo "https://github.com/owner/repo"

# Per-package override (e.g. only the frontend gets a beta suffix)
releasaurus release-pr --set-package frontend.prerelease.suffix=beta \
  --repo "https://github.com/owner/repo"

# Skip one commit and reword another
releasaurus release-pr --skip-sha abc123d \
  --reword "def456e=feat: improved authentication" \
  --repo "https://github.com/owner/repo"
```

See [Configuration](./configuration.md) for what these settings mean.

## Known Limitations

### Forgejo: Closed Release PRs on Repeated Runs

Forgejo's API does not support force-pushing a branch. As a workaround,
Releasaurus deletes and re-creates the release branch on each run;
Forgejo auto-closes the PR targeting the deleted branch, so each run
leaves a closed PR behind. Use
[hybrid mode](#hybrid-mode-local-git--remote-forge) (`--local-path`) to
avoid this. A patch to Forgejo to allow force pushing the release branch has
been accepted and will be available in v16.
<https://codeberg.org/forgejo/forgejo/pulls/12663>

### Gitea and Forgejo Actions: Injected Token Shadows Your PAT

Gitea and Forgejo Actions runners (including Codeberg) automatically
inject an ephemeral, limited per-job token into the job environment
under the names `GITHUB_TOKEN`, `GITEA_TOKEN`, and `FORGEJO_TOKEN`. If
you supply your own token through one of those environment variables
— for example `env: FORGEJO_TOKEN: ${{ secrets.RELEASE_TOKEN }}` — the
runner's injected value can take precedence inside the action, and
Releasaurus authenticates with the limited token instead of your PAT.

That injected token can usually _read_ the repository, so startup
succeeds, but it cannot create a pull request on a **private** repo.
Gitea/Forgejo return `404 Not Found` for the unauthorized write
against the `.../pulls` endpoint, which is easy to misread as a
missing repository. Public repos hide the problem because reads are
anonymous.

**Fix (recommended):** supply your token through the
`RELEASAURUS_`-prefixed environment variable — e.g.
`RELEASAURUS_FORGEJO_TOKEN` for Forgejo, `RELEASAURUS_GITEA_TOKEN` for
Gitea. Releasaurus reads it _before_ the bare `*_TOKEN` name, and the
runner does not inject the prefixed name, so it can't be shadowed:

```yaml
env:
  RELEASAURUS_FORGEJO_TOKEN: ${{ secrets.RELEASE_TOKEN }}
```

**Alternative:** pass the token on the command line with `--token`,
which takes precedence over every environment variable. Note that
command arguments are more likely to appear in CI logs than env vars:

```yaml
command_args: >-
  --forge forgejo
  --repo ${{ github.server_url }}/${{ github.repository }}
  --token ${{ secrets.RELEASE_TOKEN }}
```

### Azure DevOps: Release Branch Requires "Allow rewriting history"

When updating a release PR, Releasaurus resets the release branch to the
tip of the base branch and replays the changelog commit. If the existing
release branch has diverged, this is a non-fast-forward update that Azure
DevOps rejects unless **Allow rewriting history** is granted on the
release branch (typically `releasaurus-release-*`).

Grant it under **Project Settings → Repositories → {repo} → Security →
Branches → {release branch}**, setting **Allow rewriting history** to
_Allow_ for the identity holding the PAT. Azure DevOps `release` also only
pushes the git tag — there is no native release object, so no release
notes page is published.

## Getting Help

```bash
releasaurus --help          # general help
releasaurus <cmd> --help    # command-specific help
releasaurus --version       # version information
```
