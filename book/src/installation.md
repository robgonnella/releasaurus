# Installation

This guide covers different methods to install Releasaurus on your system.

## System Requirements

- **Operating System**: Linux, macOS, or Windows
- **Internet Access**: Required for API calls to Git forge platforms

## Installation Methods

### Option 1: Install from Crates.io (Recommended)

The easiest way to install Releasaurus is using Cargo, Rust's package
manager:

```bash
cargo install releasaurus
```

This will download, compile, and install the latest stable version of
Releasaurus. The binary will be available in your `$HOME/.cargo/bin`
directory, which should be in your system's PATH.

### Option 2: Download Pre-built Binaries

Coming soon...

### Option 3: Build from Source

If you prefer to build from source or need the latest development features:

#### Prerequisites

- [Rust](https://rustup.rs/) 1.70 or higher
- Git

#### Build Steps

```bash
# Clone the repository
git clone https://github.com/your-username/releasaurus.git
cd releasaurus

# Build and install
cargo install --path .
```

This will build the project in release mode and install it to your Cargo bin
directory.

### Option 4: Using Docker

If you prefer to use Docker, you can pull and run the official Releasaurus
Docker image:

#### Pull the Docker Image

```bash
docker pull rgonnella/releasaurus:latest
```

#### Run Releasaurus with Docker

You can run Releasaurus directly using Docker.

```bash
# Run from your project directory
docker run --rm rgonnella/releasaurus:latest --help
```

## Verify Installation

After installation, verify that Releasaurus is working correctly:

```bash
releasaurus --help
```

You should see the help output with available commands and options.

Check the installed version:

```bash
releasaurus --version
```

## Next Steps

Now that you have Releasaurus installed, head over to the
[Quick Start](./quick-start.md) guide to learn how to use it with your first
project, or check out the [Configuration](./configuration.md) if
you want to customize the default behavior.
