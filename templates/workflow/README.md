# 🦕 Releasaurus GitLab CI Component

Automates the complete release workflow by including both `release` and `release-pr` components. This composite component manages release merge requests and publishes releases for GitLab projects.

## Usage

```yaml
include:
  - component: $CI_SERVER_FQDN/rgon/releasaurus/workflow@vX.X.X
    inputs:
      token: $GITLAB_TOKEN
```

## Inputs

| Name                | Description                                                             | Default           | Required |
| ------------------- | ----------------------------------------------------------------------- | ----------------- | -------- |
| `repo`              | The GitLab project URL to affect                                        | `$CI_PROJECT_URL` | No       |
| `token`             | GitLab token with permissions to create MRs, tags, labels, and releases |                   | Yes      |
| `debug`             | Enable debug logs                                                       | `""`              | No       |
| `pr_job_stage`      | Stage in which to run release-pr job                                    | `.pre`            | No       |
| `release_job_stage` | Stage in which to run release job                                       | `.pre`            | No       |

## What This Component Does

This composite component includes both:

1. [`release-pr`](../release-pr) - Creates or updates release merge requests
2. [`release`](../release) - Publishes releases when on a release commit

For more granular control, use the individual components directly.
