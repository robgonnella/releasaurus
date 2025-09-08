# Supported Languages & Frameworks

Releasaurus provides native support for a wide range of programming languages and frameworks. The tool automatically detects your project's language and framework, then handles version updates across all relevant files without requiring manual configuration.

## Automatic Detection

When you run Releasaurus, it automatically scans your project directory to identify:

1. **Primary language/framework** based on manifest files
2. **Version files** that need updating
3. **Project structure** and dependencies
4. **Release patterns** appropriate for your ecosystem

This detection happens transparently—you don't need to specify your project type or configure file patterns.

## Detection Priority

When multiple language indicators are present, Releasaurus uses this priority order:

1. **Rust** - `Cargo.toml` presence
2. **Python** - `pyproject.toml`, `setup.py`, or `setup.cfg`
3. **Node.js** - `package.json` presence
4. **PHP** - `composer.json` presence
5. **Java** - `pom.xml` or `build.gradle*` files
6. **Ruby** - `Gemfile` or `*.gemspec` files
7. **Generic** - Custom patterns or unrecognized projects

## Supported Languages

### Rust

**Detection Files**: `Cargo.toml`
**Updated Files**: `Cargo.toml`, `Cargo.lock` (if present)

Rust projects using Cargo for dependency management and packaging. Supports both library and binary crates, workspace configurations, and dependency version updates.

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

**Detection Files**: `package.json`
**Updated Files**: `package.json`, `package-lock.json`, `yarn.lock`, `pnpm-lock.yaml`

JavaScript and TypeScript projects using npm, Yarn, or pnpm package managers. Supports monorepos, workspaces, and dependency updates.

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

**Detection Files**: `pyproject.toml`, `setup.py`, `setup.cfg`
**Updated Files**: `pyproject.toml`, `setup.py`, `setup.cfg`, `__init__.py`, `requirements*.txt`

Python projects using modern packaging standards (PEP 518/517) or legacy setuptools. Supports Poetry, setuptools, and custom version patterns.

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

**Detection Files**: `pom.xml`, `build.gradle`, `build.gradle.kts`
**Updated Files**: Maven POMs, Gradle build files, version properties

Java projects using Maven or Gradle build systems. Supports multi-module projects, parent POMs, and version property management.

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

**Detection Files**: `composer.json`
**Updated Files**: `composer.json`, `composer.lock` (regenerated)

PHP projects using Composer for dependency management. Supports packages, applications, and dependency constraint updates.

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

**Detection Files**: `Gemfile`, `*.gemspec`
**Updated Files**: Gemspec files, version files, Gemfile dependencies

Ruby projects using Bundler and RubyGems. Supports gems, applications, and version constant management.

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

**Detection**: Fallback when no specific language is detected

Right now anything that falls back to "Generic" will only receive changelog
updates and tagging. Any specific version manifests will be left untouched.
In the future we will support the configuration of generic manifest files
that can be updated via regex-able version properties.

## Framework-Specific Features

### Version File Detection

Each language ecosystem has different conventions for where version information is stored:

- **Rust**: Single source of truth in `Cargo.toml`
- **Node.js**: Primary in `package.json`, locks updated automatically
- **Python**: Multiple possible locations (pyproject.toml, setup.py, **init**.py)
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

For projects that use multiple languages, Releasaurus can handle them in several ways:

1. **Primary Language Detection**: Uses the highest-priority language found
2. **Multi-Package Configuration**: Define separate packages for each language
3. **Generic Fallback**: Use generic patterns for unsupported combinations

Example multi-language project configuration:

```toml
# releasaurus.toml
[[package]]
path = "./backend"      # Rust API
tag_prefix = "api-v"

[[package]]
path = "./frontend"     # Node.js frontend
tag_prefix = "ui-v"

[[package]]
path = "./scripts"      # Python utilities
tag_prefix = "scripts-v"
```

## Adding New Languages

Releasaurus is designed to be extensible. Each language implementation includes:

- **Detector**: Identifies if the language is present
- **Updater**: Handles version file modifications
- **Framework Integration**: Language-specific patterns and conventions

See the [Contributing](./contributing.md) guide for information about adding support for new languages and frameworks.

## Next Steps

- Refer to the language-specific sections above for detailed information about supported features
- Explore [Configuration](./configuration.md) for customization options
