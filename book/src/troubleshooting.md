# Troubleshooting

This guide helps you diagnose and resolve common issues when using
Releasaurus. If you encounter problems not covered here, please check the
[GitHub issues](https://github.com/robgonnella/releasaurus/issues) or create
a new one.

## Common Issues

### Configuring First Release Search Depth

The `first_release_search_depth` setting controls how many commits to analyze
when **no matching tags are found** for a package's configured prefix. The
default is 400 commits.

**When this setting applies:**

This setting **only** affects the first release when Releasaurus cannot find
any existing tags matching your package's `tag_prefix`. Once a matching tag
exists, Releasaurus will analyze all commits back to that tag, regardless of
this setting.

**What this setting does:**

1. **Limits first release analysis** - Prevents analyzing thousands of
   commits when creating your very first release
2. **Performance optimization** - Improves speed for repositories with
   extensive commit history but no existing tags

**When to adjust this setting:**

#### Increase search depth

If you want more comprehensive changelogs for your first release:

```toml
# Analyze more commits for first release
first_release_search_depth = 1000

[[package]]
path = "."
```

**Use case**: You want your first release changelog to include more commit
history.

#### Decrease search depth

For faster first release creation:

```toml
# Search fewer commits for better performance
first_release_search_depth = 100

[[package]]
path = "."
```

**Use case**: You want faster first release creation and don't need extensive
commit history in the initial changelog.

**Note**: This setting does NOT affect tag discovery. Releasaurus searches
all tags regardless of this setting. This only limits how many commits are
analyzed when no matching tags exist.

#### Issue: Releasaurus doesn't find existing tags

**Possible causes:**

1. **Tag prefix mismatch** - Your configured `tag_prefix` doesn't match
   existing tags
2. **Tags don't follow semver** - Existing tags don't use semantic versioning
   format

**Solutions:**

**Match your tag prefix:**

```toml
[[package]]
path = "."
tag_prefix = "v"  # Must match your existing tags like "v1.0.0"
```

Common prefix patterns:

- `tag_prefix = "v"` for tags like `v1.0.0`, `v2.1.0`
- `tag_prefix = "api-v"` for tags like `api-v1.0.0`
- `tag_prefix = ""` for tags like `1.0.0` (no prefix)

### Authentication Issues

#### Issue: "Authentication failed" or "401 Unauthorized"

**Symptoms**:

- Cannot access repository
- API calls fail with authentication errors
- Permission denied messages

**Solutions**:

1. **Verify token is set**:

   ```bash
   # Check environment variable
   echo $GITHUB_TOKEN
   echo $GITLAB_TOKEN
   echo $GITEA_TOKEN

   # Or provide via command line
   releasaurus release-pr \
     --forge github \
     --repo "https://github.com/owner/repo" \
     --token "your_token"
   ```

2. **Check token scopes** - Ensure your token has required permissions (see
   [Environment Variables](./environment-variables.md))

3. **Token expiration** - Generate a new token if the current one has expired

#### Issue: "Repository not found" with valid repository

**Cause**: Token doesn't have access to the repository or repository URL is
incorrect.

**Solutions**:

1. **Verify repository URL**:

   ```bash
   # Correct format examples
   --forge github --repo "https://github.com/owner/repository"
   --forge gitlab --repo "https://gitlab.com/group/project"
   --forge gitea --repo "https://gitea.example.com/owner/repo"
   ```

2. **Check repository access** - Ensure your token's associated account has
   appropriate permissions.

3. **Test with local mode first**:

   ```bash
   # Clone the repository and test locally
   git clone https://github.com/owner/repo
   cd repo
   releasaurus get next-release --forge local --repo "."
   ```

## Inspecting Projected Releases

Use the `get next-release` command to inspect what Releasaurus will do
before creating a PR or making any changes:

```bash
# See all projected releases
releasaurus get next-release \
  --forge github \
  --repo "https://github.com/owner/repo"

# Inspect specific package
releasaurus get next-release \
  --package my-pkg \
  --forge github \
  --repo "https://github.com/owner/repo"

# Save output to file for detailed inspection
releasaurus get next-release \
  --out-file releases.json \
  --forge github \
  --repo "https://github.com/owner/repo"

# Test locally without authentication
releasaurus get next-release --forge local --repo "."
```

**Use this command to:**

- **Verify version calculation** - Check if the next version matches your
  expectations
- **Inspect commit analysis** - See which commits are included and how
  they're categorized
- **Validate configuration** - Ensure `releasaurus.toml` settings produce
  desired results
- **Preview release notes** - Review changelog content before creating a PR
- **Debug tag matching** - Verify tag prefix configuration matches repository
  tags
- **Test monorepo setup** - Confirm package detection and independent
  versioning

The JSON output includes all information that would be used in a release PR,
making it ideal for diagnosing issues without modifying your repository.

## Debug Mode

When troubleshooting any issue, enable debug mode for detailed information:

```bash
# Via command line flag
releasaurus release-pr \
  --debug \
  --forge github \
  --repo "https://github.com/owner/repo"

# Or via environment variable
export RELEASAURUS_DEBUG=true
releasaurus release-pr \
  --forge github \
  --repo "https://github.com/owner/repo"
```

Debug output includes:

- Commit analysis and categorization
- Version calculation logic
- File detection and modification details
- API requests and responses
- Tag matching and selection

See the [Environment Variables](./environment-variables.md#releasaurus_debug)
guide for more details on `RELEASAURUS_DEBUG`.

## Dry Run Mode

Before making changes to your repository, use dry-run mode to safely test and
diagnose issues:

```bash
# Via command line flag
releasaurus release-pr \
  --dry-run \
  --forge github \
  --repo "https://github.com/owner/repo"

# Via environment variable
export RELEASAURUS_DRY_RUN=true
releasaurus release-pr \
  --forge github \
  --repo "https://github.com/owner/repo"
```

**Note:** Dry-run mode automatically enables debug logging for maximum
visibility.

See the [Commands](./commands.md#dry-run-mode) guide for complete details.

## Local Repository Mode

Test your configuration and diagnose issues against your local repository
without requiring authentication or making remote changes:

```bash
# Test from current directory
releasaurus release-pr --forge local --repo "."

# Test from specific path
releasaurus release-pr --forge local --repo "/path/to/your/repo"
```

**Use local repository mode for:**

- **Configuration validation** - Test `releasaurus.toml` changes before
  committing
- **Version detection issues** - Verify tag prefix matching and version
  calculation
- **Changelog preview** - See what changelog would be generated from local
  commits
- **Quick diagnostics** - Test without authentication setup or network access

See the [Commands](./commands.md#local-repository-mode) guide for complete
details.

## Getting Help

If you're still experiencing issues:

1. **Check existing issues**: [GitHub Issues](https://github.com/robgonnella/releasaurus/issues)
2. **Create a new issue** with:
   - Debug output (remove sensitive information)
   - Repository type and structure
   - Command used
   - Expected vs actual behavior
3. **Include environment details**:
   - Operating system
   - Releasaurus version (`releasaurus --version`)
   - Forge platform and hosting type (e.g., GitHub.com, self-hosted GitLab)
