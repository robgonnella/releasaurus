# Monorepo Configuration

Manage multiple independently-versioned packages within a single
repository.

## Overview

Releasaurus supports monorepos with:
- Multiple packages with independent versions
- Separate or combined pull requests
- Custom tag prefixes per package
- Per-package configuration overrides

## Quick Configuration

### Basic Monorepo (Combined Releases)

All packages released together in one PR:

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

Tags: `frontend-v1.0.0`, `backend-v2.3.0`

### Independent Releases (Separate PRs)

Each package gets its own PR:

```toml
separate_pull_requests = true

[[package]]
path = "./frontend"
release_type = "node"
tag_prefix = "frontend-v"

[[package]]
path = "./backend"
release_type = "rust"
tag_prefix = "backend-v"
```

Creates separate PRs like:
- `releasaurus-release-main-frontend`
- `releasaurus-release-main-backend`

## Configuration Options

### `separate_pull_requests` (default: false)

Control PR creation strategy:

```toml
separate_pull_requests = true  # or false
```

**false (default)** - Single PR for all packages
**true** - Separate PR for each package with changes

### Package Configuration

Each package needs:

- `path` - Directory relative to repository root
- `release_type` - Language/framework (rust, node, python, etc.)
- `tag_prefix` (optional) - Custom tag prefix

**Tag prefix defaults:**
- Root packages (`path = "."`) → `"v"`
- Nested packages → `"<package-name>-v"`

## When to Use Each Strategy

### Single PR Mode (Default)

**Best for:**
- Tightly coupled packages that always release together
- Small monorepos (2-5 packages)
- Teams that prefer coordinated releases
- Simple review workflow

**Benefits:**
- One review process
- Atomic releases
- Simpler workflow

### Separate PR Mode

**Best for:**
- Large monorepos (5+ packages)
- Independently versioned packages
- Different teams owning different packages
- Different release cadences per package

**Benefits:**
- Independent release cycles
- Parallel reviews
- Flexible scheduling

## Practical Examples

### Frontend + Backend + Shared Library

```toml
separate_pull_requests = true

[[package]]
path = "./apps/web"
release_type = "node"
tag_prefix = "web-v"

[[package]]
path = "./apps/api"
release_type = "rust"
tag_prefix = "api-v"

[[package]]
path = "./packages/shared"
release_type = "python"
tag_prefix = "shared-v"
```

### Microservices Repository

```toml
separate_pull_requests = true

[[package]]
path = "./services/auth"
release_type = "node"
tag_prefix = "auth-v"

[[package]]
path = "./services/payments"
release_type = "node"
tag_prefix = "payments-v"

[[package]]
path = "./services/notifications"
release_type = "python"
tag_prefix = "notifications-v"

[[package]]
path = "./services/analytics"
release_type = "rust"
tag_prefix = "analytics-v"
```

### Workspace in Subdirectory

For workspaces not at repository root:

```toml
[[package]]
name = "api-server"
workspace_root = "backend"
path = "services/api"
release_type = "rust"
tag_prefix = "api-v"

[[package]]
name = "worker"
workspace_root = "backend"
path = "services/worker"
release_type = "rust"
tag_prefix = "worker-v"
```

This updates:
- `backend/services/api/Cargo.toml`
- `backend/services/worker/Cargo.toml`
- `backend/Cargo.lock` (workspace lock file)

### Mixed Prerelease States

Some packages stable, others in beta:

```toml
separate_pull_requests = true

[[package]]
path = "./stable-api"
release_type = "rust"
tag_prefix = "api-v"
# No prerelease - stable only

[[package]]
path = "./experimental-features"
release_type = "rust"
tag_prefix = "exp-v"
prerelease = { suffix = "beta", strategy = "versioned" }
```

## Shared Code Patterns

### Packages with Shared Dependencies

Use `additional_paths` to track shared code:

```toml
[[package]]
path = "./apps/web"
release_type = "node"
tag_prefix = "web-v"
additional_paths = ["shared/types", "shared/utils"]

[[package]]
path = "./apps/mobile"
release_type = "node"
tag_prefix = "mobile-v"
additional_paths = ["shared/types", "shared/utils"]
```

Both packages release when changes occur in `shared/*`.

### Dedicated Shared Package

```toml
[[package]]
path = "./shared"
release_type = "node"
tag_prefix = "shared-v"

[[package]]
path = "./apps/web"
release_type = "node"
tag_prefix = "web-v"

[[package]]
path = "./apps/mobile"
release_type = "node"
tag_prefix = "mobile-v"
```

The shared package releases independently.

## Workflow Differences

### Single PR Workflow

1. Run `releasaurus release-pr`
2. One PR created with all package updates
3. Review and merge
4. Run `releasaurus release`
5. All packages tagged and released

### Separate PR Workflow

1. Run `releasaurus release-pr`
2. Multiple PRs created (one per package with changes)
3. Review and merge each PR independently
4. Run `releasaurus release` after each merge
5. Each package tagged and released separately

## Auto Start Next

Automatically bump versions after release:

```toml
auto_start_next = true

[[package]]
path = "./api"
release_type = "rust"
auto_start_next = false  # Override for this package

[[package]]
path = "./web"
release_type = "node"
# Uses global setting (true)
```

See [`start-next` command](./commands.md#start-next) for details.

## Testing Monorepo Configuration

Test locally to verify package detection and tagging:

```bash
# See what packages would be released
releasaurus release-pr --forge local --repo "."

# Check:
# - All packages detected correctly
# - Tag prefixes match expectations
# - Separate/combined PR strategy works
```

## Next Steps

- [Configuration Overview](./configuration.md) - Main configuration
  guide
- [Prerelease Configuration](./configuration-prerelease.md) - Per-
  package prereleases
- [Commands](./commands.md) - All command options
