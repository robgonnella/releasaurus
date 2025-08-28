# [0.1.0](https://github.com/robgonnella/releasaurus/releases/tag/v0.1.0) - 2025-08-28

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

- _(core)_ Adds config setup to core [_(c5afcce5)_](https://github.com/robgonnella/releasaurus/commit/c5afcce5ea08b7ec85f3c476b1bb3da3a8899b6f)

- _(core)_ Adds git-cliff changelog implementation [_(03db4373)_](https://github.com/robgonnella/releasaurus/commit/03db43738f63122230cb24a9cadfcb083c61d9c9)

- _(core)_ Adds support for setting gitlab remote [_(3ee68923)_](https://github.com/robgonnella/releasaurus/commit/3ee689235aa6cdb9566f04589178f97ec0ec0b66)

- _(core)_ Adds support for other remotes [_(526fc08d)_](https://github.com/robgonnella/releasaurus/commit/526fc08d5f1c10a9fe8be0e884bc39d7551a713d)

- _(core)_ Add ability to write changelog to file [_(9c103614)_](https://github.com/robgonnella/releasaurus/commit/9c103614e36fe52b8f08d4dd21ad48b9f3e77b7c)

- _(cli)_ Adds support for setting gitlab remote [_(7ccd308f)_](https://github.com/robgonnella/releasaurus/commit/7ccd308fcdb3fa85c1b08de3a6023f767eaf8607)

- _(cli)_ Adds initial cli implementation [_(6578d79f)_](https://github.com/robgonnella/releasaurus/commit/6578d79fbe8af7b48e101e4a4e5183c6c95ec8f6)

- _(cli)_ Searches for config file in parent dirs [_(3b4bf290)_](https://github.com/robgonnella/releasaurus/commit/3b4bf290f553c4ea1a3f8e6af11239e60e1b486b)

- _(core)_ Adds commit links to changelog [_(89134509)_](https://github.com/robgonnella/releasaurus/commit/891345091630605d6cfee3b7e648b521ae4c69d3)

- _(core)_ Adds version links to changelog [_(ef992192)_](https://github.com/robgonnella/releasaurus/commit/ef9921928d9bfa00cc09a505067e36e2450f55df)

- Improves links in generated changelog [_(53adf850)_](https://github.com/robgonnella/releasaurus/commit/53adf850a981a998bc200bcc985b363faf012e9d)

- _(core)_ Adds initial implementation of github forge [_(e17de037)_](https://github.com/robgonnella/releasaurus/commit/e17de03764edb05f585ee9a60ac4e4015ff0b15a)

- _(core)_ Adds initial implementation of gitlab forge [_(4b4f125e)_](https://github.com/robgonnella/releasaurus/commit/4b4f125ed17cfed558325122d63c4e1943dad4bc)

- _(core)_ Adds initial gitea forge implementation [_(553fb082)_](https://github.com/robgonnella/releasaurus/commit/553fb082b8b7e7a319480bb8f07d87de20b0ca6f)

- Adds initial implementation for local git client [_(a751cf8d)_](https://github.com/robgonnella/releasaurus/commit/a751cf8dcd6f2abe3f2c6c2a727a75b67abfd9fd)

- Updates cli entrypoint main function [_(3ca00cbe)_](https://github.com/robgonnella/releasaurus/commit/3ca00cbe2ed19967558847f36764a0b07eba837f)

- Implements release-pr command [_(689444c4)_](https://github.com/robgonnella/releasaurus/commit/689444c4fdbd01ce4135571a8dbbc8e1d93a3e38)


### 🐛 Bug Fixes

- _(core)_ Fixes issue with setting tag_prefix [_(8865b9d1)_](https://github.com/robgonnella/releasaurus/commit/8865b9d1ed465b045d858effd5b7a039cacbed74)

- _(core)_ Minor update to changelog body tera [_(5649877e)_](https://github.com/robgonnella/releasaurus/commit/5649877ea6fca6973d051eca7c3d2051d578cf42)

- _(core)_ Fixes issue with setting remotes [_(c2302e34)_](https://github.com/robgonnella/releasaurus/commit/c2302e34dea975ecd8b7df57a1a0be9c0ba23ebb)

- _(core)_ Fixes issue with setting api_url [_(2cd48eaa)_](https://github.com/robgonnella/releasaurus/commit/2cd48eaa2dc2d0f294fb98e51fb699f0f7d4b7b7)

- Fixes gitea forge implementation [_(f427c540)_](https://github.com/robgonnella/releasaurus/commit/f427c540250cc9bffee133de0c2c506f1057b75c)

- _(core)_ Fixes issues in github forge [_(30a26b50)_](https://github.com/robgonnella/releasaurus/commit/30a26b501e274b98a6028bc87a43e7005aed8ba4)

- Sets a clone depth when cloning repository [_(f1a226ba)_](https://github.com/robgonnella/releasaurus/commit/f1a226ba255a7d61cab7cb82ea08f00e18587c97)


### 📚 Documentation

- _(core)_ Adds doc comments to changelog modules [_(ba12df52)_](https://github.com/robgonnella/releasaurus/commit/ba12df529b7a99643775e1e270a7c9dafa05b442)

- _(core)_ Adds doc comments to config [_(9aedadf2)_](https://github.com/robgonnella/releasaurus/commit/9aedadf272efa47421f1f30c42097df6003929e9)


### 🚜 Refactor

- Refactors config and adds beginning of cli [_(ac7e44b0)_](https://github.com/robgonnella/releasaurus/commit/ac7e44b011ffbdb135efb9e8a18338918d3b8fad)

- Remotes remote setup in git-cliff wrapper [_(8f5508f2)_](https://github.com/robgonnella/releasaurus/commit/8f5508f2beb5c2821ea7a0af2c547bf754338b87)

- Allows git local path to be configurable [_(177ce237)_](https://github.com/robgonnella/releasaurus/commit/177ce23701ae0ed5fb8027fdb0d6be21059888a2)

- Removes logic to detect breaking change [_(3b818092)_](https://github.com/robgonnella/releasaurus/commit/3b818092c5383077142a626e2e38342979dcc76a)

- Minor refactor to main executable [_(cb62f69f)_](https://github.com/robgonnella/releasaurus/commit/cb62f69f27ecb5413fa92429d5eb0826e310a537)

- Minor refactoring of release_pr command mod [_(3fe8c68b)_](https://github.com/robgonnella/releasaurus/commit/3fe8c68b59cdb12f84a85b5551a71db29cc552d3)


### 🧪 Testing

- _(core)_ Adds unit tests for git-cliff changelog [_(4dd60272)_](https://github.com/robgonnella/releasaurus/commit/4dd60272d6e453382017c8beb1e87e50db04ce45)


### ⚙️ Miscellaneous Tasks

- _(main)_ Initial commit [_(b3fe8e60)_](https://github.com/robgonnella/releasaurus/commit/b3fe8e60e1be6359624e082e1bd9e525b767ebb8)

- _(main)_ Adds releasaurus.toml [_(6851f467)_](https://github.com/robgonnella/releasaurus/commit/6851f46748721e980d79f8948216915c266da9b4)

- Adds mise.toml [_(090a4947)_](https://github.com/robgonnella/releasaurus/commit/090a4947728d44eae40a54475fd980c79b4795a1)

- _(core)_ Use serde to rename packages field in config [_(e12ed8b7)_](https://github.com/robgonnella/releasaurus/commit/e12ed8b7dee79acfa0820e6b359dac20d1cdce73)

- _(core)_ More improvements to formatting [_(6e83cec6)_](https://github.com/robgonnella/releasaurus/commit/6e83cec661cdc3ec217b32ce9551c442c27e9fa3)

- Moves remote config to top level in core [_(67f56572)_](https://github.com/robgonnella/releasaurus/commit/67f56572739059c91b7d6960d7499b1127313ba6)

- Change remote base_url to link_base_url [_(3a0a804d)_](https://github.com/robgonnella/releasaurus/commit/3a0a804d14e9c2c1aa8a92134b996b4e6880b31a)

- _(core)_ Refactor use of config in forges [_(2a031e51)_](https://github.com/robgonnella/releasaurus/commit/2a031e511087b680d7379d7d5c2fb6658a552f10)

- Runs tests in pipeline [_(8eaf2c8c)_](https://github.com/robgonnella/releasaurus/commit/8eaf2c8cbc89f6257c76e793754abbdc1c5d2c9e)

- Set resource group on integration tests job [_(50d5270d)_](https://github.com/robgonnella/releasaurus/commit/50d5270d8765206e12687bc93813bd049f18955a)

- Combines everything into one bin crate [_(b65801e5)_](https://github.com/robgonnella/releasaurus/commit/b65801e51ee3dd8b2de45e406f21efdc81fe053b)

- Move release_pr command to mod directory [_(4e5a90b6)_](https://github.com/robgonnella/releasaurus/commit/4e5a90b6f2f91e97b1dc06648fef4f59b4909bff)


