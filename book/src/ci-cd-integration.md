# CI/CD Integration

Releasaurus provides official integrations for GitHub Actions and Gitea
Actions. For GitLab CI, use the Docker image directly.

## GitHub Actions & Gitea Actions

A single action works for both GitHub Actions and Gitea Actions
workflows, exposing the `releasaurus` executable for maximum
flexibility.

**Documentation**: [action/README.md](https://github.com/robgonnella/releasaurus/tree/main/action)

### Example

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

## GitLab CI

Use the Releasaurus Docker image directly in your `.gitlab-ci.yml`:

### Example

```yaml
publish-release:
  image:
    name: rgonnella/releasaurus:vX.X.X
    entrypoint: [""]
  script:
    - releasaurus release --forge gitlab --repo $CI_PROJECT_URL
  rules:
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
  variables:
    GITLAB_TOKEN: $GITLAB_TOKEN

release-pr:
  image:
    name: rgonnella/releasaurus:vX.X.X
    entrypoint: [""]
  script:
    - releasaurus release-pr --forge gitlab --repo $CI_PROJECT_URL
  rules:
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
  variables:
    GITLAB_TOKEN: $GITLAB_TOKEN
```
