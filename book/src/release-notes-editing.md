# Editing Release Notes

Releasaurus lets you customize the release notes for a specific release
directly in the pull request body — without touching `releasaurus.toml`
or the `CHANGELOG.md`.

## How It Works

When `release-pr` creates or updates a release PR, it renders the PR
body with a structured layout per package:

```html
<details open>
<summary>v1.2.3</summary>
<div id="my-package-header"></div>
<div id="my-package" data-tag="v1.2.3">
<!--{"metadata":{"sha_compare_link":"...","tag_compare_link":"..."}}-->

## [v1.2.3](...) - 2026-04-10
### Features
- feat: some new feature (abc1234)
</div>
<div id="my-package-footer"></div>
</details>
```

At release time, `releasaurus release` reads the notes directly from
the PR body rather than regenerating them from commit history. This
means any edits you make before merging are reflected in the published
forge release.

> **Note**: Edits to the PR body affect only the **forge release
> notes**. `CHANGELOG.md` is generated from commit history and is
> not affected.

## Editing the Release Notes

Open the PR body and edit the text inside the notes `<div>`. The
metadata comment (`<!--{...}-->`) must be left intact — it carries
the tag and link information needed at publish time.

**Before editing:**

```html
<div id="my-package" data-tag="v1.2.3">
<!--{"metadata":{"sha_compare_link":"...","tag_compare_link":"..."}}-->

## [v1.2.3](...) - 2026-04-10
### Features
- feat: some new feature (abc1234)
</div>
```

**After editing:**

```html
<div id="my-package" data-tag="v1.2.3">
<!--{"metadata":{"sha_compare_link":"...","tag_compare_link":"..."}}-->

## [v1.2.3](...) - 2026-04-10

This release improves startup performance and fixes a crash on
empty input. See the [migration guide](https://example.com) for
details.

### Features
- feat: some new feature (abc1234)
</div>
```

## Persistent Header and Footer

For content that should survive re-runs of `release-pr` (for example,
if you run the command again after new commits land), place it in the
dedicated header and footer `<div>`s.

```html
<div id="my-package-header">
## Highlights

This is a major stability release. All users on v1.x are encouraged
to upgrade.
</div>

<div id="my-package-footer">
Full migration guide: https://example.com/migrate
</div>
```

When `release-pr` regenerates the PR body, it reads back the content
of these `<div>`s and re-embeds it. The header is prepended and the
footer is appended to the final release notes at publish time.

> **Tip**: Leave the header and footer `<div>`s empty (the default)
> if you have nothing to add. They will not appear in the published
> release notes.

## Monorepo: Multiple Packages

In a monorepo, each package gets its own set of sections. The `id`
attributes are derived from the package name with any characters
outside `[a-zA-Z0-9-_]` replaced by `-`.

For example, a package named `@scope/my-pkg` gets:

- `<div id="-scope-my-pkg">` — notes
- `<div id="-scope-my-pkg-header">` — header
- `<div id="-scope-my-pkg-footer">` — footer

Edit each package's sections independently.

## Backward Compatibility

PRs created by an older version of Releasaurus use a different body
format. The `release` command detects the format automatically and
falls back to reading release notes from the hidden metadata for those
PRs — no manual migration required.

## Limits

- The metadata comment (`<!--{...}-->`) inside the notes `<div>` must
  not be removed or modified.
- Header and footer content is preserved verbatim. Markdown is
  supported by all major forge platforms.
- Re-running `release-pr` regenerates the notes from commit history
  and overwrites any direct edits to the notes `<div>`. Use the
  header/footer sections for content you want to survive re-runs.
