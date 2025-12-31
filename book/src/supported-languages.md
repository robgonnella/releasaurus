# Supported Languages & Frameworks

Releasaurus provides native support for multiple programming languages.
Configure your project's `release_type` in `releasaurus.toml`, and
Releasaurus automatically updates all relevant version files.

## Quick Configuration

```toml
[[package]]
path = "."
release_type = "node"  # or rust, python, java, php, ruby, generic
```

## Language Support Reference

| Language    | release_type | Files Updated                                |
| ----------- | ------------ | -------------------------------------------- |
| **Rust**    | `"rust"`     | Cargo.toml, Cargo.lock                       |
| **Node.js** | `"node"`     | package.json, package-lock.json, yarn.lock   |
| **Python**  | `"python"`   | pyproject.toml, setup.py, setup.cfg          |
| **Java**    | `"java"`     | pom.xml, build.gradle, build.gradle.kts      |
| **PHP**     | `"php"`      | composer.json, composer.lock                 |
| **Ruby**    | `"ruby"`     | \*.gemspec, Gemfile, Gemfile.lock            |
| **Generic** | `"generic"`  | Custom files via `additional_manifest_files` |

## Notes

- **Workspaces/Monorepos**: All languages support workspace
  configurations with multiple packages
- **Lock files**: Automatically updated when present
- **Generic projects**: Use `additional_manifest_files` to specify
  custom version file patterns. See
  [Configuration Reference](./configuration-reference.md#additional_manifest_files) for
  details.

## Next Steps

For complete configuration options including monorepo setup, changelog
customization, and prerelease versions, see the
[Configuration](./configuration.md) guide.
