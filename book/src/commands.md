# Commands

Releasaurus provides commands for release automation and inspection to create a
safe, reviewable release process. The goal is to help ensure that all changes
are reviewed before publication while automating the tedious aspects of version
management and changelog generation.

**Important**: Releasaurus operates entirely through forge platform APIs
without requiring local repository clones. You can run these commands from
any location with network access to your forge platform.

## Command Overview

### `release-pr`

**Purpose**: Analyze commits, update versions, generate changelog, and create
a pull request

This command does the heavy lifting of release preparation:

- Analyzes commits since the last release
- Determines the appropriate version bump (patch, minor, major)
- Updates version files across your project
- Generates a changelog from commit history
- Creates a pull request with all changes
- Supports prerelease versions (alpha, beta, rc, etc.)
- Supports dry-run mode for testing

### `release`

**Purpose**: Create tags and publish the release after PR approval

This command finalizes the release:

- Validates that you're on a release commit
- Creates a Git tag for the new version
- Pushes the tag to the remote repository
- Creates a release on your forge platform
- Supports prerelease versions (alpha, beta, rc, etc.)
- Supports dry-run mode for testing

### `start-next`

**Purpose**: Prepare for the next development cycle by bumping patch versions

This command helps maintain a continuous development workflow by automatically
incrementing patch versions immediately after a release:

- Bumps patch version in manifest files for each package
- Creates "chore" commits directly on the base branch
- Does NOT create pull requests or tags
- Skips packages that haven't been tagged yet
- Supports filtering to specific packages with `--packages` flag
- Ensures version numbers are always ahead of the last release
- Supports dry-run mode for testing

**Usage:**

```bash
# Start next release cycle for all packages
releasaurus start-next \
  --forge github \
  --repo "https://github.com/owner/repo"

# Target specific packages only
releasaurus start-next \
  --forge github \
  --repo "https://github.com/owner/repo" \
  --packages pkg-a,pkg-b

# With custom base branch
releasaurus start-next \
  --forge github \
  --repo "https://github.com/owner/repo" \
  --base-branch develop
```

**When to use:**

- After merging a release PR and publishing a release
- To immediately bump versions for the next development cycle
- To keep manifest versions ahead of released versions

**How it works:**

1. Identifies all packages that have been previously tagged
2. Analyzes each package's current version from its latest tag
3. Bumps the patch version (e.g., `1.2.3` → `1.2.4`)
4. Updates manifest files (package.json, Cargo.toml, etc.)
5. Creates a chore commit directly on the base branch

**Note:** This command commits directly to your base branch without creating
a pull request. Make sure you have the appropriate permissions and that your
branch protection rules allow this operation.

### `get`

**Purpose**: Query release information without making changes

This command provides release data for inspection, debugging, and custom
automation:

- View projected next releases or retrieve existing release notes
- Useful for debugging configuration and troubleshooting version detection
- Generate custom notification scripts for pre/post-release workflows
- Supports writing output to files for processing

**Note**: The `show` command is maintained as an alias for backwards
compatibility.

**Sub-commands:**

#### `get next-release`

Returns projected next release information as JSON:

```bash
# Get all projected releases
releasaurus get next-release \
  --forge github \
  --repo "https://github.com/owner/repo"

# Filter to specific package
releasaurus get next-release \
  --package my-pkg \
  --forge github \
  --repo "https://github.com/owner/repo"

# Write to file
releasaurus get next-release \
  --out-file releases.json \
  --forge github \
  --repo "https://github.com/owner/repo"

# Test locally
releasaurus get next-release --forge local --repo "."
```

**Output:** JSON array of releasable packages with version, commits, and
notes.

#### `get current-release`

Returns information about the most recent release for each package:

```bash
# Get all current releases
releasaurus get current-release \
  --forge github \
  --repo "https://github.com/owner/repo"

# Filter to specific package
releasaurus get current-release \
  --package my-pkg \
  --forge github \
  --repo "https://github.com/owner/repo"

# Write to file
releasaurus get current-release \
  --out-file current.json \
  --forge github \
  --repo "https://github.com/owner/repo"
```

**Output:** JSON array of release objects. Packages without releases are
omitted.

**Use case:** Check what versions are currently deployed or compare
current releases across packages in a monorepo.

#### `get release`

Retrieves release data for an existing tag, including the tag name, commit SHA, and release notes:

```bash
# Display release notes
releasaurus get release \
  --tag v1.0.0 \
  --forge github \
  --repo "https://github.com/owner/repo"

# Save to file
releasaurus get release \
  --tag v1.0.0 \
  --out-file release.json \
  --forge github \
  --repo "https://github.com/owner/repo"
```

**Output:** JSON object containing release data with fields:

- `tag` - The release tag name
- `sha` - The commit SHA the tag points to
- `notes` - The release notes content

