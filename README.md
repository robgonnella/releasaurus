[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/robgonnella/releasaurus)

# Releasaurus

A comprehensive release automation tool that streamlines the software release
process across multiple programming languages and forge platforms.

## Documentation

For complete documentation, installation instructions, and usage examples,
for current release, please visit:

**[https://releasaurus.rgon.io](https://releasaurus.rgon.io)**

For documentation for tip of `main`, view mdbook documentation
[SUMMARY.md](./book/src/SUMMARY.md).

If you install mdbook, you can build and view documentation locally as html via

```bash
cd book
mdbook serve --open
```

## Upgrading to v1.0.0

v1.0.0 restructures `releasaurus.toml`; a v0.22.x config will not load.
See the [migration guide](./book/src/migration-0.22-to-1.0.md)
([published version](https://releasaurus.rgon.io/migration-0.22-to-1.0.html)).

## License

Licensed under either of

- Apache License, Version 2.0
- MIT License
