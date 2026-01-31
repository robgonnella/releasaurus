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

| Language    | release_type | Files Updated                                                                    |
| ----------- | ------------ | -------------------------------------------------------------------------------- |
| **Generic** | `"generic"`  | Custom files via `additional_manifest_files`                                     |
| **Go**      | `"go"`       | version.go, version/version.go, internal/version.go, internal/version/version.go |
| **Java**    | `"java"`     | pom.xml, build.gradle, build.gradle.kts                                          |
| **Node.js** | `"node"`     | package.json, package-lock.json, yarn.lock                                       |
| **PHP**     | `"php"`      | composer.json, composer.lock                                                     |
| **Python**  | `"python"`   | pyproject.toml, setup.py, setup.cfg                                              |
| **Ruby**    | `"ruby"`     | \*.gemspec, Gemfile, Gemfile.lock                                                |
| **Rust**    | `"rust"`     | Cargo.toml, Cargo.lock                                                           |

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