#### `get recompiled-notes`

Converts release JSON from `get next-release` back into formatted notes
using your configured Tera template. This enables custom transformations like
replacing author names with Slack user IDs before generating final release
notes.

**Note**: The `show notes` command is maintained as an alias for backwards
compatibility.

```bash
# Convert release JSON to notes
releasaurus get recompiled-notes \
  --file releases.json \
  --forge github \
  --repo "https://github.com/owner/repo"

# Save to file
releasaurus get recompiled-notes \
  --file releases.json \
  --out-file notes.json \
  --forge github \
  --repo "https://github.com/owner/repo"
```

**Output:** JSON array of objects with `name` (package name) and `notes`
(rendered markdown) fields.

**Workflow example:**

```bash
# 1. Generate release data
releasaurus get next-release --out-file releases.json \
  --forge github --repo "https://github.com/owner/repo"

# 2. Transform data (e.g., replace author names with Slack IDs)
python transform_authors.py releases.json

# 3. Regenerate notes with transformations
releasaurus get recompiled-notes --file releases.json \
  --forge github --repo "https://github.com/owner/repo"
```

**Custom Notifications:** Use these commands to build notification scripts
that announce upcoming releases (pre-release) or published releases
(post-release) to Slack, Discord, email, or other channels.

See [Environment Variables](./environment-variables.md) for authentication
setup.

## Basic Usage Pattern

The typical Releasaurus workflow follows this pattern (can be run from any
directory):

```bash
# Step 1: Create release preparation PR (run from anywhere)
releasaurus release-pr \
  --forge github \
  --repo "https://github.com/owner/repo"

# Step 2: Review and merge the PR (done via web interface)

# Step 3: Publish the release (run from anywhere)
releasaurus release \
  --forge github \
  --repo "https://github.com/owner/repo"

# Step 4 (Optional): Start next development cycle
releasaurus start-next \
  --forge github \
  --repo "https://github.com/owner/repo"
```

**Note**: These commands work by accessing your repository through the forge
API, analyzing commits and files, creating branches with updates, and
managing pull requests—all without requiring a local clone.

## Global Options

All commands support these global options:

### Platform Selection

Choose your Git forge platform using the `--forge` flag with a repository
URL:

```bash
# GitHub
--forge github --repo "https://github.com/owner/repository"

# GitLab
--forge gitlab --repo "https://gitlab.com/group/project"

# Gitea (or Forgejo)
--forge gitea --repo "https://git.example.com/owner/repo"

# Local repository (for testing)
--forge local --repo "."
```

**Available forge types:**

- `github` - For github.com and GitHub Enterprise
- `gitlab` - For gitlab.com and self-hosted GitLab instances
- `gitea` - For gitea.com and self-hosted Gitea instances
- `local` - For testing against local repositories

### Dry Run Mode

Test your release workflow without making any actual changes to your
repository. Dry-run mode performs all analysis and validation steps while
logging what actions would be taken, but prevents any modifications to your
forge platform.

**Note:** Dry-run mode automatically enables debug logging for maximum
visibility into what would happen.

**What dry-run mode does:**

- ✅ Analyzes commit history since the last release
- ✅ Determines version bumps based on conventional commits
- ✅ Generates changelog content
- ✅ Validates configuration and file formats
- ✅ Logs detailed information about what would be created/modified

**What dry-run mode prevents:**

- ❌ Creating or updating branches
- ❌ Creating or updating pull requests
- ❌ Creating Git tags
- ❌ Publishing releases
- ❌ Modifying repository labels

**Usage:**

```bash
# Via command line flag
releasaurus release-pr \
  --dry-run \
  --forge github \
  --repo "https://github.com/owner/repo"

releasaurus release \
  --dry-run \
  --forge github \
  --repo "https://github.com/owner/repo"

# Via environment variable
export RELEASAURUS_DRY_RUN=true
releasaurus release-pr \
  --forge github \
  --repo "https://github.com/owner/repo"

releasaurus release \
  --forge github \
  --repo "https://github.com/owner/repo"
```

**Output:** Dry-run mode produces detailed debug logs prefixed with
`dry_run:` that show exactly what operations would be performed, including
PR titles, version numbers, file changes, and release notes. Debug mode is
automatically enabled to provide maximum visibility.

### Local Repository Mode

Test your release workflow against a local repository without making any
changes to remote forge platforms. Local repository mode is ideal for testing
configuration changes before committing them.

**Use case:** Validate your `releasaurus.toml` configuration, version
detection, and changelog generation against your local working directory
before pushing changes to your remote forge.

**What local repository mode does:**

- ✅ Reads configuration and files from your local repository
- ✅ Analyzes local commit history and tags
- ✅ Determines version bumps based on local commits
- ✅ Generates changelog content from local history
- ✅ Validates configuration and file formats
- ✅ Logs what would be created (PRs, tags, releases)

