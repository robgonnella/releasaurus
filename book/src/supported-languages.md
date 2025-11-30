# Supported Languages & Frameworks

Releasaurus provides native support for a wide range of programming languages
and frameworks. You configure your project's `release_type` in
`releasaurus.toml`, and Releasaurus handles version updates across all relevant
files for that language.

## Configuration

Each package in your repository requires a `release_type` configuration to
enable version file updates:

```toml
[[package]]
path = "."
release_type = "node"  # Specify your language/framework
```

Available release types: `"Rust"`, `"Node"`, `"Python"`, `"Java"`, `"Php"`,
`"Ruby"`, `"Generic"`

## Supported Languages

### Rust

**Configuration**: `release_type = "rust"`

**Updated Files**: `Cargo.toml`, `Cargo.lock` (if present)

Rust projects using Cargo for dependency management and packaging. Supports
both library and binary crates, workspace configurations, and dependency
version updates.

**Example Configuration**:

```toml
[[package]]
path = "."
release_type = "rust"
```

**Example Project Structure**:

```
my-rust-project/
├── Cargo.toml          # ← Version updated here
├── Cargo.lock          # ← Automatically updated
├── src/
│   └── lib.rs
└── README.md
```

### Node.js

**Configuration**: `release_type = "node"`

**Updated Files**: `package.json`, `package-lock.json`, `yarn.lock`,
`pnpm-lock.yaml`

JavaScript and TypeScript projects using npm, Yarn, or pnpm package managers.
Supports monorepos, workspaces, and dependency updates.

**Example Configuration**:

```toml
[[package]]
path = "."
release_type = "node"
```

**Example Project Structure**:

```
my-node-project/
├── package.json        # ← Version updated here
├── package-lock.json   # ← Automatically updated
├── src/
│   └── index.js
└── README.md
```

### Python

**Configuration**: `release_type = "python"`

**Updated Files**: `pyproject.toml`, `setup.py`, `setup.cfg`, `__init__.py`,
`requirements*.txt`

Python projects using modern packaging standards (PEP 518/517) or legacy
setuptools. Supports Poetry, setuptools, and custom version patterns.

**Example Configuration**:

```toml
[[package]]
path = "."
release_type = "python"
```

**Example Project Structure**:

```
my-python-project/
├── pyproject.toml      # ← Primary version location
├── requirements.txt    # ← Version references updated
├── src/
│   └── my_package/
│       └── __init__.py # ← Version string updated
└── README.md
```

### Java

**Configuration**: `release_type = "java"`

**Updated Files**: Maven POMs, Gradle build files, version properties

Java projects using Maven or Gradle build systems. Supports multi-module
projects, parent POMs, and version property management.

**Example Configuration**:

```toml
[[package]]
path = "."
release_type = "java"
```

**Example Maven Project**:

```
my-java-project/
├── pom.xml             # ← Version updated here
├── src/
│   └── main/java/
└── README.md
```

**Example Gradle Project**:

```
my-gradle-project/
├── build.gradle        # ← Version updated here
├── gradle.properties   # ← Version properties updated
├── src/
│   └── main/java/
└── README.md
```

### PHP

**Configuration**: `release_type = "php"`

**Updated Files**: `composer.json`, `composer.lock` (regenerated)

PHP projects using Composer for dependency management. Supports packages,
applications, and dependency constraint updates.

**Example Configuration**:

```toml
[[package]]
path = "."
release_type = "php"
```

**Example Project Structure**:

```
my-php-project/
├── composer.json       # ← Version updated here
├── composer.lock       # ← Automatically updated
├── src/
│   └── MyClass.php
└── README.md
```

### Ruby

**Configuration**: `release_type = "ruby"`

**Updated Files**: Gemspec files, version files, Gemfile dependencies

Ruby projects using Bundler and RubyGems. Supports gems, applications, and
version constant management.

**Example Configuration**:

```toml
[[package]]
path = "."
release_type = "ruby"
```

**Example Gem Project**:

```
my-ruby-gem/
├── my_gem.gemspec      # ← Version updated here
├── Gemfile
├── lib/
│   └── my_gem/
│       └── version.rb  # ← Version constant updated
└── README.md
```

### Generic Projects

**Configuration**: `release_type = "generic"`

Generic projects receive changelog generation and tagging only. By default,
version files are not modified automatically. However, you can enable version
file updates by using the `additional_manifest_files` configuration option,
which applies generic version pattern matching to specified files.

This is useful for documentation repositories, configuration projects, or
languages not yet supported by framework-specific updaters.

**Example Configuration**:

```toml
[[package]]
path = "."
release_type = "generic"
```

**Example with Version File Updates**:

```toml
[[package]]
path = "."
release_type = "generic"
additional_manifest_files = [
  "VERSION",
  "README.md"
]
```

For details on configuring additional manifest files, see the
[`additional_manifest_files`](./configuration.md#additional_manifest_files)
section in the Configuration documentation.

## Framework-Specific Features

### Version File Detection

Each language ecosystem has different conventions for where version information
is stored:

- **Rust**: Single source of truth in `Cargo.toml`
- **Node.js**: Primary in `package.json`, locks updated automatically
- **Python**: Multiple possible locations (pyproject.toml, setup.py,
  **init**.py)
- **Java**: Maven coordinates or Gradle version properties
- **PHP**: Composer metadata with semantic versioning
- **Ruby**: Gemspec version specifications

### Dependency Updates

Some frameworks support updating dependency versions during releases:

- **Node.js**: Updates lock files and can bump dependencies
- **Rust**: Updates Cargo.lock automatically
- **PHP**: Regenerates composer.lock with new constraints
- **Python**: Can update version pins in requirements files

### Build System Integration

Releasaurus integrates with various build systems:

- **Cargo** (Rust): Workspace and crate management
- **npm/yarn/pnpm** (Node.js): Workspace and package management
- **Poetry/setuptools** (Python): Build backend integration
- **Maven/Gradle** (Java): Multi-module project support
- **Bundler** (Ruby): Gem dependency management

## Multi-Language Projects

For projects that use multiple languages, configure separate packages with
their respective `release_type`:

```toml
# releasaurus.toml
[[package]]
path = "./backend"
release_type = "rust"
tag_prefix = "api-v"

[[package]]
path = "./frontend"
release_type = "node"
tag_prefix = "ui-v"

[[package]]
path = "./scripts"
release_type = "python"
tag_prefix = "scripts-v"
```

Each package is processed independently with its own version calculation,
changelog, and release cycle.

## Adding New Languages

Releasaurus is designed to be extensible. Each language implementation
includes an updater that handles version file modifications and
language-specific patterns. See the [Contributing](./contributing.md) guide
for information about adding support for new languages and frameworks.

## Next Steps

- Refer to the language-specific sections above for detailed information about
  supported features
- Explore [Configuration](./configuration.md) for customization options
