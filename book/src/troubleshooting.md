# Troubleshooting

This guide helps you diagnose and resolve common issues when using Releasaurus.
If you encounter problems not covered here, please check the [GitHub issues](https://github.com/robgonnella/releasaurus/issues) or create a new one.

## Common Issues

### Clone Depth Problems

#### Issue: "Not enough commit history found"

**Symptoms**:

- Releasaurus can't determine the next version
- Error messages about missing previous releases
- Empty or incomplete changelog generation

**Cause**: The default clone depth (250 commits) doesn't include enough history
for analysis.

**Solutions**:

```bash
# Option 1: Increase clone depth
releasaurus release-pr \
  --github-repo "https://github.com/owner/repo" \
  --clone-depth 500

# Option 2: Clone full history
releasaurus release-pr \
  --github-repo "https://github.com/owner/repo" \
  --clone-depth 0
```

#### Issue: Clone operation is too slow

**Symptoms**:

- Long wait times during repository cloning
- Timeouts in CI/CD environments
- High bandwidth usage

**Cause**: Repository has extensive history or large binary files.

**Solutions**:

```bash
# Reduce clone depth for faster operations
releasaurus release-pr \
  --github-repo "https://github.com/owner/repo" \
  --clone-depth 50

# For CI/CD environments
releasaurus release-pr \
  --github-repo "https://github.com/owner/repo" \
  --clone-depth 25
```

#### Issue: "Could not find any releases" despite having tags

**Symptoms**:

- Repository has existing tags/releases
- Releasaurus acts like it's the first release
- Version numbers don't follow expected sequence

**Causes**:

- Clone depth is too shallow to reach previous release tags.
- Configured tag prefixes don't match existing tags

**Solution**:

- Set larger clone depth to ensure last tag is included

```bash
# Ensure full history is available
releasaurus release-pr \
  --github-repo "https://github.com/owner/repo" \
  --clone-depth 0
```

- Ensure configured tag prefixes in `releasaurus.toml` match existing tag
  patterns

### Authentication Issues

#### Issue: "Authentication failed" or "401 Unauthorized"

**Symptoms**:

- Cannot access repository
- API calls fail with authentication errors
- Permission denied messages

**Solutions**:

- **Check token scopes** - Ensure your token has required permissions:

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

1. **Check existing issues**: [GitHub Issues](https://github.com/robgonnella/releasaurus/issues)
2. **Create a new issue** with:
   - Debug output (remove sensitive information)
   - Repository type and structure
   - Command used
   - Expected vs actual behavior
3. **Include environment details**:
   - Operating system
   - Releasaurus version (`releasaurus --version`)
   - Git version (`git --version`)
