# Troubleshooting

Common issues and how to diagnose them. If your problem isn't covered
here, check the
[GitHub issues](https://github.com/robgonnella/releasaurus/issues).

## Inspect Before You Run

`get next-release` shows exactly what Releasaurus would do — version,
included commits, and release notes — without making any changes. It's
the fastest way to debug version detection, tag matching, and config:

```bash
releasaurus get next-release --repo "https://github.com/owner/repo"

# Or fully offline against a local checkout
releasaurus get next-release --forge local --repo "."
```

For deeper diagnostics, add `--debug` (or `--dry-run`, which also enables
debug). See [Testing Modes](./commands.md#testing-modes).

## Releasaurus doesn't find existing tags

Usually a **tag prefix mismatch** — the package's `tag_prefix` must match
your existing tags:

```toml
[[package]]
path = "."
tag_prefix = "v"   # for v1.0.0; use "api-v" for api-v1.0.0; "" for 1.0.0
```

If no matching tag exists, Releasaurus treats it as a first release and
analyzes up to `first_release_search_depth` commits (default 400). Raise
it for a fuller first changelog, or lower it for speed. This affects
**only** the first release — once a matching tag exists, all commits back
to that tag are analyzed. To control how many tags are fetched while
searching, use `tag_search_depth`.

## "Authentication failed" / 401 Unauthorized

1. **Confirm the token is set** for the forge you're targeting
   (`echo $GITHUB_TOKEN`), or pass `--token` explicitly.
2. **Check the scopes** — see
   [required token scopes](./configuration-reference.md#required-token-scopes).
3. **Check expiration** — regenerate if expired.

## "Repository not found" with a valid repo

The token lacks access, or the URL is wrong.

1. **Verify the URL format**, e.g.
   `--repo "https://github.com/owner/repository"`.
2. **Confirm the token's account** has access to the repository.
3. **Reproduce offline** to rule out config issues:

   ```bash
   git clone https://github.com/owner/repo && cd repo
   releasaurus get next-release --forge local --repo "."
   ```

## "must finish previous release first"

`release-pr` found a merged-but-not-yet-tagged release PR. Run `release`
first to tag it, then `release-pr`. In CI, always order the two commands
`release` → `release-pr` in a single job — see
[CI/CD Integration](./ci-cd-integration.md).

## Getting Help

When opening an issue, include: debug output (with secrets removed), your
repository structure, the exact command, expected vs. actual behavior,
your OS, `releasaurus --version`, and your forge platform and hosting
type.
