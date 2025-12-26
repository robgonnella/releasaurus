# Configuration

Releasaurus works out-of-the-box with zero configuration, but provides
extensive customization through an optional `releasaurus.toml` file.

## Do You Need Configuration?

### You DON'T need configuration if:

- You only need changelog generation and tagging (no version file
  updates)
- You're happy with the default changelog format
- You're happy with the default tag prefix "v" (e.g., `v1.0.0`)

### You DO need configuration if:

- You want version file updates (requires specifying `release_type`)
- You want custom changelog templates or formatting
- You have multiple packages in one repository (monorepo)
- You want custom prefixed tags (e.g., `cli-v1.0.0`)

## Quick Start Examples

### Single Package with Version Updates

The most common setup:

```toml
# releasaurus.toml
[[package]]
path = "."
release_type = "node"  # or rust, python, java, php, ruby
```

### Multi-Package (Monorepo)

Multiple independently-versioned packages:

```toml
[[package]]
path = "./frontend"
release_type = "node"
tag_prefix = "frontend-v"

[[package]]
path = "./backend"
release_type = "rust"
tag_prefix = "backend-v"
```

See [Monorepo Configuration](./configuration-monorepo.md) for complete
details.

### Prerelease Versions

Create alpha/beta releases:

```toml
[prerelease]
suffix = "alpha"
strategy = "versioned"  # or "static"

[[package]]
path = "."
release_type = "node"
```

This creates versions like `v1.0.0-alpha.1`, `v1.0.0-alpha.2`, etc.

See [Prerelease Configuration](./configuration-prerelease.md) for
complete details.

### Custom Changelog

Filter commits and customize formatting:

```toml
[changelog]
skip_ci = true
skip_chore = true
skip_miscellaneous = true

[[package]]
path = "."
release_type = "rust"
```

See [Changelog Configuration](./configuration-changelog.md) for template
customization and complete options.

## Configuration Topics

### Core Settings

- **[Prerelease Versions](./configuration-prerelease.md)** - Alpha,
  beta, RC releases with versioned or static strategies
- **[Changelog Customization](./configuration-changelog.md)** - Filter
  commits, customize templates, format output
- **[Monorepo Setup](./configuration-monorepo.md)** - Multiple packages,
  separate PRs, independent versioning

### Reference

- **[Configuration Reference](./configuration-reference.md)** - Complete
  list of all configuration options with descriptions

## Configuration File Location

Place `releasaurus.toml` in your project's root directory:

```
my-project/
├── releasaurus.toml    # ← Configuration file
├── src/
└── README.md
```

## Command-Line Overrides

Many configuration options can be overridden from the command line
without modifying your config file. This is useful for testing different
settings or using different values in CI/CD pipelines.

See [Configuration Overrides](./commands.md#configuration-overrides) in
the Commands guide for details.

## Testing Your Configuration

Test your configuration locally without making remote changes:

```bash
# Test against your local repository
releasaurus release-pr --forge local --repo "."

# Review the output to verify settings
# Then run against your remote forge when ready
```

See [Local Repository Mode](./commands.md#local-repository-mode) for
complete details.

## Next Steps

- **Getting started?** See [Quick Start](./quick-start.md) for a 2-minute
  tutorial
- **Need help?** Check [Troubleshooting](./troubleshooting.md) for
  common issues
- **CI/CD?** See [CI/CD Integration](./ci-cd-integration.md) for
  automation
