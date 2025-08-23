# [cli-v0.1.0](https://github.com/robgonnella/releasaurus/releases/tag/cli-v0.1.0) - 2025-08-23

### ❌ Breaking Changes

[**breaking**]: Refactors changelog to return output struct [_(cf932cf7)_](https://github.com/robgonnella/releasaurus/commit/cf932cf74eb63557924cc8d4f7e807a1d2ff52cf)
> Instead of separate traits, this approach is more streamlined by
returning an output struct that includes current_version, next_version,
and is_breaking as fields.
> Removes CurrentVersion and NextVersion traits and
modifies Generator and Writer traits to instead return an Output struct
which includes current_version, next_version, and is_breaking as fields

[**breaking**]: Improves monorepo setup [_(a9e4a00c)_](https://github.com/robgonnella/releasaurus/commit/a9e4a00c8f5dd52e74e78240aa7dc538834a8027)
> Uses an iterator setup on Config allowing users to simply loop the
config into SinglePackageConfigs. This is more intuitive than generating
each SinglePackageConfig by index directly.
> Toml config property changed from packages to package
as it's more idiomatic. In toml it's common to represent a list item as
the singular form encased in double brackets i.e. `[[package]]`

[**breaking**]: Removes support for bitbucket [_(c4352932)_](https://github.com/robgonnella/releasaurus/commit/c43529320250da2e2a08152965818a581d4726d2)
> Removes support for bitbucket remote. We will decide
later if we want to include support for this forge


### 🚀 Features

- _(cli)_ Adds support for setting gitlab remote [_(7ccd308f)_](https://github.com/robgonnella/releasaurus/commit/7ccd308fcdb3fa85c1b08de3a6023f767eaf8607)

- _(cli)_ Adds initial cli implementation [_(6578d79f)_](https://github.com/robgonnella/releasaurus/commit/6578d79fbe8af7b48e101e4a4e5183c6c95ec8f6)

- _(cli)_ Searches for config file in parent dirs [_(3b4bf290)_](https://github.com/robgonnella/releasaurus/commit/3b4bf290f553c4ea1a3f8e6af11239e60e1b486b)

- Improves links in generated changelog [_(53adf850)_](https://github.com/robgonnella/releasaurus/commit/53adf850a981a998bc200bcc985b363faf012e9d)


### 💼 Other

- Wip [_(c69b1bc1)_](https://github.com/robgonnella/releasaurus/commit/c69b1bc1f13e862a13d8766fb1e45b8317bd5363)


### 🚜 Refactor

- Refactors config and adds beginning of cli [_(ac7e44b0)_](https://github.com/robgonnella/releasaurus/commit/ac7e44b011ffbdb135efb9e8a18338918d3b8fad)


### ⚙️ Miscellaneous Tasks

- _(main)_ Initial commit [_(b3fe8e60)_](https://github.com/robgonnella/releasaurus/commit/b3fe8e60e1be6359624e082e1bd9e525b767ebb8)

- _(core)_ More improvements to formatting [_(6e83cec6)_](https://github.com/robgonnella/releasaurus/commit/6e83cec661cdc3ec217b32ce9551c442c27e9fa3)

- Moves remote config to top level in core [_(67f56572)_](https://github.com/robgonnella/releasaurus/commit/67f56572739059c91b7d6960d7499b1127313ba6)

- Change remote base_url to link_base_url [_(3a0a804d)_](https://github.com/robgonnella/releasaurus/commit/3a0a804d14e9c2c1aa8a92134b996b4e6880b31a)


