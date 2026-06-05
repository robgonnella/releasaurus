# CI/CD Integration

Releasaurus provides official integrations for GitHub Actions, Gitea
Actions, and Forgejo Actions. For GitLab CI and Azure Pipelines, use
the Docker image directly.

> **Note on fetch depth:** When using `--local-path` (hybrid mode),
> Releasaurus reads commit history and tags directly from the local
> clone. Most CI systems shallow-clone by default, which will cause
> missing commits or tags. Configure your CI checkout for full depth
> when using `--local-path`. Platform-specific instructions are in
> each section below.

## GitHub Actions, Gitea Actions & Forgejo Actions

A single action works for GitHub Actions, Gitea Actions, and Forgejo
Actions workflows. See the
[action README](https://github.com/robgonnella/releasaurus/tree/main/action)
for inputs, usage examples, and fetch depth configuration for
`--local-path`.

## GitLab CI

Use the Releasaurus Docker image directly in your `.gitlab-ci.yml`.
You may provide an authentication token either by specifying a CI/CD
variable named `GITLAB_TOKEN`, or by directly passing the `--token`
option with a reference to your defined variable, e.g.
`--token $RELEASE_TOKEN`.

**Required Scopes**:

- `api` (full API access)
- `write_repository` (repository write access)

Run both commands in a **single job** so they execute sequentially:
`release` first (it tags any merged release PR), then `release-pr` (it
opens or updates the next one). This matches the order used by the
GitHub, Gitea, and Forgejo action. Defining them as two separate jobs
with the same `rules:` lets GitLab schedule them in the same stage
concurrently, which races: `release-pr` may observe a merged but
not-yet-tagged release PR and abort with
`must finish previous release first`.

### Example

```yaml
releasaurus:
  image:
    name: rgonnella/releasaurus:vX.X.X
    entrypoint: [""]
  script:
    # Assumes a CI/CD variable named $GITLAB_TOKEN for authentication.
    # Alternatively, pass `--token $RELEASE_TOKEN` to each command.
    #
    # Run `release` BEFORE `release-pr`: `release` tags any merged
    # release PR, then `release-pr` opens/updates the next one. The
    # reverse order (or two parallel jobs) lets `release-pr` see a
    # merged-but-untagged release PR and abort with
    # "must finish previous release first".
    - releasaurus release --forge gitlab --repo $CI_PROJECT_URL
    - releasaurus release-pr --forge gitlab --repo $CI_PROJECT_URL
  rules:
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
```

### Using `--local-path`

When using `--local-path`, Releasaurus reads commit history and tags
from the local clone and requires a full checkout. Configure
`GIT_DEPTH: 0` to ensure a full clone when the runner starts fresh:

```yaml
variables:
  GIT_DEPTH: 0
```

If the runner reuses an existing workspace from a prior job
(i.e. `GIT_STRATEGY: fetch`), `GIT_DEPTH` has no effect on the
already-shallow repository. Unshallow explicitly in `before_script`:

```yaml
before_script:
  - git fetch --unshallow || true # no-op if already full-depth
```

Using both together is safe and covers all runner states.

## Azure Pipelines (EXPERIMENTAL)

Azure DevOps support is experimental. No first-party Azure Pipelines
task is provided — use the Releasaurus Docker image directly in your
pipeline. Note that the `release` command only pushes the git tag
(the changelog commit lands when the release PR is merged); Azure
DevOps Git has no native release object, so no release notes page is
created.

Provide a PAT via the `AZURE_DEVOPS_TOKEN` pipeline secret variable.
The PAT needs `Code: Read & Write` and `Pull Request Threads: Read &
Write` scopes.

The release branch (typically `releasaurus-release-*`) must have
**Allow rewriting history**
enabled for the build service identity — releasaurus performs
a non-fast-forward reset to the base branch when updating an existing
release PR. See the [Azure DevOps known
limitation](./commands.md#azure-devops-release-branch-requires-allow-rewriting-history)
for the exact setting.

Run `release` first (it tags any merged release PR), then `release-pr` (it
opens or updates the next one). This matches the order used by the
GitHub, Gitea, and Forgejo action. Running `release-pr` first may observe a
merged but not-yet-tagged release PR and abort with
`must finish previous release first`.

```yaml
trigger:
  branches:
    include:
      - main

pool:
  vmImage: ubuntu-latest

container: rgonnella/releasaurus:vX.X.X

steps:
  - checkout: self
    fetchDepth: 0 # required if you also pass --local-path

  - script: |
      releasaurus release \
        --forge azure-devops \
        --repo "$(Build.Repository.Uri)"
    env:
      AZURE_DEVOPS_TOKEN: $(AZURE_DEVOPS_TOKEN)

  - script: |
      releasaurus release-pr \
        --forge azure-devops \
        --repo "$(Build.Repository.Uri)"
    env:
      AZURE_DEVOPS_TOKEN: $(AZURE_DEVOPS_TOKEN)
```
