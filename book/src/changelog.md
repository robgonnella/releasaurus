# Changelog Customization

Releasaurus generates changelogs from conventional commits. Control what
appears with the filtering options, and how it's formatted with a Tera
template — both in the `[changelog]` section of `releasaurus.toml`.

## Commit Groups & Filtering

Each commit is matched against a set of **parsers**. A parser decides
which `group` (changelog heading) a commit belongs to, and whether the
commit is skipped entirely. Configure them in `[changelog]`.

A parser has three fields:

| Field     | Type   | Effect                                                                                        |
| --------- | ------ | --------------------------------------------------------------------------------------------- |
| `pattern` | regex  | Matched against the raw commit message to decide if the parser applies                        |
| `title`   | string | The changelog heading commits in this group appear under                                      |
| `skip`    | bool   | When `true`, matching commits are dropped from **both** the changelog and version calculation |

### Built-in groups (`default_parsers`)

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
`[changelog.default_parsers]`; everything you omit falls back to the
built-in default. For example, to drop CI and chore commits — the only
change needed is `skip`:

```toml
[changelog.default_parsers]
ci.skip = true
chore.skip = true
```

This is the modern replacement for the old `skip_ci`, `skip_chore`,
`skip_doc`, `skip_test`, `skip_style`, `skip_refactor`, `skip_perf`,
`skip_revert`, and `skip_miscellaneous` flags. To skip a group, set its
`skip = true`. You can also retitle a group or change its matching
pattern the same way:

```toml
[changelog.default_parsers]
feature.title = "<!-- 01 -->✨ New Stuff"
```

### Custom groups (`custom_parsers`)

Define entirely new groups with `[[changelog.custom_parsers]]`. Each
custom parser is checked **before** the built-in parsers, so it takes
precedence over the defaults:

```toml
[[changelog.custom_parsers]]
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

### Dropping or rewriting individual commits

`skip_shas` removes specific commits by SHA prefix (use 7+ characters).
Handy for commits that shouldn't affect versioning or appear in the
changelog:

```toml
[changelog]
skip_shas = ["abc123d", "def456e"]
```

`reword` rewrites a commit's message in the changelog. The new message
affects **both** the changelog text **and** the version bump — changing
`fix:` to `feat:`, for example, bumps minor instead of patch:

```toml
[[changelog.reword]]
sha = "abc123d"
message = "feat: added user authentication"
```

Both have CLI equivalents for one-off runs: `--skip-sha <sha>` and
`--reword <sha>=<message>`. The
[Configuration Reference](./configuration-reference.md#changelog) lists
these options again in terse lookup form.

## The `body` Template

`body` is a [Tera](https://keats.github.io/tera/) template rendered once
per release. The default groups commits by type, links each commit, and
highlights breaking changes:

```toml
[changelog]
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
[changelog]
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