**What local repository mode prevents:**

- ❌ Creating or updating remote branches
- ❌ Creating or updating pull requests on forge platforms
- ❌ Creating or pushing Git tags to remote
- ❌ Publishing releases to forge platforms
- ❌ Any modifications to remote repositories

**Usage:**

```bash
# Test from current directory
releasaurus release-pr --forge local --repo "."

# Test from specific path
releasaurus release-pr --forge local --repo "/path/to/your/repo"

# Works with release command too
releasaurus release --forge local --repo "."
```

**Typical workflow:**

```bash
# 1. Make changes to releasaurus.toml
vim releasaurus.toml

# 2. Test locally to verify configuration
releasaurus release-pr --forge local --repo "."

# 3. Review the logs to ensure everything looks correct

# 4. Commit your config changes
git add releasaurus.toml
git commit -m "chore: update release configuration"
git push

# 5. Run against remote forge
releasaurus release-pr \
  --forge github \
  --repo "https://github.com/owner/repo"
```

**Note:** Local repository mode operates on your working directory's Git
repository and does not require forge authentication tokens. See the
[Troubleshooting](./troubleshooting.md) guide for help diagnosing
configuration issues.

### Configuration Overrides

Override configuration properties from the command line without modifying your
`releasaurus.toml` file. This is useful for testing different settings, creating
one-off releases with custom configurations, or using different values in
CI/CD pipelines.

**Available overrides:**

- `--base-branch <branch>` - Override the base branch for the release
- `--tag-prefix <prefix>` - Set or override global tag prefix. Applied to
  all packages unless overridden per-package
- `--prerelease-suffix <suffix>` - Set or override global prerelease
  suffix. Applied to all packages unless overridden per-package
- `--prerelease-strategy <strategy>` - Set global prerelease strategy
  (`versioned` or `static`). Applied to all packages unless overridden
  per-package
- `--skip-sha <sha>` - Skip specific commits by SHA prefix (7+ characters).
  Can be used multiple times to skip multiple commits
- `--reword <sha>=<message>` - Rewrite a commit message. Use format
  `sha=new message`. Can be used multiple times for multiple commits
- `--set-package <package_name>.<property>=<value>` - Override
  package-specific properties. This takes precedence over all global overrides
  and config. Not all properties are overridable. If you try to set an
  unsupported property an error will be displayed with available valid values.
  Currently supported:
  - `--set-package <pkg_name>.tag_prefix=<prefix>`
  - `--set-package <pkg_name>.prerelease.suffix=<suffix>`
  - `--set-package <pkg_name>.prerelease.strategy=<strategy>`

**Override precedence (highest to lowest):**

1. Package-specific CLI overrides (`--set-package`)
2. Global CLI overrides (`--base-branch`, `--tag-prefix`, `--prerelease-*`)
3. Package configuration in `releasaurus.toml`
4. Global configuration in `releasaurus.toml`
5. Default values

**Usage examples:**

```bash
# Override base branch
releasaurus release-pr \
  --base-branch develop \
  --forge github \
  --repo "https://github.com/owner/repo"

# Override global tag prefix for all packages
releasaurus release-pr \
  --tag-prefix release-v \
  --forge github \
  --repo "https://github.com/owner/repo"

# Override global prerelease configuration
releasaurus release-pr \
  --prerelease-suffix beta \
  --prerelease-strategy versioned \
  --forge github \
  --repo "https://github.com/owner/repo"

# Override package-specific prerelease suffix
releasaurus release-pr \
  --set-package my-pkg.prerelease.suffix=rc \
  --forge github \
  --repo "https://github.com/owner/repo"

# Override package-specific prerelease strategy
releasaurus release-pr \
  --set-package my-pkg.prerelease.strategy=static \
  --forge github \
  --repo "https://github.com/owner/repo"

# Override package tag prefix (useful for monorepos or custom tagging)
releasaurus release-pr \
  --set-package my-pkg.tag_prefix=custom-v \
  --forge github \
  --repo "https://github.com/owner/repo"

# Combine multiple overrides
releasaurus release-pr \
  --base-branch staging \
  --prerelease-suffix alpha \ # applies to all packages
  --set-package frontend.prerelease.suffix=beta \ # applies only to frontend
  --forge github \
  --repo "https://github.com/owner/repo"

# Skip specific commits and reword others
releasaurus release-pr \
  --skip-sha abc123d \
  --reword "def456e=feat: improved authentication" \
  --forge github \
  --repo "https://github.com/owner/repo"
```

**Use cases:**

- Test prerelease configurations without modifying your config file
- Create emergency releases from different branches
- Use different settings across environments / branches (dev/staging/prod)
- Override per-package settings for specific releases

