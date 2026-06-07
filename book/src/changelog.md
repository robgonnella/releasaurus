# Changelog Customization

Releasaurus generates changelogs from conventional commits. Control what
appears with the filtering options, and how it's formatted with a Tera
template — both in the `[changelog]` section of `releasaurus.toml`.

## Filtering Commits

Each `skip_*` flag drops a category of commit from the changelog; the
remaining flags adjust what's shown. Set them in `[changelog]`:

```toml
[changelog]
skip_ci = true
skip_chore = true
skip_miscellaneous = true
include_author = true

[[package]]
path = "."
release_type = "node"
```

| Option | Default | Effect |
| ------ | ------- | ------ |
| `skip_ci` | `false` | Excludes `ci:` commits (e.g. *ci: update workflow*) |
| `skip_chore` | `false` | Excludes `chore:` commits (e.g. *chore: update deps*) |
| `skip_doc` | `false` | Excludes `docs:` commits |
| `skip_test` | `false` | Excludes `test:` commits |
| `skip_style` | `false` | Excludes `style:` commits |
| `skip_refactor` | `false` | Excludes `refactor:` commits |
| `skip_perf` | `false` | Excludes `perf:` commits |
| `skip_revert` | `false` | Excludes `revert:` commits |
| `skip_miscellaneous` | `false` | Excludes non-conventional commits (no recognized type prefix) |
| `skip_merge_commits` | `true` | Excludes merge commits |
| `include_author` | `false` | Adds the commit author's name to each entry |
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

| Variable | Description |
| -------- | ----------- |
| `version` | Semantic version (e.g. `1.2.3`) |
| `tag_name` | Full tag including prefix/suffix |
| `link` | URL to the release |
| `tag_compare_link` | Diff vs. previous tag (empty for first release) |
| `sha_compare_link` | Diff vs. previous tag, by commit SHA (empty for first release) |
| `sha` | Release commit SHA |
| `timestamp` | Unix timestamp |
| `include_author` | Whether author display is enabled |

### Commit (each item in `commits`)

| Variable | Description |
| -------- | ----------- |
| `id` / `short_id` | Full / abbreviated SHA |
| `group` | Category (Features, Bug Fixes, …) |
| `scope` | Optional conventional-commit scope |
| `title` | Message without type/scope |
| `body` | Optional extended description |
| `link` | URL to the commit |
| `breaking` / `breaking_description` | Breaking-change flag and details |
| `merge_commit` | Whether it's a merge commit |
| `timestamp` | Commit timestamp |
| `author_name` / `author_email` | Commit author |
| `raw_title` / `raw_message` | Original unprocessed title / message |

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
