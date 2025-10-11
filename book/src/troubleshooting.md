# Troubleshooting

This guide helps you diagnose and resolve common issues when using Releasaurus.
If you encounter problems not covered here, please check the
[GitHub issues](https://github.com/robgonnella/releasaurus/issues) or create a
new one.

## Common Issues

### First Release History Configuration

#### Issue: "Not enough commit history found" for first release

**Symptoms**:

- Releasaurus can't determine the next version for first release
- Error messages about insufficient commit history
- Empty or incomplete changelog generation on initial release

**Cause**: The default `first_release_search_depth` (400 commits) doesn't
include enough history for your repository's first release analysis.

**Solution**: Increase the search depth in your `releasaurus.toml`:

```toml
# Increase commit search depth for first release
first_release_search_depth = 1000

[[package]]
path = "."
```

**Note**: This setting only affects the first release when no tags exist. Once
you have a release tag, subsequent releases automatically find all commits
since the last tag.

#### Issue: Slow first release analysis

**Symptoms**:

- Long wait times during first release PR creation
- Timeouts in CI/CD environments on initial release
- High API usage during analysis

**Cause**: Searching through extensive commit history for the first release.

**Solution**: Reduce the search depth in your `releasaurus.toml`:

```toml
# Reduce commit search depth for faster first release analysis
first_release_search_depth = 100

[[package]]
path = "."
```

For CI/CD environments with large repositories:

```toml
# Minimal search depth for CI/CD
first_release_search_depth = 50

[[package]]
path = "."
```

#### Issue: "Could not find any releases" despite having tags

**Symptoms**:

- Repository has existing tags/releases
- Releasaurus acts like it's the first release
- Version numbers don't follow expected sequence

**Causes**:

- Configured tag prefixes in `releasaurus.toml` don't match existing tags
- Tags exist but don't follow semantic versioning format

**Solution**: Ensure configured tag prefixes match existing tag patterns in
your `releasaurus.toml`:

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

- **Check token scopes** - Ensure your token has required permissions

#### Issue: "Repository not found" with valid repository

**Cause**: Token doesn't have access to the repository or repository URL is
incorrect.

**Solutions**:

1. **Verify repository URL**:

   ```bash
   # Correct format examples
   --github-repo "https://github.com/owner/repository"
   --gitlab-repo "https://gitlab.com/group/project"
   --gitea-repo "https://gitea.example.com/owner/repo"
   ```

2. **Check repository access** - Ensure your token's associated account has
   appropriate permissions.

## Debug Mode

When troubleshooting any issue, enable debug mode for detailed information:

```bash
releasaurus release-pr --debug \
  --github-repo "https://github.com/owner/repo"
```

## Getting Help

If you're still experiencing issues:

1. **Check existing issues**: [GitHub Issues]
2. **Create a new issue** with:
   - Debug output (remove sensitive information)
   - Repository type and structure
   - Command used
   - Expected vs actual behavior
3. **Include environment details**:
   - Operating system
   - Releasaurus version (`releasaurus --version`)

[GitHub Issues]: https://github.com/robgonnella/releasaurus/issues
