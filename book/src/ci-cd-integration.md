# CI/CD Integration

Releasaurus provides official integrations for GitHub Actions and Gitea
Actions. For GitLab CI, use the Docker image directly.

> **Note on fetch depth:** When using `--local-path` (hybrid mode),
> Releasaurus reads commit history and tags directly from the local
> clone. Most CI systems shallow-clone by default, which will cause
> missing commits or tags. Configure your CI checkout for full depth
> when using `--local-path`. Platform-specific instructions are in
> each section below.

## GitHub Actions & Gitea Actions

A single action works for both GitHub Actions and Gitea Actions
workflows. See the
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

### Example

```yaml
publish-release:
  image:
    name: rgonnella/releasaurus:vX.X.X
    entrypoint: [""]
  script:
    # Assumes use of $GITLAB_TOKEN var for token authentication
    - releasaurus release --forge gitlab --repo $CI_PROJECT_URL
  rules:
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH

release-pr:
  image:
    name: rgonnella/releasaurus:vX.X.X
    entrypoint: [""]
  script:
    # Uses custom var for token authentication
    - releasaurus release-pr --forge gitlab \
        --repo $CI_PROJECT_URL --token $RELEASE_TOKEN
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
  - git fetch --unshallow || true  # no-op if already full-depth
```

Using both together is safe and covers all runner states.