See the [Configuration](./configuration.md) guide for details on
prerelease configuration.

### Authentication

Provide access tokens for API authentication:

```bash
# Via command line
--token "your_token_here"

# Via environment variables (recommended)
export GITHUB_TOKEN="ghp_xxxxxxxxxxxxxxxxxxxx"
export GITLAB_TOKEN="glpat_xxxxxxxxxxxxxxxxxxxx"
export GITEA_TOKEN="xxxxxxxxxxxxxxxxxx"
```

Releasaurus automatically selects the appropriate environment variable based
on the `--forge` type:

- `--forge github` uses `GITHUB_TOKEN`
- `--forge gitlab` uses `GITLAB_TOKEN`
- `--forge gitea` uses `GITEA_TOKEN`
- `--forge local` requires no authentication

### Debug Logging

Enable detailed logging for troubleshooting:

```bash
# Via command line flag
--debug

# Via environment variable
export RELEASAURUS_DEBUG=true
```

This provides verbose output including:

- Project detection logic
- File modification details
- API request/response information
- Git operations and status

**Note:** Debug mode is automatically enabled when using `--dry-run` or
`RELEASAURUS_DRY_RUN=true`.

See the [Environment Variables](./environment-variables.md#releasaurus_debug)
guide for more details on `RELEASAURUS_DEBUG`.

### Prerelease Versions

Prerelease versions can be configured in your `releasaurus.toml` file:

```toml
# Global prerelease for all packages
[prerelease]
suffix = "alpha"
strategy = "versioned"

[[package]]
path = "."
release_type = "node"
```

Or configure per-package:

```toml
[[package]]
path = "./packages/stable"
release_type = "rust"
# No prerelease - stable releases

[[package]]
path = "./packages/experimental"
release_type = "rust"
prerelease = { suffix = "beta", strategy = "versioned" }  # Beta releases
```

The prerelease strategy can be one of "versioned" or "static". A "versioned"
strategy will result in a trailing version as part of the prerelease, e.g.
`-alpha.0, -alpha.1 ...`. A "static" strategy will only add a static
prerelease suffix, e.g. `-SNAPSHOT`.

**Prerelease Behavior:**

- **Starting**: `v1.0.0` → `v1.1.0-alpha.1` (with feature commit and
  `suffix = "alpha", strategy = "versioned"`)
- **Continuing**: `v1.1.0-alpha.1` → `v1.1.0-alpha.2` (same identifier in
  config)
- **Switching**: `v1.0.0-alpha.3` → `v1.1.0-beta.1` (change
  `suffix = "beta"` in config)
- **Graduating**: `v1.0.0-alpha.5` → `v1.0.0` (remove `prerelease` from
  config)

To change prerelease identifiers or graduate to stable, update your
configuration file and create a new release PR.

See the [Configuration](./configuration.md) guide for complete prerelease
configuration details.

## Platform-Specific Examples

### GitHub

```bash
# With explicit token
releasaurus release-pr \
  --forge github \
  --repo "https://github.com/myorg/myproject" \
  --token "ghp_xxxxxxxxxxxxxxxxxxxx"

# With environment variable
export GITHUB_TOKEN="ghp_xxxxxxxxxxxxxxxxxxxx"
releasaurus release-pr \
  --forge github \
  --repo "https://github.com/myorg/myproject"
```

### GitLab

```bash
# GitLab.com
releasaurus release-pr \
  --forge gitlab \
  --repo "https://gitlab.com/mygroup/myproject" \
  --token "glpat_xxxxxxxxxxxxxxxxxxxx"

# Self-hosted GitLab
releasaurus release-pr \
  --forge gitlab \
  --repo "https://gitlab.company.com/team/project" \
  --token "glpat_xxxxxxxxxxxxxxxxxxxx"
```

### Gitea

```bash
# Self-hosted Gitea
releasaurus release-pr \
  --forge gitea \
  --repo "https://git.company.com/team/project" \
  --token "xxxxxxxxxxxxxxxxxx"

# Works with Forgejo too
releasaurus release-pr \
  --forge gitea \
  --repo "https://forgejo.example.com/org/repo" \
  --token "xxxxxxxxxxxxxxxxxx"
```

## Environment Variables

For security and convenience, use environment variables instead of
command-line tokens. When environment variables are set, you can omit the
`--token` flag.

See the [Environment Variables](./environment-variables.md) guide for
complete details on all available environment variables, authentication token
setup, and configuration options.

## Help and Documentation

Get help for any command:

```bash
# General help
releasaurus --help

# Command-specific help
releasaurus <cmd> --help

# Version information
releasaurus --version
```

## Next Steps

For integration and automation:

- **[Troubleshooting](./troubleshooting.md)** - Common issues and solutions
