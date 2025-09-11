# Gitea Actions Integration

Releasaurus provides seamless integration with Gitea Actions through the
official [releasaurus-gitea-action]. This action automates your release
workflow directly in your Gitea repository, eliminating the need to run
Releasaurus commands manually. See action
[documentation][releasaurus-gitea-action] for all available options.

## Basic Setup

### Step 1: Create the Workflow File

Create a `.gitea/workflows/release.yml` file in your repository:

```yaml
name: Release
on:
  workflow_dispatch:
  push:
    branches: [main]

jobs:
  release:
    runs-on: ubuntu-latest
    permissions:
      contents: write
      pull-requests: write
      issues: write
    steps:
      - name: Checkout
        uses: actions/checkout@v5
      - name: Run Releasaurus
        uses: https://gitea.com/rgon/releasaurus-gitea-action@v1
```

### Step 2: Configure Repository Permissions

Ensure your Gitea repository has the correct permissions:

1. Go to your repository **Settings → Actions**
2. Enable **Actions** if not already enabled
3. Under **Secrets and Variables**, add any custom tokens if needed
4. Ensure the default `GITEA_TOKEN` has sufficient permissions

### Step 3: Token Permissions (If Using Custom Token)

If you need to use a custom Gitea token instead of the default, ensure it has:

- **Contents**: `write` - To create tags and releases
- **Pull requests**: `write` - To create release pull requests
- **Issues**: `write` - To create and manage labels

That's it! Your repository now has fully automated releases.

[releasaurus-gitea-action]: https://gitea.com/rgon/releasaurus-gitea-action
