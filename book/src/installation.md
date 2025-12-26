# Installation

Get Releasaurus installed and running in under a minute.

## Fastest: Pre-built Binary (Recommended)

Install via [cargo-binstall](https://github.com/cargo-bins/cargo-binstall)
for the quickest setup:

```bash
cargo binstall releasaurus
```

This downloads a pre-compiled binary—much faster than compiling from
source.

## Alternative Methods

### From Crates.io

Install and compile from Rust's package registry:

```bash
cargo install releasaurus
```

This compiles from source, which takes longer but ensures compatibility
with your system.

### From Source

Build the latest development version:

```bash
git clone https://github.com/robgonnella/releasaurus.git
cd releasaurus
cargo install --path .
```

**Prerequisites**: Rust 1.80+ and Git

### Docker

Use the official Docker image:

```bash
# Pull the image
docker pull rgonnella/releasaurus:latest

# Run commands
docker run --rm rgonnella/releasaurus:latest --help
```

### Manual Binary Download

Download pre-built binaries directly from the [releases
page](https://github.com/robgonnella/releasaurus/releases).

## Verify Installation

Confirm Releasaurus is working:

```bash
releasaurus --version
```

## Next Steps

→ **Ready to go?** See the [Quick Start](./quick-start.md) guide to
release your first project in under 2 minutes.

→ **Need configuration?** Check out
[Configuration](./configuration.md) for version file updates and custom
settings.
