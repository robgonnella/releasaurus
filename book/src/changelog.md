# Changelog Customization

Releasaurus generates changelogs from conventional commits. Control what
appears with the filtering options, and how it's formatted with a Tera
template — both in the `[global.changelog]` section of `releasaurus.toml`
(or [per package](#per-package-changelog)).

## Commit Groups & Filtering

Each commit is matched against a set of **parsers**. A parser decides
which `group` (changelog heading) a commit belongs to, and whether the
commit is skipped entirely. Configure them in `[global.changelog]`.

A parser has three fields:

| Field     | Type   | Effect                                                                                        |
| --------- | ------ | --------------------------------------------------------------------------------------------- |
| `pattern` | regex  | Matched against the raw commit message to decide if the parser applies                        |
| `title`   | string | The changelog heading commits in this group appear under                                      |
| `skip`    | bool   | When `true`, matching commits are dropped from **both** the changelog and version calculation |

### Built-in groups (`named_parsers`)

Releasaurus ships with these default parsers:

| Group (toml key) | Pattern     | Default title                       |
| ---------------- | ----------- | ----------------------------------- |
| `breaking`       | _(none)_    | `<!-- 00 -->❌ Breaking`            |
| `feature`        | `^feat`     | `<!-- 01 -->🚀 Features`            |
| `fix`            | `^fix`      | `<!-- 02 -->🐛 Bug Fixes`           |
| `revert`         | `^revert`   | `<!-- 03 -->◀️ Revert`              |
| `refactor`       | `^refactor` | `<!-- 04 -->🚜 Refactor`            |
| `performance`    | `^perf`     | `<!-- 05 -->⚡ Performance`         |
| `documentation`  | `^doc`      | `<!-- 06 -->📚 Documentation`       |
| `style`          | `^style`    | `<!-- 07 -->🎨 Styling`             |
| `test`           | `^test`     | `<!-- 08 -->🧪 Testing`             |
| `chore`          | `^chore`    | `<!-- 09 -->🧹 Chore`               |
| `ci`             | `^ci`       | `<!-- 10 -->⏩ CI/CD`               |
| `miscellaneous`  | `.*`        | `<!-- 11 -->⚙️ Miscellaneous Tasks` |

`breaking` has no default `pattern` — breaking changes are detected via
conventional-commit syntax (`feat!:`, `BREAKING CHANGE:`). Setting a
`pattern` on `breaking` uses your pattern instead, but only for
changelog grouping: the version bump is always computed from
conventional-commit syntax, so a `feat!:` still bumps major even if your
pattern routes it under another group.

Override only the fields you want to change under
`[global.changelog.named_parsers]`; everything you omit falls back to the
built-in default. For example, to drop CI and chore commits — the only
change needed is `skip`:

```toml
[global.changelog.named_parsers]
ci.skip = true
chore.skip = true
```

To skip a group, set its `skip = true`. You can also retitle a group or
change its matching pattern the same way:

```toml
[global.changelog.named_parsers]
feature.title = "<!-- 01 -->✨ New Stuff"
```

### Custom groups (`custom_parsers`)

Define entirely new groups with `[[global.changelog.custom_parsers]]`.
Each custom parser is checked **before** the built-in parsers, so it
takes precedence over the defaults:

```toml
[[global.changelog.custom_parsers]]
pattern = "^deps"
title = "<!-- 02 -->📦 Dependencies"
skip = false
```

### Ordering groups

Groups are sorted by their `title`, and the default template sorts on
the `group` attribute. To control ordering independently of the visible
text, prefix each title with an HTML-comment index tag of the form
`<!-- NN -->` (see the tables above). The default template sorts on this
tag and then strips it before rendering:

```tera
{% ... | sort(attribute="group") | group_by(attribute="group") %}
### {{ group | striptags | trim }}
```

So `<!-- 02 -->📦 Dependencies` sorts after `<!-- 01 -->🚀 Features` but
renders as just `📦 Dependencies`. See
[The `body` Template](#the-body-template) below for the full template.

### Other options

| Option                  | Default | Effect                                                                                                                                                |
| ----------------------- | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `skip_merge_commits`    | `true`  | Excludes merge commits                                                                                                                                |
| `include_author`        | `false` | Adds the commit author's name to each entry                                                                                                           |
| `aggregate_prereleases` | `false` | When graduating a prerelease to stable, folds in the changelog entries from all prior prereleases (see [Prereleases](./configuration.md#prereleases)) |

To drop specific commits entirely or rewrite their messages — which also
affects the version bump — see "Skipping or Rewording Commits" in the
[configuration guide](./configuration.md#skipping-or-rewording-commits).

## Per-package changelog

Everything on this page lives under `[global.changelog]` and applies to
every package. To customize the changelog for a single package, set the
same fields on that package's `changelog` key.
Packages are an array of tables (`[[package]]`), so use an **inline
table** to keep it scoped to the right entry:

```toml
[[package]]
name = "frontend"
path = "./apps/web"
release_type = "node"
changelog = { include_author = true, named_parsers = { ci = { skip = true } } }
```

A package `changelog` **merges field-by-field** with `[global.changelog]`:
any field you set on the package wins, and any field you omit is inherited
from your global config (falling back to the built-in default). Global and
package `custom_parsers` are combined, and `named_parsers` overrides apply
per group — so the example above turns on `include_author` and skips `ci`
for `frontend` while still inheriting every other global setting and
default. See
[Per-package changelog](./configuration-reference.md#per-package-changelog)
in the reference for the exact precedence rules.

## The `body` Template

`body` is a [Tera](https://keats.github.io/tera/) template rendered once
per release. The default groups commits by type, links each commit, and
highlights breaking changes:

```toml
[global.changelog]
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
[global.changelog]
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
