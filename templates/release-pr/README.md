# 🦕 Releasaurus Release PR Component

Runs `releasaurus release-pr` to automate the creation and management of release merge requests. This component analyzes commits, determines version bumps, updates version files, and generates changelogs.

## Usage

```yaml
include:
  - component: gitlab.com/rgon/releasaurus/release-pr@vX.X.X
    inputs:
      token: $GITLAB_TOKEN
```

## Inputs

| Name             | Description                                            | Default                      | Required |
| ---------------- | ------------------------------------------------------ | ---------------------------- | -------- |
| `repo`           | The GitLab project URL to affect                       | `$CI_PROJECT_URL`            | No       |
| `token`          | GitLab token with permissions to create MRs and labels |                              | Yes      |
| `debug`          | Enable debug logs                                      | `""`                         | No       |
| `git_user_name`  | Git user name for commits                              | `ReleasaurusCI`              | No       |
| `git_user_email` | Git user email for commits                             | `releasaurus-ci@noreply.com` | No       |
| `job_name`       | Customize the generated job name                       | `release-pr`                 | No       |

## What This Component Does

- Analyzes commits since the last release
- Determines the appropriate version bump (patch, minor, major)
- Updates version files across the project
- Generates a changelog from commit history
- Creates or updates a release merge request
- Supports prerelease versions (alpha, beta, rc, etc.)
- Runs in the `deploy` stage
- Uses resource group `releasaurus` to prevent concurrent executions
