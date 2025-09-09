# [0.3.0](https://github.com/robgonnella/releasaurus/releases/tag/v0.3.0) - 2025-09-09

### 🚀 Features

- Adds stubs for updater framework (#41) [_(62b3db7e)_](https://github.com/robgonnella/releasaurus/commit/62b3db7ea8c07ccb8401255720f55f7285644028)

- Implements rust updater (#44) [_(7ee38f2a)_](https://github.com/robgonnella/releasaurus/commit/7ee38f2a57d1578f9774217af3509ae35bdfe72e)

- Implements rust Cargo.lock file updates (#46) [_(f336d0bc)_](https://github.com/robgonnella/releasaurus/commit/f336d0bc6109ccb321f3ed4573324b0526da2a79)

- Adds initial implementation for node updater (#47) [_(11f0996a)_](https://github.com/robgonnella/releasaurus/commit/11f0996a9ea68625ebb8c1d79c54d0c64d7f2011)

- Fully implements node updater (#48) [_(0da7b558)_](https://github.com/robgonnella/releasaurus/commit/0da7b5581950c7691c61ad469bc06068d7a5f387)

- Adss initial implementation for python updater [_(e1880e22)_](https://github.com/robgonnella/releasaurus/commit/e1880e22b52bc91e93785216cb4a682d5126e3ff)

- Adds support for setup.cfg in python updater (#53) [_(558a8c77)_](https://github.com/robgonnella/releasaurus/commit/558a8c77c794ff7f2b76e756ad186486a377ddd8)

- Adds setup.py updater implementation (#55) [_(5e6f5cc5)_](https://github.com/robgonnella/releasaurus/commit/5e6f5cc5d75d08b2ee9269464dff0d805163e93d)

- Adds initial php detector (#57) [_(bf769971)_](https://github.com/robgonnella/releasaurus/commit/bf769971167e142a5b54b34c3a332f1fcb06d1fe)

- Adds basic java detector [_(c619c7c6)_](https://github.com/robgonnella/releasaurus/commit/c619c7c63f912893ff8508f75ab3ef7d88d094e2)

- Implements java updater (#60) [_(eafe66fa)_](https://github.com/robgonnella/releasaurus/commit/eafe66fac86f34cda98b4d84641fa5a3b8b1ba59)

- Implements ruby detector (#61) [_(df95a893)_](https://github.com/robgonnella/releasaurus/commit/df95a893ddea8fe308eff17b5878bede3ab078cb)

- Implements ruby updater (#62) [_(7fcade17)_](https://github.com/robgonnella/releasaurus/commit/7fcade17070b50a99b53b7f2c094bcf28f499419)


### 💼 Other

- Implement php updater (#58)

* feat: implements php updater

* feat: preserve formatting in node updater [_(7914122e)_](https://github.com/robgonnella/releasaurus/commit/7914122ef493ea4b288e4e819620d788b9e3f132)


### 🐛 Bug Fixes

- Fixes issue with package path in updaters [_(e5fc606f)_](https://github.com/robgonnella/releasaurus/commit/e5fc606fb7704a0bc00958eecbc3ad2969dc05a1)

- Fixes issue in processing repository paths (#54) [_(1dff5a38)_](https://github.com/robgonnella/releasaurus/commit/1dff5a38bbec2c552ea9b3914eba096bf734786e)

- Handles workspace links in node monorepos (#59) [_(b396bea7)_](https://github.com/robgonnella/releasaurus/commit/b396bea77243b2b8987b2734459e3408a7cb36a2)

- Fixes issues with header and footer not working (#64) [_(4cbacb8c)_](https://github.com/robgonnella/releasaurus/commit/4cbacb8cc38d78e4486a1b72edf5b624a41cb325)


### 📚 Documentation

- Adds documentation as mdbook (#65) [_(0447a4d7)_](https://github.com/robgonnella/releasaurus/commit/0447a4d704b5a254cac9bc1759bff1edf8b519a3)


### 🚜 Refactor

- Simplify updater framework (#42) [_(62dac299)_](https://github.com/robgonnella/releasaurus/commit/62dac2992e5bfaea241327566c97298f434c9943)

- Minor refactor to rust updater [_(a59effa4)_](https://github.com/robgonnella/releasaurus/commit/a59effa408e2a5b315dab8bb18f4539c83815560)

- Removes the need to switch to repo dir [_(c2762e98)_](https://github.com/robgonnella/releasaurus/commit/c2762e9829b9581a5096baa06bc57544bb1d9232)

- Removes support for lockFileVersion 1 in node updater [_(90f6a59f)_](https://github.com/robgonnella/releasaurus/commit/90f6a59f9cd3b15dd7e9112030ea7a15a89e2368)

- Removes support pnpm-lockfile.yaml updates [_(76ed1dbe)_](https://github.com/robgonnella/releasaurus/commit/76ed1dbe6f1500d847af97c7a11315dca18e348b)


### 🧪 Testing

- Fix e2e tests [_(46981994)_](https://github.com/robgonnella/releasaurus/commit/4698199402f0532c711c9ddd4fc36f414ecb7e6d)

- Adds unit tests for rust updater (#45) [_(64876014)_](https://github.com/robgonnella/releasaurus/commit/6487601416f25fd8e2448c69668bd5f425ecb746)

- Adds test for python setuptools detection [_(c2106ee7)_](https://github.com/robgonnella/releasaurus/commit/c2106ee74f28641d7df68cb9ee489b8f8c1431dc)


### ⚙️ Miscellaneous Tasks

- Show more info when e2e tests fail [_(1806ca13)_](https://github.com/robgonnella/releasaurus/commit/1806ca135ab3a1a51e135bf64753b41be54b3288)

- Minor update to some logging [_(14559f48)_](https://github.com/robgonnella/releasaurus/commit/14559f4819acb284ec64a28bfcef7abe7fdee776)

- Use custom internal Result everywhere (#56) [_(49429ace)_](https://github.com/robgonnella/releasaurus/commit/49429ace50c7f257a5f1ccbaaea29901eb0e9573)

- Adds thorough doc comments (#63) [_(bf6bd583)_](https://github.com/robgonnella/releasaurus/commit/bf6bd5835be374951fdb6e30129754ca40775597)

- Adds licenses and updates mdbook docs [_(e1c7f710)_](https://github.com/robgonnella/releasaurus/commit/e1c7f71051c6bc1e142ace2e64f2f54ba00a8cea)

- Adds workflow to publish docs [_(afd0195e)_](https://github.com/robgonnella/releasaurus/commit/afd0195e78529354b5e793c5661f97f8fe04702d)


# [0.2.0](https://github.com/robgonnella/releasaurus/releases/tag/v0.2.0) - 2025-09-02

### 🚀 Features

- Implements release command [_(ce4a2e54)_](https://github.com/robgonnella/releasaurus/commit/ce4a2e545a2a773aa72d8cc1c80ca25d172b88e2)


### 🐛 Bug Fixes

- Fixes issue in github forge (#36) [_(2dd556c7)_](https://github.com/robgonnella/releasaurus/commit/2dd556c7fdff8c61dc7fc7d08daef57f7e382aab)

- Fixes issue in changelog generation (#39) [_(3d07da79)_](https://github.com/robgonnella/releasaurus/commit/3d07da79678e29c2b5efe3587d93e9d22765cf30)

- Another fix for changelog generation [_(56d5b614)_](https://github.com/robgonnella/releasaurus/commit/56d5b614c7410e5e45df6d8b8e1a6ed39b835245)

- Fixes trailing release in changelog (#40) [_(60843806)_](https://github.com/robgonnella/releasaurus/commit/60843806388672c04817f44b05393da1a5bba7f0)


### ⚙️ Miscellaneous Tasks

- Fixes test-all job [_(06b476a1)_](https://github.com/robgonnella/releasaurus/commit/06b476a1b08e848a73233904ef9059d527889130)

- Fixes e2e tests [_(66b7e376)_](https://github.com/robgonnella/releasaurus/commit/66b7e3766fb2f6fd664797a66b4142c95d3ddd70)

- Renames processor module to analyzer [_(3eb0978d)_](https://github.com/robgonnella/releasaurus/commit/3eb0978dc6957220f1dc26527262ade44ae2239d)

- Remove log line [_(d0e623ab)_](https://github.com/robgonnella/releasaurus/commit/d0e623ab1c368322c7ae4e0e8e89dc83ef1ccb75)

- Update deps [_(ce19946e)_](https://github.com/robgonnella/releasaurus/commit/ce19946e21a4c13489b642f7437c1cf8fc08f839)

# [0.1.0](https://github.com/robgonnella/releasaurus/releases/tag/v0.1.0) - 2025-08-30

### ❌ Breaking Changes

[**breaking**]: Improves monorepo setup [_(a9e4a00c)_](https://github.com/robgonnella/releasaurus/commit/a9e4a00c8f5dd52e74e78240aa7dc538834a8027)
> Uses an iterator setup on Config allowing users to simply loop the
config into SinglePackageConfigs. This is more intuitive than generating
each SinglePackageConfig by index directly.
> Toml config property changed from packages to package
as it's more idiomatic. In toml it's common to represent a list item as
the singular form encased in double brackets i.e. `[[package]]`

[**breaking**]: Refactors changelog to return output struct [_(cf932cf7)_](https://github.com/robgonnella/releasaurus/commit/cf932cf74eb63557924cc8d4f7e807a1d2ff52cf)
> Instead of separate traits, this approach is more streamlined by
returning an output struct that includes current_version, next_version,
and is_breaking as fields.
> Removes CurrentVersion and NextVersion traits and
modifies Generator and Writer traits to instead return an Output struct
which includes current_version, next_version, and is_breaking as fields

[**breaking**]: Removes support for bitbucket [_(c4352932)_](https://github.com/robgonnella/releasaurus/commit/c43529320250da2e2a08152965818a581d4726d2)
> Removes support for bitbucket remote. We will decide
later if we want to include support for this forge


### 🚀 Features

- _(core)_ Adds git-cliff changelog implementation [_(03db4373)_](https://github.com/robgonnella/releasaurus/commit/03db43738f63122230cb24a9cadfcb083c61d9c9)

- _(core)_ Adds config setup to core [_(c5afcce5)_](https://github.com/robgonnella/releasaurus/commit/c5afcce5ea08b7ec85f3c476b1bb3da3a8899b6f)

- _(core)_ Adds support for setting gitlab remote [_(3ee68923)_](https://github.com/robgonnella/releasaurus/commit/3ee689235aa6cdb9566f04589178f97ec0ec0b66)

- _(cli)_ Adds initial cli implementation [_(6578d79f)_](https://github.com/robgonnella/releasaurus/commit/6578d79fbe8af7b48e101e4a4e5183c6c95ec8f6)

- _(cli)_ Adds support for setting gitlab remote [_(7ccd308f)_](https://github.com/robgonnella/releasaurus/commit/7ccd308fcdb3fa85c1b08de3a6023f767eaf8607)

- _(core)_ Adds support for other remotes [_(526fc08d)_](https://github.com/robgonnella/releasaurus/commit/526fc08d5f1c10a9fe8be0e884bc39d7551a713d)

- _(core)_ Add ability to write changelog to file [_(9c103614)_](https://github.com/robgonnella/releasaurus/commit/9c103614e36fe52b8f08d4dd21ad48b9f3e77b7c)

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

- Improves commit processing [_(38bd6aa6)_](https://github.com/robgonnella/releasaurus/commit/38bd6aa61045ef7cff3f6cd15c8c436bb14c14f2)

- Adds back the 250 commit limit when cloning [_(441ee801)_](https://github.com/robgonnella/releasaurus/commit/441ee80145cf7583160697b061443b114c2d5915)


### 🐛 Bug Fixes

- _(core)_ Fixes issue with setting tag_prefix [_(8865b9d1)_](https://github.com/robgonnella/releasaurus/commit/8865b9d1ed465b045d858effd5b7a039cacbed74)

- _(core)_ Minor update to changelog body tera [_(5649877e)_](https://github.com/robgonnella/releasaurus/commit/5649877ea6fca6973d051eca7c3d2051d578cf42)

- _(core)_ Fixes issue with setting remotes [_(c2302e34)_](https://github.com/robgonnella/releasaurus/commit/c2302e34dea975ecd8b7df57a1a0be9c0ba23ebb)

- _(core)_ Fixes issue with setting api_url [_(2cd48eaa)_](https://github.com/robgonnella/releasaurus/commit/2cd48eaa2dc2d0f294fb98e51fb699f0f7d4b7b7)

- Fixes gitea forge implementation [_(f427c540)_](https://github.com/robgonnella/releasaurus/commit/f427c540250cc9bffee133de0c2c506f1057b75c)

- _(core)_ Fixes issues in github forge [_(30a26b50)_](https://github.com/robgonnella/releasaurus/commit/30a26b501e274b98a6028bc87a43e7005aed8ba4)

- Sets a clone depth when cloning repository [_(f1a226ba)_](https://github.com/robgonnella/releasaurus/commit/f1a226ba255a7d61cab7cb82ea08f00e18587c97)

- Fixes issues with repo paths and gitlab requests [_(cac6c1d7)_](https://github.com/robgonnella/releasaurus/commit/cac6c1d79eabc8b2071cee72761c12b2263f9ce8)

- Fixes integration tests [_(9c942dcd)_](https://github.com/robgonnella/releasaurus/commit/9c942dcd960d4e4144aa914f7167aeb299c6270b)


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

- More refactoring [_(6adf5e20)_](https://github.com/robgonnella/releasaurus/commit/6adf5e20364c74cc6a0b3899751f7eeb08178324)

- Return pull request object in forge calls [_(5f46f00c)_](https://github.com/robgonnella/releasaurus/commit/5f46f00c78a814fe45969ccaad29a14780824f89)


### 🧪 Testing

- _(core)_ Adds unit tests for git-cliff changelog [_(4dd60272)_](https://github.com/robgonnella/releasaurus/commit/4dd60272d6e453382017c8beb1e87e50db04ce45)


### ⚙️ Miscellaneous Tasks

- _(main)_ Initial commit [_(b3fe8e60)_](https://github.com/robgonnella/releasaurus/commit/b3fe8e60e1be6359624e082e1bd9e525b767ebb8)

- Adds mise.toml [_(090a4947)_](https://github.com/robgonnella/releasaurus/commit/090a4947728d44eae40a54475fd980c79b4795a1)

- _(main)_ Adds releasaurus.toml [_(6851f467)_](https://github.com/robgonnella/releasaurus/commit/6851f46748721e980d79f8948216915c266da9b4)

- _(core)_ Use serde to rename packages field in config [_(e12ed8b7)_](https://github.com/robgonnella/releasaurus/commit/e12ed8b7dee79acfa0820e6b359dac20d1cdce73)

- _(core)_ More improvements to formatting [_(6e83cec6)_](https://github.com/robgonnella/releasaurus/commit/6e83cec661cdc3ec217b32ce9551c442c27e9fa3)

- Moves remote config to top level in core [_(67f56572)_](https://github.com/robgonnella/releasaurus/commit/67f56572739059c91b7d6960d7499b1127313ba6)

- Change remote base_url to link_base_url [_(3a0a804d)_](https://github.com/robgonnella/releasaurus/commit/3a0a804d14e9c2c1aa8a92134b996b4e6880b31a)

- _(core)_ Refactor use of config in forges [_(2a031e51)_](https://github.com/robgonnella/releasaurus/commit/2a031e511087b680d7379d7d5c2fb6658a552f10)

- Runs tests in pipeline [_(8eaf2c8c)_](https://github.com/robgonnella/releasaurus/commit/8eaf2c8cbc89f6257c76e793754abbdc1c5d2c9e)

- Set resource group on integration tests job [_(50d5270d)_](https://github.com/robgonnella/releasaurus/commit/50d5270d8765206e12687bc93813bd049f18955a)

- Combines everything into one bin crate [_(b65801e5)_](https://github.com/robgonnella/releasaurus/commit/b65801e51ee3dd8b2de45e406f21efdc81fe053b)

- Move release_pr command to mod directory [_(4e5a90b6)_](https://github.com/robgonnella/releasaurus/commit/4e5a90b6f2f91e97b1dc06648fef4f59b4909bff)



---
Generated by Releasaurus 🦕