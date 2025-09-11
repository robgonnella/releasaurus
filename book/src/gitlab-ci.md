# GitLab CI/CD Integration

Releasaurus provides seamless integration with GitLab CI/CD through the
official [releasaurus-component]. This component automates your release
workflow directly in your GitLab project, eliminating the need to run
Releasaurus commands manually. See component
[documentation][releasaurus-component] for all available options.

## Basic Setup

### Step 1: Create the Pipeline Configuration

Create a `.gitlab-ci.yml` file in your repository or add the following to your
existing pipeline:

```yaml
include:
  - component: gitlab.com/rgon/releasaurus-component/releasaurus@~latest
    inputs:
      token: $GITLAB_TOKEN
```

### Step 2: Configure Project Permissions

Ensure your GitLab project has the correct permissions:

1. Go to **Settings → CI/CD → Variables**
2. Add a project variable `GITLAB_TOKEN` with a Personal Access Token that has:
   - `api` scope
   - `write_repository` scope

### Step 3: Project Access Token (Recommended)

For better security, use a Project Access Token instead of a Personal Access
Token:

1. Go to **Settings → Access Tokens**
2. Create a new token with:
   - **Token name**: `releasaurus-ci`
   - **Role**: `Maintainer`
   - **Scopes**: `api`, `write_repository`
3. Copy the generated token
4. Go to **Settings → CI/CD → Variables**
5. Add variable `GITLAB_TOKEN` with the project access token as the value
6. Mark it as **Protected** and **Masked**
