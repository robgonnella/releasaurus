# [0.3.0](https://github.com/robgonnella/releasaurus/releases/tag/v0.3.0) - 2025-09-18

### 🚀 Features

- adds support for workspace_root config option (#96) [_(f44ede18)_](https://github.com/robgonnella/releasaurus/commit/f44ede18d2f3dbebdc028f4be57f2aac217d1c6d) (Rob Gonnella)

- adds prerelease feature (#93) [_(52c8c489)_](https://github.com/robgonnella/releasaurus/commit/52c8c489b61e501d8d38571920e6e8499a787358) (Rob Gonnella)

- implements separate_pull_requests feature (#92) [_(891cb4e2)_](https://github.com/robgonnella/releasaurus/commit/891cb4e2722531d16daa6ddfd682eff806a92b98) (Rob Gonnella)

- add support for skipping some groups and including author (#90) [_(553c2157)_](https://github.com/robgonnella/releasaurus/commit/553c215787b2af66b0d80cd4b45533fb6a380a2c) (Rob Gonnella)

- makes author name and email available in tera template [_(ea532fb9)_](https://github.com/robgonnella/releasaurus/commit/ea532fb9cab140d91a28d40a48817d61fb33e222) (Rob Gonnella)

### 🐛 Bug Fixes

- errors if release-pr is run before previous release has been tagged (#94) [_(9533e136)_](https://github.com/robgonnella/releasaurus/commit/9533e1368a63df979c5f9543ecbcf63904b3ad5b) (Rob Gonnella)

- fixes issue in release command [_(d49b8934)_](https://github.com/robgonnella/releasaurus/commit/d49b89348587bf66262625a6b0ff45e3364b66ab) (Rob Gonnella)

- fixes issue in gitlab forge [_(8f6f9bf7)_](https://github.com/robgonnella/releasaurus/commit/8f6f9bf7eff23efdab8d2497a449e0f8a33641d1) (Rob Gonnella)

- fixes issue in release command [_(7d39e136)_](https://github.com/robgonnella/releasaurus/commit/7d39e13625584fd78a91131b57a90a4616aad297) (Rob Gonnella)

- improves commit author display [_(5a8bcb2f)_](https://github.com/robgonnella/releasaurus/commit/5a8bcb2fdaac533a2c9c5f14e46698e487b5f24b) (Rob Gonnella)

- another fix for release_type configuration [_(5956632d)_](https://github.com/robgonnella/releasaurus/commit/5956632d418d62b6b66705226b37ac0cfd10f58b) (Rob Gonnella)

- adds release_type and more logging [_(dd4fffe5)_](https://github.com/robgonnella/releasaurus/commit/dd4fffe50eaaa3fe18583e927c1775766b84de9c) (Rob Gonnella)

- fixes issue with generating changelog [_(3943d7ce)_](https://github.com/robgonnella/releasaurus/commit/3943d7ce75a9808b79966d278195cb26b7375086) (Rob Gonnella)

- re-implements ruby updater [_(5d9b29c5)_](https://github.com/robgonnella/releasaurus/commit/5d9b29c5cda6a72670f796f1fb1a3c8cd60d40d5) (Rob Gonnella)

- fixes issue with processing tag_prefix [_(12aa60c3)_](https://github.com/robgonnella/releasaurus/commit/12aa60c307e79226a05eb35470c07bcc1a1e2004) (Rob Gonnella)

### 🚜 Refactor

- updates commands to take mockable params [_(a9acf4a7)_](https://github.com/robgonnella/releasaurus/commit/a9acf4a7d6d0c3f181ecc35dea33220326b10115) (Rob Gonnella)

- moves commit_search_depth to config [_(7a0e09a8)_](https://github.com/robgonnella/releasaurus/commit/7a0e09a88a0fe811c4ce848a0d0f9534ddb4fed0) (Rob Gonnella)

- implements tag_commit method for each forge [_(6e09e372)_](https://github.com/robgonnella/releasaurus/commit/6e09e372d82349c827ec3e29001630ddf53dbbaf) (Rob Gonnella)

- implements updaters in new flow [_(7ac135e9)_](https://github.com/robgonnella/releasaurus/commit/7ac135e973f6eee6a511ff596fb188628e47ebd3) (Rob Gonnella)

- partially implement new flow for gitea [_(accddf5a)_](https://github.com/robgonnella/releasaurus/commit/accddf5aeba13c99d93dc93712f3898ccb1614d6) (Rob Gonnella)

- partial implementation of new forge flow [_(21330189)_](https://github.com/robgonnella/releasaurus/commit/213301896fddb2e447ce54b51cedd73e64dd1ca9) (Rob Gonnella)

- stub out trait method and refactor types [_(82aec90f)_](https://github.com/robgonnella/releasaurus/commit/82aec90fbecdb08c3430cb5454e0049e6fe90b93) (Rob Gonnella)

- gets latest tag directly from forge (#87) [_(2b9b0ff4)_](https://github.com/robgonnella/releasaurus/commit/2b9b0ff413338c702549630b922d96c9453ca3e0) (Rob Gonnella)

### 📚 Documentation

- updates all documentation (#89) [_(ba48d6ac)_](https://github.com/robgonnella/releasaurus/commit/ba48d6acf80d37161748b289ef72e885a950387f) (Rob Gonnella)

### 🧪 Testing

- implements integration / e2e tests for each forge [_(93918cca)_](https://github.com/robgonnella/releasaurus/commit/93918cca26af64985ec2af3c7a87b70440e7c66f) (Rob Gonnella)

- creates common test_helpers module [_(e2bc383c)_](https://github.com/robgonnella/releasaurus/commit/e2bc383c649701b739713708df989bf86746e092) (Rob Gonnella)

- adds unit tests for src/command/release.rs [_(d08a211c)_](https://github.com/robgonnella/releasaurus/commit/d08a211c1b1e686d947492686550b1c0cafef58e) (Rob Gonnella)

- adds unit tests for src/command/release_pr.rs [_(e1d9e9e8)_](https://github.com/robgonnella/releasaurus/commit/e1d9e9e8883b9f4986b64c0d79b8b098d504e577) (Rob Gonnella)

- adds unit tests for src/analyzer.rs [_(c6364112)_](https://github.com/robgonnella/releasaurus/commit/c63641123ad6053f227fd01c12cc4fb34d5bfe1f) (Rob Gonnella)

- adds unit tests for src/updater/manager.rs [_(53e6ba91)_](https://github.com/robgonnella/releasaurus/commit/53e6ba91a06a857fcdc81fdca8d25e1c5ed39aaf) (Rob Gonnella)

- adds manual mock for PackageUpdater trait [_(c342101c)_](https://github.com/robgonnella/releasaurus/commit/c342101c830f7aabb0be3d484119b6ce758997fd) (Rob Gonnella)

- adds back test for rust updater [_(2a7c5c05)_](https://github.com/robgonnella/releasaurus/commit/2a7c5c05085262a9d4465e486ef60ddab69816a0) (Rob Gonnella)

- adds back tests for python updater [_(81388033)_](https://github.com/robgonnella/releasaurus/commit/81388033e811d9336731914c0c823593e01d021f) (Rob Gonnella)

- adds back tests for php updater [_(1ffa7342)_](https://github.com/robgonnella/releasaurus/commit/1ffa7342635b366329ae723ef9ba6c8e061d7520) (Rob Gonnella)

- adds back tests for node updater [_(9ddfd8dd)_](https://github.com/robgonnella/releasaurus/commit/9ddfd8dd4295fda1624267ae8f6c2661c48b34f6) (Rob Gonnella)

- adds back java updater tests [_(94a63648)_](https://github.com/robgonnella/releasaurus/commit/94a636481f2ad2a4d9cfe2d979203c935b0c7359) (Rob Gonnella)

- add mocks for forge traits [_(570ccd23)_](https://github.com/robgonnella/releasaurus/commit/570ccd23c0373b23c6a600a5656d05fb37811335) (Rob Gonnella)

- adds unit tests for src/forge/config.rs [_(57c1c2ae)_](https://github.com/robgonnella/releasaurus/commit/57c1c2aead34cc44bc8f6367ed11b18e5e87210a) (Rob Gonnella)

- adds unit tests for analyzer/helpers.rs [_(f61b8cd9)_](https://github.com/robgonnella/releasaurus/commit/f61b8cd9f1eab912e98b6668be284d02e210082e) (Rob Gonnella)

- adds unit tests for analyzer/commit.rs [_(81776ad4)_](https://github.com/robgonnella/releasaurus/commit/81776ad42d3c121164a8451c5fc7b59bbf527837) (Rob Gonnella)

# [0.2.3](https://github.com/robgonnella/releasaurus/releases/tag/v0.2.3) - 2025-09-17

### 🐛 Bug Fixes

- handles headers / footers and note parsing more intelligently (#80) [_(dc33475a)_](https://github.com/robgonnella/releasaurus/commit/dc33475a43f2ff079643f53c42129a1136073406)

- cleanup extra spaces in changelog [_(cf5bba54)_](https://github.com/robgonnella/releasaurus/commit/cf5bba546ad36da06d5b80d40712f6e879e59357)

- strip extra lines when writing changelog [_(e9a672d1)_](https://github.com/robgonnella/releasaurus/commit/e9a672d1511c41d355c1ebb8d539808b10da701a)

- another fix for stripping extra space in changelog [_(293e5a45)_](https://github.com/robgonnella/releasaurus/commit/293e5a4526bd4ed8477c4e17abb4b80990ef2ccc)

- fixes issues in analyzer (#84) [_(666a1224)_](https://github.com/robgonnella/releasaurus/commit/666a12241401da5887a5a7c8139909356d834d84)

- fixes ordering of groups in tera output (#85) [_(5d68ecca)_](https://github.com/robgonnella/releasaurus/commit/5d68eccad45aab26b3baaf167db9bae4a80547bf)

### 🚜 Refactor

- minor refactor in analyzer [_(39c6d452)_](https://github.com/robgonnella/releasaurus/commit/39c6d45267d023cea445ba31cd4f84b27e7479a2)

- removes dependency on git-cliff-core (#83) [_(84d36e8a)_](https://github.com/robgonnella/releasaurus/commit/84d36e8a0ddca324181b3ccc7aa4452240bbb2c5)

### ⚙️ Miscellaneous Tasks

- Revert "fix: handles headers / footers and note parsing more intelligently (#80)" [_(faeee380)_](https://github.com/robgonnella/releasaurus/commit/faeee380903dc91f8e11cd3d311144f965f1d500)

# [0.2.2](https://github.com/robgonnella/releasaurus/releases/tag/v0.2.2) - 2025-09-11

### 🐛 Bug Fixes

- Fixes issue with Dockerfile and updates docs [_(28c2c797)_](https://github.com/robgonnella/releasaurus/commit/28c2c7971f4552ae27d87f816202dade950eed2f)

# [0.2.1](https://github.com/robgonnella/releasaurus/releases/tag/v0.2.1) - 2025-09-10

### 🐛 Bug Fixes

- Adds missing dependencies to docker build [_(86310337)_](https://github.com/robgonnella/releasaurus/commit/86310337aee8df9b65d41658e2c15b7e4ce8b73c)

# [0.2.0](https://github.com/robgonnella/releasaurus/releases/tag/v0.2.0) - 2025-09-10

### 🚀 Features

- Adds option to configure clone depth (#74) [_(5c86f065)_](https://github.com/robgonnella/releasaurus/commit/5c86f06594e065fbbeb77efe22c69ab29c9d8c16)

### 🐛 Bug Fixes

- Skip chore and ci commits [_(61fa42b1)_](https://github.com/robgonnella/releasaurus/commit/61fa42b13b507901ea91ca651fe9456ead1def68)

# [0.1.1](https://github.com/robgonnella/releasaurus/releases/tag/v0.1.1) - 2025-09-10

### 🐛 Bug Fixes

- Adds repo url to mdbook [_(e5d45848)_](https://github.com/robgonnella/releasaurus/commit/e5d458487b571bc4821fc919d396266a7b49434f)

- Update homepage in Cargo.toml [_(9fb67c3b)_](https://github.com/robgonnella/releasaurus/commit/9fb67c3b0b344aff21969d34f7c909c10633f713)

- Fixes docker publish job [_(dacdc151)_](https://github.com/robgonnella/releasaurus/commit/dacdc15113ad14b540ce74334b769b917cd7ba63)

# [0.1.0](https://github.com/robgonnella/releasaurus/releases/tag/v0.1.0) - 2025-09-09

### 🚀 Features

- Initial release

<!--releasaurus_footer_start-->
---
Generated by Releasaurus 🦕
<!--releasaurus_footer_end-->