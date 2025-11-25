# 🦕 Releasaurus Release Component

Runs `releasaurus release` to automate Git tag creation and GitLab release publication. This component should run after a release merge request has been merged.

## Usage

```yaml
include:
  - component: $CI_SERVER_FQDN/rgon/releasaurus/release@vX.X.X
    inputs:
      token: $GITLAB_TOKEN
```

## Inputs

| Name       | Description                                               | Default           | Required |
| ---------- | --------------------------------------------------------- | ----------------- | -------- |
| `repo`     | The GitLab project URL to affect                          | `$CI_PROJECT_URL` | No       |
| `token`    | GitLab token with permissions to create tags and releases |                   | Yes      |
| `debug`    | Enable debug logs                                         | `""`              | No       |
| `job_name` | Customize the generated job name                          | `release`         | No       |

## What This Component Does

- Validates that the current commit is a release commit
- Creates a Git tag for the new version
- Pushes the tag to the repository
- Creates a GitLab release with generated changelog
- Runs in the `.post` stage
- Uses resource group `releasaurus` to prevent concurrent executions
