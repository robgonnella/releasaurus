# Prerelease Configuration

Create alpha, beta, RC, or snapshot releases with automatic version
management.

## Overview

Prerelease versions allow you to publish test releases before final
stable versions. Releasaurus supports two strategies:

- **Versioned**: Incremental counters (e.g., `1.0.0-alpha.1`,
  `1.0.0-alpha.2`)
- **Static**: Fixed suffix (e.g., `1.0.0-SNAPSHOT`, `1.1.0-SNAPSHOT`)

## Quick Configuration

### Global Prerelease (All Packages)

```toml
[prerelease]
suffix = "alpha"
strategy = "versioned"  # or "static"

[[package]]
path = "."
release_type = "node"
```

### Per-Package Override

```toml
[prerelease]
suffix = "beta"
strategy = "versioned"

[[package]]
path = "./stable"
release_type = "rust"
# Uses global beta prerelease

[[package]]
path = "./experimental"
release_type = "rust"
prerelease = { suffix = "alpha", strategy = "versioned" }
# Override with alpha
```

## Configuration Options

### `suffix`

The identifier to append to version numbers.

**Common values:**
- `"alpha"` - Early testing phase
- `"beta"` - Feature complete, testing phase
- `"rc"` - Release candidate
- `"preview"` - Preview releases
- `"dev"` - Development/nightly builds
- `"SNAPSHOT"` - Java snapshot versioning

**Omit or set to empty string to disable prereleases.**

### `strategy`

Controls how version numbers are generated.

**Options:**
- `"versioned"` (default) - Adds incremental counter (`.1`, `.2`,
  `.3`)
- `"static"` - Uses suffix as-is without counter

## Versioned Strategy

Incremental counters track iteration within a prerelease series.

### Starting a Prerelease

```
Current:  v1.0.0
Commit:   feat: new feature
Config:   suffix = "alpha", strategy = "versioned"
Result:   v1.1.0-alpha.1
```

### Continuing with Same Identifier

```
Current:  v1.1.0-alpha.1
Commit:   fix: bug fix
Config:   suffix = "alpha", strategy = "versioned"
Result:   v1.1.0-alpha.2
```

### Switching Identifier

```
Current:  v1.0.0-alpha.3
Commit:   feat: new feature
Config:   suffix = "beta", strategy = "versioned"
Result:   v1.1.0-beta.1
```

Switching identifiers recalculates the base version and resets the
counter.

### Graduating to Stable

```
Current:  v1.0.0-alpha.5
Commit:   fix: final fix
Config:   (no prerelease section)
Result:   v1.0.0
```

Remove the `[prerelease]` section or set `suffix = ""` to graduate.

## Static Strategy

Fixed suffix without counters, common in Java ecosystems.

### Starting a Prerelease

```
Current:  v1.0.0
Commit:   fix: bug fix
Config:   suffix = "SNAPSHOT", strategy = "static"
Result:   v1.0.1-SNAPSHOT
```

### Continuing with Same Identifier

```
Current:  v1.0.1-SNAPSHOT
Commit:   feat: new feature
Config:   suffix = "SNAPSHOT", strategy = "static"
Result:   v1.1.0-SNAPSHOT
```

Version number updates based on commit type, suffix stays constant.

### Graduating to Stable

```
Current:  v1.0.1-SNAPSHOT
Commit:   fix: final fix
Config:   (no prerelease section)
Result:   v1.0.1
```

## Practical Examples

### Monorepo with Mixed Prerelease States

```toml
# Most packages stable, some experimental
separate_pull_requests = true

[[package]]
path = "./core"
release_type = "rust"
tag_prefix = "core-v"
# No prerelease - stable only

[[package]]
path = "./experimental"
release_type = "rust"
tag_prefix = "exp-v"
prerelease = { suffix = "alpha", strategy = "versioned" }

[[package]]
path = "./staging"
release_type = "node"
tag_prefix = "staging-v"
prerelease = { suffix = "beta", strategy = "versioned" }
```

### Java Project with Snapshots

```toml
[prerelease]
suffix = "SNAPSHOT"
strategy = "static"

[[package]]
path = "."
release_type = "java"
```

### Progressive Release Pipeline

```toml
# Start in alpha, progress to beta, then stable
[[package]]
path = "."
release_type = "node"

# Phase 1: Configure alpha
# prerelease = { suffix = "alpha", strategy = "versioned" }

# Phase 2: Switch to beta when ready
# prerelease = { suffix = "beta", strategy = "versioned" }

# Phase 3: Remove prerelease for stable release
```

## Command-Line Overrides

Override prerelease settings without modifying your config file:

```bash
# Test beta prereleases
releasaurus release-pr \
  --prerelease-suffix beta \
  --prerelease-strategy versioned \
  --forge github \
  --repo "https://github.com/org/repo"

# Override specific package
releasaurus release-pr \
  --set-package my-pkg.prerelease.suffix=rc \
  --set-package my-pkg.prerelease.strategy=versioned \
  --forge github \
  --repo "https://github.com/org/repo"

# Disable prerelease temporarily
releasaurus release-pr \
  --prerelease-suffix "" \
  --forge github \
  --repo "https://github.com/org/repo"
```

See [Configuration Overrides](./commands.md#configuration-overrides)
for complete details.

## Configuration Priority

Settings are applied in this order (highest to lowest):

1. Package-specific CLI overrides (`--set-package`)
2. Global CLI overrides (`--prerelease-suffix`, `--prerelease-strategy`)
3. Package `prerelease` configuration in `releasaurus.toml`
4. Global `[prerelease]` configuration in `releasaurus.toml`
5. No prerelease (stable versions)

## Testing Prerelease Configuration

Test your prerelease settings locally:

```bash
# See what version would be generated
releasaurus release-pr --forge local --repo "."

# With command-line overrides
releasaurus release-pr \
  --prerelease-suffix beta \
  --forge local \
  --repo "."
```

## Next Steps

- [Configuration Overview](./configuration.md) - Main configuration
  guide
- [Configuration Overrides](./commands.md#configuration-overrides) -
  CLI override details
- [Quick Start](./quick-start.md) - Get started in 2 minutes
