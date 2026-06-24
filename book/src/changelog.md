# Changelog Customization

Releasaurus generates changelogs from conventional commits. Two sections
of `releasaurus.toml` control the result:

- `[defaults.versioning]` — which commits are included and how they're
  grouped. These settings affect the **version bump** as well as the
  changelog, which is why they live alongside the versioning options.
- `[defaults.changelog]` — how the included commits are rendered (the Tera
  template and display flags).

Both can also be set [per package](#per-package-changelog).

## Commit Groups & Filtering

Each commit is matched against a set of **parsers**. A parser decides
which `group` (changelog heading) a commit belongs to, and whether the
commit is skipped entirely. Configure them in `[defaults.versioning]`.

A parser has four fields:

| Field     | Type   | Effect                                                                                        |
| --------- | ------ | --------------------------------------------------------------------------------------------- |
| `pattern` | regex  | Matched against the raw commit message to decide if the parser applies                        |
| `title`   | string | The changelog heading commits in this group appear under                                      |
| `order`   | int    | Position of the heading in the changelog, `0`-`99`, lowest first                              |
| `skip`    | bool   | When `true`, matching commits are dropped from **both** the changelog and version calculation |

### Built-in groups (`named_parsers`)

Releasaurus ships with these default parsers:

| Group (toml key) | Pattern     | Default title            | Order |
| ---------------- | ----------- | ------------------------ | ----- |
| `breaking`       | _(none)_    | `❌ Breaking`            | `0`   |
| `feature`        | `^feat`     | `🚀 Features`            | `1`   |
| `fix`            | `^fix`      | `🐛 Bug Fixes`           | `2`   |
| `revert`         | `^revert`   | `◀️ Revert`              | `3`   |
| `refactor`       | `^refactor` | `🚜 Refactor`            | `4`   |
| `performance`    | `^perf`     | `⚡ Performance`         | `5`   |
| `documentation`  | `^doc`      | `📚 Documentation`       | `6`   |
| `style`          | `^style`    | `🎨 Styling`             | `7`   |
| `test`           | `^test`     | `🧪 Testing`             | `8`   |
| `chore`          | `^chore`    | `🧹 Chore`               | `9`   |
| `ci`             | `^ci`       | `⏩ CI/CD`               | `10`  |
| `miscellaneous`  | `.*`        | `⚙️ Miscellaneous Tasks` | `11`  |

`breaking` has no default `pattern` — breaking changes are detected via
conventional-commit syntax (`feat!:`, `BREAKING CHANGE:`). Setting a
`pattern` on `breaking` uses your pattern instead, but only for
changelog grouping: the version bump is always computed from
conventional-commit syntax, so a `feat!:` still bumps major even if your
pattern routes it under another group.

Override only the fields you want to change under
`[defaults.versioning.named_parsers]`; everything you omit falls back to the
built-in default. For example, to drop CI and chore commits — the only
change needed is `skip`:

```toml
[defaults.versioning.named_parsers]
ci.skip = true
chore.skip = true
```

To skip a group, set its `skip = true`. You can also retitle a group,
move it, or change its matching pattern the same way. A retitle does not
move the group — position comes from `order` alone:

```toml
[defaults.versioning.named_parsers]
feature.title = "✨ New Stuff"
fix.order = 1                   # bug fixes above features
feature.order = 2
```

### Custom groups (`custom_parser`)

Define entirely new groups with `[[defaults.versioning.custom_parser]]`.
Note the key is singular, matching the `[[package]]` convention. Each
custom parser is checked **before** the built-in parsers, so it takes
precedence over the defaults:

```toml
[[defaults.versioning.custom_parser]]
pattern = "^deps"
title = "📦 Dependencies"
order = 3
skip = false
```

Unlike named parsers, custom parsers have no defaults to fall back on:
`pattern`, `title` and `order` are all required. Omitting any of them is
a configuration error.

Because custom parsers are checked first, they also win over `breaking` —
so a custom parser with `skip = true` drops matching commits even when
they are breaking changes, removing them from the changelog **and** from
the version bump. Keep custom patterns narrow, or leave `skip = false`
if you only want to regroup commits rather than discard them.

### Ordering groups

Each group's `order` places its heading in the changelog, lowest first
(see the table above for the built-in values). Groups sharing an `order`
fall back to title order.

Order is independent of the heading text, so retitling a group never
moves it. Mechanically, `order` is rendered into the `group` attribute as
an `<!-- NN -->` prefix, which the default template sorts on and then
strips:

```tera
{% ... | sort(attribute="group") | group_by(attribute="group") %}
### {{ group | striptags | trim }}
```

A custom template that sorts on `group` gets ordering for free; one that
prints `{{ group }}` without `striptags` will show the prefix. See
[The `body` Template](#the-body-template) below for the full template.

### Other options

In `[defaults.versioning]`:

| Option               | Default | Effect                 |
| -------------------- | ------- | ---------------------- |
| `skip_merge_commits` | `true`  | Excludes merge commits |

In `[defaults.changelog]`:

| Option                  | Default | Effect                                                                                                                                                |
| ----------------------- | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `include_author`        | `false` | Adds the commit author's name to each entry                                                                                                           |
| `aggregate_prereleases` | `false` | When graduating a prerelease to stable, folds in the changelog entries from all prior prereleases (see [Prereleases](./configuration.md#prereleases)) |

To drop specific commits entirely or rewrite their messages — which also
affects the version bump — see "Skipping or Rewording Commits" in the
[configuration guide](./configuration.md#skipping-or-rewording-commits).

## Per-package changelog

Everything on this page applies to every package by default. To customize
a single package, set the same fields on that package's `changelog` and
`versioning` keys — matching the `[defaults]` table each option belongs
to.
Packages are an array of tables (`[[package]]`), so use an **inline
table** to keep it scoped to the right entry:

```toml
[[package]]
name = "frontend"
path = "./apps/web"
release_type = "node"
changelog = { include_author = true }
versioning = { named_parsers = { ci = { skip = true } } }
```

Both keys **merge field-by-field** with their `[defaults]` counterpart:
any field you set on the package wins, and any field you omit is
inherited from `[defaults]` (falling back to the built-in default).
`custom_parser` entries from `[defaults]` and the package are combined,
with the package's checked first, and `named_parsers` overrides apply per
group and per field — so the example above turns on `include_author` and
skips `ci` for `frontend` while still inheriting every other default. The
one
exception is `versioning.prerelease`, which is replaced as a whole table
rather than merged. See
[Per-package overrides](./configuration-reference.md#per-package-overrides)
in the reference for the exact precedence rules.

## The `body` Template

`body` is a [Tera](https://keats.github.io/tera/) template rendered once
per release. The default groups commits by type, links each commit, and
highlights breaking changes:

```toml
[defaults.changelog]
body = """# [{{ version  }}]{% if tag_compare_link %}({{ tag_compare_link }}){% else %}({{ link }}){% endif %} - {{ timestamp | date(format="%Y-%m-%d") }}
{% for group, commits in commits | filter(attribute="merge_commit", value=false) | sort(attribute="group") | group_by(attribute="group") %}
### {{ group | striptags | trim }}
{% for commit in commits %}
{% if commit.breaking -%}
{% if commit.scope %}_({{ commit.scope }})_ {% endif -%}[**breaking**]: {{ commit.title }} [_({{ commit.short_id }})_]({{ commit.link }}){% if include_author %} ({{ commit.author_name }}){% endif %}
{% if commit.body -%}
> {{ commit.body }}
{% endif -%}
{% if commit.breaking_description -%}
> {{ commit.breaking_description }}
{% endif -%}
{% else -%}
- {% if commit.scope %}_({{ commit.scope }})_ {% endif %}{{ commit.title }} [_({{ commit.short_id }})_]({{ commit.link }}){% if include_author %} ({{ commit.author_name }}){% endif %}
{% endif -%}
{% endfor %}
{% endfor %}"""
```

A simpler custom template:

```toml
[defaults.changelog]
body = """## Release v{{ version }} — {{ timestamp | date(format="%Y-%m-%d") }}

{% for group, commits in commits | group_by(attribute="group") %}
### {{ group }}
{% for commit in commits %}
- {{ commit.title }} ({{ commit.short_id }}){% if include_author %} by {{ commit.author_name }}{% endif %}
{% endfor %}
{% endfor %}"""
```

## Template Variables

### Release

| Variable           | Description                                                    |
| ------------------ | -------------------------------------------------------------- |
| `version`          | Semantic version (e.g. `1.2.3`)                                |
| `tag_name`         | Full tag including prefix/suffix                               |
| `link`             | URL to the release                                             |
| `tag_compare_link` | Diff vs. previous tag (empty for first release)                |
| `sha_compare_link` | Diff vs. previous tag, by commit SHA (empty for first release) |
| `sha`              | Release commit SHA                                             |
| `timestamp`        | Unix timestamp                                                 |
| `include_author`   | Whether author display is enabled                              |

### Commit (each item in `commits`)

| Variable                            | Description                          |
| ----------------------------------- | ------------------------------------ |
| `id` / `short_id`                   | Full / abbreviated SHA               |
| `group`                             | Category (Features, Bug Fixes, …)    |
| `scope`                             | Optional conventional-commit scope   |
| `title`                             | Message without type/scope           |
| `body`                              | Optional extended description        |
| `link`                              | URL to the commit                    |
| `breaking` / `breaking_description` | Breaking-change flag and details     |
| `merge_commit`                      | Whether it's a merge commit          |
| `timestamp`                         | Commit timestamp                     |
| `author_name` / `author_email`      | Commit author                        |
| `raw_title` / `raw_message`         | Original unprocessed title / message |

## Tips

Filter merge commits and conditionally show authors:

```tera
{% for commit in commits | filter(attribute="merge_commit", value=false) %}
- {{ commit.title }}{% if include_author %} <{{ commit.author_name }}>{% endif %}
{% endfor %}
```

Test any template change locally before committing it:

```bash
releasaurus release-pr --forge local --repo "."
```

See the [Tera documentation](https://keats.github.io/tera/) for advanced
filtering and formatting.
