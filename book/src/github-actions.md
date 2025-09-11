# GitHub Actions Integration

Releasaurus provides seamless integration with GitHub Actions through the
official [robgonnella/releasaurus-action]. This action automates your release
workflow directly in your GitHub repository, eliminating the need to run
Releasaurus commands manually. See action
[documentation][robgonnella/releasaurus-action]] for all available options.

## Basic Setup

### Step 1: Create the Workflow File

Create a `.github/workflows/release.yml` file in your repository:

```yaml
name: Release
on:
  workflow_dispatch:
  push:
    branches: [main]

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - name: Release
        uses: robgonnella/releasaurus-action@v1
        with:
          github-token: ${{ secrets.GITHUB_TOKEN }}
```

### Step 2: Configure Repository Permissions

Ensure your repository has the correct permissions:

1. Go to your repository **Settings → Actions → General**
2. Under **Workflow permissions**, select **Read and write permissions**
3. Check **Allow GitHub Actions to create and approve pull requests**

That's it! Your repository now has fully automated releases.

[robgonnella/releasaurus-action]: https://github.com/robgonnella/releasaurus-action
