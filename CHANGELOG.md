# [0.10.2](https://github.com/robgonnella/releasaurus/releases/tag/v0.10.2) - 2026-01-16

### 🐛 Bug Fixes

- fixes issue in tag searches [_(df2f10f)_](https://github.com/robgonnella/releasaurus/commit/df2f10f083b923e2db21bb17e532098ce555ae92) (Rob Gonnella)

# [0.10.1](https://github.com/robgonnella/releasaurus/releases/tag/v0.10.1) - 2026-01-16

### 🐛 Bug Fixes

- unify errors for getting open and merged release PRs [_(2520b6a)_](https://github.com/robgonnella/releasaurus/commit/2520b6aa6c240f76d35ae32e3c3850ca9c169d02) (Rob Gonnella)

- addresses pagination issues in forges [_(b2eb837)_](https://github.com/robgonnella/releasaurus/commit/b2eb8376330360e5b739f04660f285bbb46fed8d) (Rob Gonnella)

# [0.10.0](https://github.com/robgonnella/releasaurus/releases/tag/v0.10.0) - 2026-01-06

### 🚀 Features

- adds command aliases [_(ab958f7)_](https://github.com/robgonnella/releasaurus/commit/ab958f74f1d65e740c7632985423fd9c04a87659) (Rob Gonnella)

- implements show notes feature [_(eed040c)_](https://github.com/robgonnella/releasaurus/commit/eed040c3cfebb4ec577385a20dc62a8160017df0) (Rob Gonnella)

- implements sub-package feature [_(eb3c8aa)_](https://github.com/robgonnella/releasaurus/commit/eb3c8aaa44e026f3f5d24cc023325e3fc836de4b) (Rob Gonnella)

### 🐛 Bug Fixes

- another fix for restricting show next-release to target package [_(80375b8)_](https://github.com/robgonnella/releasaurus/commit/80375b8193f67c4e4cbbb4f9bca4e37338381001) (Rob Gonnella)

- prevent fetching commits for all when target supplied [_(93986ec)_](https://github.com/robgonnella/releasaurus/commit/93986ec92f0292bb1297cd6bd25b3324ca7df517) (Rob Gonnella)

### 🚜 Refactor

- major refactor of repository structure [_(df3b73c)_](https://github.com/robgonnella/releasaurus/commit/df3b73c66968701141f4c4aaf11631e69c4aeba0) (Rob Gonnella)

# [0.9.0](https://github.com/robgonnella/releasaurus/releases/tag/v0.9.0) - 2025-12-30

### 🚀 Features

- adds "show current-release" command [_(596cdbe)_](https://github.com/robgonnella/releasaurus/commit/596cdbe5c4229dd42ce3086ced1373a87adb571a) (Rob Gonnella)

- simplifies CI actions and components [_(b5efea2)_](https://github.com/robgonnella/releasaurus/commit/b5efea2e2de3feb2e74059b8a77b55ee65b23c99) (Rob Gonnella)

- support custom regex for additional manifests [_(06cd89d)_](https://github.com/robgonnella/releasaurus/commit/06cd89d706f4fd8038069c6d4cb19bd81b7eb217) (Rob Gonnella)

- adds skip_shas and reword features [_(e945142)_](https://github.com/robgonnella/releasaurus/commit/e945142646780d60090c1f6d01482e6bcf5e72af) (Rob Gonnella)

# [0.8.0](https://github.com/robgonnella/releasaurus/releases/tag/v0.8.0) - 2025-12-20

### 🚀 Features

- implements prerelease cli overrides [_(7864e23)_](https://github.com/robgonnella/releasaurus/commit/7864e235127daf72b030ceb047219eca8c6269ab) (Rob Gonnella)

- implements start-next feature [_(b451611)_](https://github.com/robgonnella/releasaurus/commit/b451611f9eb2854728b446e806068c6f4758bbd6) (Rob Gonnella)

- implements "show release" command [_(ecb945a)_](https://github.com/robgonnella/releasaurus/commit/ecb945a221e3389a2e862ee69280919b3bc592cb) (Rob Gonnella)

- implements base_branch feature [_(8876a9c)_](https://github.com/robgonnella/releasaurus/commit/8876a9cdc2e60b435fd3c8b926c1b3e645a5be88) (Rob Gonnella)

### 🐛 Bug Fixes

- fixes issues with ci actions [_(b441fbb)_](https://github.com/robgonnella/releasaurus/commit/b441fbbf19cee52ae10ec638e1968dd170b75107) (Rob Gonnella)

- fixes default group sorting in generated changelog [_(68f8ec7)_](https://github.com/robgonnella/releasaurus/commit/68f8ec77e20ca35ace1a8ce735082b4f52ae912e) (Rob Gonnella)

- fixes bug in start-next command [_(b4a38d7)_](https://github.com/robgonnella/releasaurus/commit/b4a38d7e70c68af8a12a31d93aff9c71eb0f15fa) (Rob Gonnella)

- ensures correct base_branch is used in forge.create_release_branch [_(8974d3f)_](https://github.com/robgonnella/releasaurus/commit/8974d3f3c1c32e1743537fd854a1951bf004be58) (Rob Gonnella)

- fixes issues in base_branch implementation [_(dd950c0)_](https://github.com/robgonnella/releasaurus/commit/dd950c0508a6751abf957dd33e056b6e1ab7f194) (Rob Gonnella)

- addresses issue in gitlab forge `get_merged_release_pr` [_(baa75bd)_](https://github.com/robgonnella/releasaurus/commit/baa75bd5eb68746bbc679659b94c805465702576) (Rob Gonnella)

### 🚜 Refactor

- introduces FileLoader trait [_(434af51)_](https://github.com/robgonnella/releasaurus/commit/434af51ea859e9ad94c7f75838444dbb856f65ba) (Rob Gonnella)

- removes duplication of logs across forges [_(7a45e6e)_](https://github.com/robgonnella/releasaurus/commit/7a45e6eed040c7762d273e5168086875336df890) (Rob Gonnella)

- implements factory pattern for creating forges [_(9d488cd)_](https://github.com/robgonnella/releasaurus/commit/9d488cda2958847b4e2131d4a080f5d655494c6d) (Rob Gonnella)

- uses strategies architecture for next version [_(0be5f51)_](https://github.com/robgonnella/releasaurus/commit/0be5f51b01ba50f0cf32c51e1a8b4a4cf17d27d5) (Rob Gonnella)

- removes unnecessary derives of Clone trait [_(8adcda7)_](https://github.com/robgonnella/releasaurus/commit/8adcda76df1425ebc764e43b60e15aa2b41501e9) (Rob Gonnella)

- improves error handling with dedicated types [_(5005e0e)_](https://github.com/robgonnella/releasaurus/commit/5005e0e0653e9b3bb1b9e6e10b36e9d5f497a652) (Rob Gonnella)

- reduces code duplication in updaters [_(976b2b2)_](https://github.com/robgonnella/releasaurus/commit/976b2b2abf8784f773d3c0a2ac04d8fa8963bcb1) (Rob Gonnella)

### ⚡ Performance

- improves handling of updaters [_(bf531ef)_](https://github.com/robgonnella/releasaurus/commit/bf531ef0b9f957524034554dd00d8f2779446c5a) (Rob Gonnella)

- improves handling of RemoteConfig in forges [_(96b5b34)_](https://github.com/robgonnella/releasaurus/commit/96b5b34d37a3e8fdf46dfba9e2a3f7b5bf1bd0d5) (Rob Gonnella)

- improves string handling in forges [_(3f83aff)_](https://github.com/robgonnella/releasaurus/commit/3f83aff26fd761e8ce056129f4d208b2195244b4) (Rob Gonnella)

- improves performance by reducing cloning [_(b7fc8b9)_](https://github.com/robgonnella/releasaurus/commit/b7fc8b97dd523675d1367b110ec0e7a9520aa757) (Rob Gonnella)

### 📚 Documentation

- improves documentation [_(082a564)_](https://github.com/robgonnella/releasaurus/commit/082a564a81447b1a45d98ae54c21ad68172d0e65) (Rob Gonnella)

### 🧪 Testing

- adds integration tests for forges [_(5bc9812)_](https://github.com/robgonnella/releasaurus/commit/5bc98128f600682d2d7a1d635c066d938820832d) (Rob Gonnella)

# [0.7.1](https://github.com/robgonnella/releasaurus/releases/tag/v0.7.1) - 2025-12-07

### 🐛 Bug Fixes

- updates documentation for projected-release command [_(5c6c272)_](https://github.com/robgonnella/releasaurus/commit/5c6c272a7f1628611b00a0734d6983106c363f36) (Rob Gonnella)

- bump action versions [_(8f38d8f)_](https://github.com/robgonnella/releasaurus/commit/8f38d8f1b54b874095194c59cbadf0355fec4221) (Rob Gonnella)

# [0.7.0](https://github.com/robgonnella/releasaurus/releases/tag/v0.7.0) - 2025-12-07

### 🚀 Features

- adds projected-release command [_(49241bf)_](https://github.com/robgonnella/releasaurus/commit/49241bf348b94d155cd9d4aa4a3dc4c77eccd06b) (Rob Gonnella)

### 🐛 Bug Fixes

- expose job_name input in gitlab parent workflow [_(d5f783a)_](https://github.com/robgonnella/releasaurus/commit/d5f783aec5c98c6555ac07ea94d33d0c2b9e1c35) (Rob Gonnella)

# [0.6.0](https://github.com/robgonnella/releasaurus/releases/tag/v0.6.0) - 2025-12-06

### 🐛 Bug Fixes

- removes unnecessary log lines in gitea forge [_(0a3423d)_](https://github.com/robgonnella/releasaurus/commit/0a3423dd44aab7fcd15059db43d8ffa7dd4f7c14) (Rob Gonnella)

- fixes issue in github forge with finding open release PRs [_(f0bf56c)_](https://github.com/robgonnella/releasaurus/commit/f0bf56c7513659696eedbf01487828d4a41f534c) (Rob Gonnella)

### 🚀 Features

- add ability to specify job stage in gitlab components [_(f7d18f1)_](https://github.com/robgonnella/releasaurus/commit/f7d18f1762ab137b9eb064537c8bfeaf30168e9a) (Rob Gonnella)

# [0.5.2](https://github.com/robgonnella/releasaurus/releases/tag/v0.5.2) - 2025-12-05

### 🐛 Bug Fixes

- prevent logging manifest file content [_(09c69e8)_](https://github.com/robgonnella/releasaurus/commit/09c69e86e8b091fa6492ef94675bfb4c53b2ba15) (Rob Gonnella)

- bump forge client versions [_(98e8efe)_](https://github.com/robgonnella/releasaurus/commit/98e8efe22e09345a9181748e3b5fc69e8fcf1d58) (Rob Gonnella)

# [0.5.1](https://github.com/robgonnella/releasaurus/releases/tag/v0.5.1) - 2025-12-01

### 🐛 Bug Fixes

- fixes issue with additional_manifest_files feature [_(0e3ae29)_](https://api.github.com/repos/robgonnella/releasaurus/commits/0e3ae29c0655da6095176c65f4e6fb47fab94cb3) (Rob Gonnella)

# [0.5.0](https://github.com/robgonnella/releasaurus/releases/tag/v0.5.0) - 2025-11-29

### 🚀 Features

- implements generic manifest version updates [_(49f6472)_](https://api.github.com/repos/robgonnella/releasaurus/commits/49f647237c34d8ab96f6434effa6435146f890ac) (Rob Gonnella)

### 🐛 Bug Fixes

- updates documentation to include binary install instructions [_(130bbd2)_](https://api.github.com/repos/robgonnella/releasaurus/commits/130bbd2f842203cd4b5127e5982171c8be375183) (Rob Gonnella)

# [0.4.13](https://github.com/robgonnella/releasaurus/releases/tag/v0.4.13) - 2025-11-28

### 🐛 Bug Fixes

- fixes cargo binstall metadata [_(a42ad96)_](https://api.github.com/repos/robgonnella/releasaurus/commits/a42ad96451a96a309cbd0bc7a58c9ec4da244660) (Rob Gonnella)

# [0.4.12](https://github.com/robgonnella/releasaurus/releases/tag/v0.4.12) - 2025-11-28

### 🐛 Bug Fixes

- exclude dev-scripts when building and publishing [_(2030c11)_](https://api.github.com/repos/robgonnella/releasaurus/commits/2030c11663c6337c00b4cdf6e0b361aee208e6af) (Rob Gonnella)

# [0.4.11](https://github.com/robgonnella/releasaurus/releases/tag/v0.4.11) - 2025-11-28

### 🐛 Bug Fixes

- fixes issue with uploading artifacts to release [_(a4af60d)_](https://api.github.com/repos/robgonnella/releasaurus/commits/a4af60df671825ac49321d9fcdb298e55624f864) (Rob Gonnella)

# [0.4.10](https://github.com/robgonnella/releasaurus/releases/tag/v0.4.10) - 2025-11-28

### 🐛 Bug Fixes

- another fix for publishing binaries [_(ca6ea74)_](https://api.github.com/repos/robgonnella/releasaurus/commits/ca6ea7428cf98cd9373212c40204f9ecf387fd3b) (Rob Gonnella)

# [0.4.9](https://github.com/robgonnella/releasaurus/releases/tag/v0.4.9) - 2025-11-28

### 🐛 Bug Fixes

- uses cross-platform action for creating tar archives [_(8d60e22)_](https://api.github.com/repos/robgonnella/releasaurus/commits/8d60e221fa0b3c3c45e86f6f341f670dbc73caef) (Rob Gonnella)

# [0.4.8](https://github.com/robgonnella/releasaurus/releases/tag/v0.4.8) - 2025-11-28

### 🐛 Bug Fixes

- fixes use of matrix var in packaging job [_(eff6c19)_](https://api.github.com/repos/robgonnella/releasaurus/commits/eff6c19ddcea89866554db6e480fbf936926428f) (Rob Gonnella)

- fixes issue with publishing binaries [_(1d14e5d)_](https://api.github.com/repos/robgonnella/releasaurus/commits/1d14e5d6b175bd002d47b34c301b72e394c96502) (Rob Gonnella)

# [0.4.7](https://github.com/robgonnella/releasaurus/releases/tag/v0.4.7) - 2025-11-28

### 🐛 Bug Fixes

- improves publishing binaries to release [_(e2b5eb7)_](https://api.github.com/repos/robgonnella/releasaurus/commits/e2b5eb76f40ae4855c107f8b376edc9528cc1936) (Rob Gonnella)

# [0.4.6](https://github.com/robgonnella/releasaurus/releases/tag/v0.4.6) - 2025-11-28

### 🐛 Bug Fixes

- fixes issue with building windows binary [_(ec9f6d2)_](https://api.github.com/repos/robgonnella/releasaurus/commits/ec9f6d285a19c91ee015f879656302e11f41beab) (Rob Gonnella)

- fixes docker build and bumps actions versions [_(34977ea)_](https://api.github.com/repos/robgonnella/releasaurus/commits/34977ea2e74ce5459cce6cd6c490ede0e9eef14d) (Rob Gonnella)

# [0.4.5](https://github.com/robgonnella/releasaurus/releases/tag/v0.4.5) - 2025-11-28

### 🐛 Bug Fixes

- another fix for caching in pipeline [_(a106382)_](https://api.github.com/repos/robgonnella/releasaurus/commits/a10638266db5b1d79d2d15c80eb918c98a323d65) (Rob Gonnella)

- fixes asset names in releases [_(3e2dd32)_](https://api.github.com/repos/robgonnella/releasaurus/commits/3e2dd3262b11eabb8c14e2e81850b1b0319ab1a5) (Rob Gonnella)

- include statically linked openssl for cross compiling [_(76cfa02)_](https://api.github.com/repos/robgonnella/releasaurus/commits/76cfa02a1b0e4f960209874c87e2bf1658f1144c) (Rob Gonnella)

# [0.4.4](https://github.com/robgonnella/releasaurus/releases/tag/v0.4.4) - 2025-11-28

### 🐛 Bug Fixes

- temporarily disable building windows binary [_(98fedff)_](https://api.github.com/repos/robgonnella/releasaurus/commits/98fedff656d25f0d9ace4959da2e47127eb2494c) (Rob Gonnella)

# [0.4.3](https://github.com/robgonnella/releasaurus/releases/tag/v0.4.3) - 2025-11-28

### 🐛 Bug Fixes

- fixes matrix usage in pipeline binaries job [_(e06de41)_](https://api.github.com/repos/robgonnella/releasaurus/commits/e06de4121a1c741a62f4076353051d4efe1837a7) (Rob Gonnella)

# [0.4.2](https://github.com/robgonnella/releasaurus/releases/tag/v0.4.2) - 2025-11-28

### 🐛 Bug Fixes

- another fix for building binaries in pipeline [_(5300607)_](https://api.github.com/repos/robgonnella/releasaurus/commits/5300607c0a5c34bdd077c8cd959c95ad5e536739) (Rob Gonnella)

# [0.4.1](https://github.com/robgonnella/releasaurus/releases/tag/v0.4.1) - 2025-11-28

### 🐛 Bug Fixes

- fixes issue with building and publishing binaries [_(376a1ca)_](https://api.github.com/repos/robgonnella/releasaurus/commits/376a1cafb8a7e2583b57cb32fad5616e852da619) (Rob Gonnella)

# [0.4.0](https://github.com/robgonnella/releasaurus/releases/tag/v0.4.0) - 2025-11-27

### 🚀 Features

- adds script to automate schema generation [_(be63344)_](https://api.github.com/repos/robgonnella/releasaurus/commits/be63344bc5fefdd3ff94a6af93f1bd3f8363e88a) (Rob Gonnella)

### 🐛 Bug Fixes

- improves the gitea get_commits implementation [_(7ef1238)_](https://api.github.com/repos/robgonnella/releasaurus/commits/7ef1238d526ce996d8122558c5d017ed6ae9d4e9) (Rob Gonnella)

- fixes issues with backfilling file list in gitlab forge [_(1ad6679)_](https://api.github.com/repos/robgonnella/releasaurus/commits/1ad6679b7c93a97a23d2e00d4631cfe262ac4c88) (Rob Gonnella)

# [0.3.12](https://github.com/robgonnella/releasaurus/releases/tag/v0.3.12) - 2025-11-26

### 🐛 Bug Fixes

- fixes issue with getting commits in github forge [_(39d6fbf)_](https://github.com/robgonnella/releasaurus/commit/39d6fbf76fbb80c79dd6d8527e46e1917fa925f3) (Rob Gonnella)

# [0.3.11](https://github.com/robgonnella/releasaurus/releases/tag/v0.3.11) - 2025-11-25

### 🐛 Bug Fixes

- another fix for github and gitea actions [_(be2bb83)_](https://github.com/robgonnella/releasaurus/commit/be2bb83dd16e05d4612ea8fe80d2435f834ce0de) (Rob Gonnella)

# [0.3.10](https://github.com/robgonnella/releasaurus/releases/tag/v0.3.10) - 2025-11-25

### 🐛 Bug Fixes

- fixes gitea actions and prunes options [_(e5d1cf9)_](https://github.com/robgonnella/releasaurus/commit/e5d1cf90f28262e40f8e62e032035fbff4b3fb76) (Rob Gonnella)

# [0.3.9](https://github.com/robgonnella/releasaurus/releases/tag/v0.3.9) - 2025-11-25

### 🐛 Bug Fixes

- final attempt to fix action gates [_(12a0560)_](https://github.com/robgonnella/releasaurus/commit/12a05600314bb5b48babe6b57f61b4843ee153e7) (Rob Gonnella)

# [0.3.8](https://github.com/robgonnella/releasaurus/releases/tag/v0.3.8) - 2025-11-25

### 🐛 Bug Fixes

- another attempt at fixing action gates [_(fb8be23)_](https://github.com/robgonnella/releasaurus/commit/fb8be23dc4d24870dc4b8f860540b61b4fc11ea8) (Rob Gonnella)

# [0.3.7](https://github.com/robgonnella/releasaurus/releases/tag/v0.3.7) - 2025-11-25

### 🐛 Bug Fixes

- fixes issue in github / gitea actions [_(ebc4be2)_](https://github.com/robgonnella/releasaurus/commit/ebc4be249b59e1b4446a9ab0e5e54ac727cc7558) (Rob Gonnella)

# [0.3.6](https://github.com/robgonnella/releasaurus/releases/tag/v0.3.6) - 2025-11-24

### 🐛 Bug Fixes

- fixes issue with adding and reading metada to release PRs [_(48b6b42)_](https://github.com/robgonnella/releasaurus/commit/48b6b420b2212b6d05470448abf0762753e16a5c) (Rob Gonnella)

- fixes issues in release command [_(f0d527c)_](https://github.com/robgonnella/releasaurus/commit/f0d527cf8940a27e33a00b31d323b9b8abc38fef) (Rob Gonnella)

# [0.3.5](https://github.com/robgonnella/releasaurus/releases/tag/v0.3.5) - 2025-11-24

### 🐛 Bug Fixes

- fixes debug inputs for github and gitea actions [_(7d100ca)_](https://github.com/robgonnella/releasaurus/commit/7d100ca2b6f54a3fd499836cd9cc5b649b757ceb) (Rob Gonnella)

# [0.3.4](https://github.com/robgonnella/releasaurus/releases/tag/v0.3.4) - 2025-11-24

### 🐛 Bug Fixes

- fixes issue with parsing PR metadata [_(b29ad7c)_](https://github.com/robgonnella/releasaurus/commit/b29ad7c300418596e8fbbf1aa610044d8d4c542f) (Rob Gonnella)

# [0.3.3](https://github.com/robgonnella/releasaurus/releases/tag/v0.3.3) - 2025-11-24

### 🐛 Bug Fixes

- fixes issues with github and gitea actions [_(b2c774e)_](https://github.com/robgonnella/releasaurus/commit/b2c774e0f55d40a01c1bcf988a0684ff680a96fb) (Rob Gonnella)

# [0.3.2](https://github.com/robgonnella/releasaurus/releases/tag/v0.3.2) - 2025-11-24

### 🐛 Bug Fixes

- updates actions to latest versions [_(232aca1)_](https://github.com/robgonnella/releasaurus/commit/232aca116d03e0001e3cf6b14af3c785c4ba6b15) (Rob Gonnella)

# [0.3.1](https://github.com/robgonnella/releasaurus/releases/tag/v0.3.1) - 2025-11-24

### 🐛 Bug Fixes

- fixes issues [_(0fffb0d)_](https://github.com/robgonnella/releasaurus/commit/0fffb0dd652a5ecc37eb2cc890f8e8c114e5c64a) (Rob Gonnella)

# [0.3.0](https://github.com/robgonnella/releasaurus/releases/tag/v0.3.0) - 2025-09-18

### 🐛 Bug Fixes

- fixes issue in local forge [_(f54b0a3)_](https://github.com/robgonnella/releasaurus/commit/f54b0a32d7e3811cb6370b2e9f93266083c9671b) (Rob Gonnella)

- updates schema to add newly added properties [_(ca39fc5)_](https://github.com/robgonnella/releasaurus/commit/ca39fc5bed1e2b49e3fb7423a4b404eb68bf6d99) (Rob Gonnella)

- preserves formatting in ruby and java updaters [_(679d788)_](https://github.com/robgonnella/releasaurus/commit/679d7885ad75f932aed58fe68b3922443c2ae9d1) (Rob Gonnella)

- preserve indentation in python updaters [_(7536361)_](https://github.com/robgonnella/releasaurus/commit/75363619355de5a1804cf51c08316262e127906e) (Rob Gonnella)

- preserve formatting in json files [_(f756315)_](https://github.com/robgonnella/releasaurus/commit/f7563157aa3f5767dc8056e2af880fdbe9ebcd69) (Rob Gonnella)

- fixes issues with package path processing [_(4077adf)_](https://github.com/robgonnella/releasaurus/commit/4077adf37601a69dd12ab6ca3771a2526413c4a2) (Rob Gonnella)

- errors if release-pr is run before previous release has been tagged (#94) [_(9533e13)_](https://github.com/robgonnella/releasaurus/commit/9533e1368a63df979c5f9543ecbcf63904b3ad5b) (Rob Gonnella)

- fixes issue in release command [_(d49b893)_](https://github.com/robgonnella/releasaurus/commit/d49b89348587bf66262625a6b0ff45e3364b66ab) (Rob Gonnella)

- fixes issue in gitlab forge [_(8f6f9bf)_](https://github.com/robgonnella/releasaurus/commit/8f6f9bf7eff23efdab8d2497a449e0f8a33641d1) (Rob Gonnella)

- fixes issue in release command [_(7d39e13)_](https://github.com/robgonnella/releasaurus/commit/7d39e13625584fd78a91131b57a90a4616aad297) (Rob Gonnella)

- improves commit author display [_(5a8bcb2)_](https://github.com/robgonnella/releasaurus/commit/5a8bcb2fdaac533a2c9c5f14e46698e487b5f24b) (Rob Gonnella)

- another fix for release*type configuration [*(5956632)\_](https://github.com/robgonnella/releasaurus/commit/5956632d418d62b6b66705226b37ac0cfd10f58b) (Rob Gonnella)

- adds release*type and more logging [*(dd4fffe)\_](https://github.com/robgonnella/releasaurus/commit/dd4fffe50eaaa3fe18583e927c1775766b84de9c) (Rob Gonnella)

- fixes issue with generating changelog [_(3943d7c)_](https://github.com/robgonnella/releasaurus/commit/3943d7ce75a9808b79966d278195cb26b7375086) (Rob Gonnella)

- re-implements ruby updater [_(5d9b29c)_](https://github.com/robgonnella/releasaurus/commit/5d9b29c5cda6a72670f796f1fb1a3c8cd60d40d5) (Rob Gonnella)

- fixes issue with processing tag*prefix [*(12aa60c)\_](https://github.com/robgonnella/releasaurus/commit/12aa60c307e79226a05eb35470c07bcc1a1e2004) (Rob Gonnella)

### 📚 Documentation

- updates documentation [_(b69d155)_](https://github.com/robgonnella/releasaurus/commit/b69d155e4ff9243e58330ee8b9890d7dcd6a6a7f) (Rob Gonnella)

- updates contributing doc [_(828fb2c)_](https://github.com/robgonnella/releasaurus/commit/828fb2ce2ba219c163708eab536acb489cd008ce) (Rob Gonnella)

- updates documentation for additional*paths feature [*(1a59909)\_](https://github.com/robgonnella/releasaurus/commit/1a5990928be0e5b3507d60edf586d4d079e802e1) (Rob Gonnella)

- updates all documentation (#89) [_(ba48d6a)_](https://github.com/robgonnella/releasaurus/commit/ba48d6acf80d37161748b289ef72e885a950387f) (Rob Gonnella)

### 🚀 Features

- exposes VersionUpdater options in user facing config [_(f3598cc)_](https://github.com/robgonnella/releasaurus/commit/f3598cc75f206e62e1f16770075d3c9505c002f6) (Rob Gonnella)

- adds local repo forge for testing config changes [_(576fd54)_](https://github.com/robgonnella/releasaurus/commit/576fd54c3e4551a01da33468744c74c59e64acf6) (Rob Gonnella)

- implements dry-run option for commands [_(05c5d5b)_](https://github.com/robgonnella/releasaurus/commit/05c5d5bd7f285519d1d1e5b41847f2361f377895) (Rob Gonnella)

- adds github and gitea actions and gitlab-ci components [_(476f40a)_](https://github.com/robgonnella/releasaurus/commit/476f40a17ac69f4ef39ae7d04a92633f7e58dee9) (Rob Gonnella)

- adds new options to changelog config [_(bcd2c7b)_](https://github.com/robgonnella/releasaurus/commit/bcd2c7bcec5095486882a113a44c06b9c4cdda0f) (Rob Gonnella)

- implements additional*paths [*(11ed7ef)\_](https://github.com/robgonnella/releasaurus/commit/11ed7efde2cfc643a3c5d47725abea216d9bd38b) (Rob Gonnella)

- adds support for workspace*root config option (#96) [*(f44ede1)\_](https://github.com/robgonnella/releasaurus/commit/f44ede18d2f3dbebdc028f4be57f2aac217d1c6d) (Rob Gonnella)

- adds prerelease feature (#93) [_(52c8c48)_](https://github.com/robgonnella/releasaurus/commit/52c8c489b61e501d8d38571920e6e8499a787358) (Rob Gonnella)

- implements separate*pull_requests feature (#92) [*(891cb4e)\_](https://github.com/robgonnella/releasaurus/commit/891cb4e2722531d16daa6ddfd682eff806a92b98) (Rob Gonnella)

- add support for skipping some groups and including author (#90) [_(553c215)_](https://github.com/robgonnella/releasaurus/commit/553c215787b2af66b0d80cd4b45533fb6a380a2c) (Rob Gonnella)

- makes author name and email available in tera template [_(ea532fb)_](https://github.com/robgonnella/releasaurus/commit/ea532fb9cab140d91a28d40a48817d61fb33e222) (Rob Gonnella)

### ⏩ CI/CD

- use hard coded versions for github / gitea actions [_(42de0d8)_](https://github.com/robgonnella/releasaurus/commit/42de0d8063c01fd1701caa43783841f8bbb8f3f8) (Rob Gonnella)

- trying env.action*ref instead [*(72cf563)\_](https://github.com/robgonnella/releasaurus/commit/72cf563c598891182decbdccfb8bc3b4aad37da3) (Rob Gonnella)

- try to use env.GITHUB*ACTION_REF in github action [*(2bbbfaf)\_](https://github.com/robgonnella/releasaurus/commit/2bbbfaf7ce4cb108cda1d678317d89b74a240511) (Rob Gonnella)

- another test for github actions [_(c0273af)_](https://github.com/robgonnella/releasaurus/commit/c0273af95486fe86c672129c4d332f11a78dc911) (Rob Gonnella)

- use github.action*ref in github action [*(ae8f1f3)\_](https://github.com/robgonnella/releasaurus/commit/ae8f1f3fc6f19b5550a16a95f0b2aaca8d90f2e2) (Rob Gonnella)

- test different path for Dockerfile in github action [_(eb7c0dc)_](https://github.com/robgonnella/releasaurus/commit/eb7c0dcc48e4670f69daf87af8e97ab11b55a42f) (Rob Gonnella)

- temporarily use @main in github actions to test [_(8aa5d5a)_](https://github.com/robgonnella/releasaurus/commit/8aa5d5a02b4b80ea1a2cff387466313ce507f43f) (Rob Gonnella)

- removes unecessary checkout in release workflow [_(4283c7a)_](https://github.com/robgonnella/releasaurus/commit/4283c7af4bb51d97af3eacf7a5d8e12705369d38) (Rob Gonnella)

- another fix for syncing mirror repos [_(5568780)_](https://github.com/robgonnella/releasaurus/commit/556878095e0d0d72cf9513cb68fa56895b645218) (Rob Gonnella)

- fixes sync workflow [_(f169ec1)_](https://github.com/robgonnella/releasaurus/commit/f169ec1836f2329b162e4c6dc59bf94a4a365428) (Rob Gonnella)

- sets ssh-strict to false for mirror sync workflow [_(7e63372)_](https://github.com/robgonnella/releasaurus/commit/7e63372c469b69a61e7e67c23c5f30db537d11bc) (Rob Gonnella)

### 🧹 Chore

- sync gitlab and gitea mirrors [_(a5cc653)_](https://github.com/robgonnella/releasaurus/commit/a5cc65367a384f73b16798c342df1d2b6604abe3) (Rob Gonnella)

### 🚜 Refactor

- improves handling of tag timestamp parsing [_(1322f8f)_](https://github.com/robgonnella/releasaurus/commit/1322f8fb82b194ae302341ebb79e6cd3f6b8c0b4) (Rob Gonnella)

- refactors release command [_(d1366ce)_](https://github.com/robgonnella/releasaurus/commit/d1366cead792750b993b40fea75caa62aa9e34b2) (Rob Gonnella)

- set default*branch once at initialization of forge [*(95b4ff4)\_](https://github.com/robgonnella/releasaurus/commit/95b4ff40910e056e89fd1d667eb01fd4725eca7c) (Rob Gonnella)

- removes uses of unwrap and expect in live code [_(61552a2)_](https://github.com/robgonnella/releasaurus/commit/61552a2f23911353542ebff168c6cf5d9eb8582b) (Rob Gonnella)

- minor refactor to more idomatic rust conventions [_(0d3efb3)_](https://github.com/robgonnella/releasaurus/commit/0d3efb30518ce8b066146c4a1a68033e46236679) (Rob Gonnella)

- major refactor to updater processing logic [_(ad6af80)_](https://github.com/robgonnella/releasaurus/commit/ad6af80e6f47a8464e8a9203a4a06d35f7f13090) (Rob Gonnella)

- refactors ruby and php updaters [_(23336b7)_](https://github.com/robgonnella/releasaurus/commit/23336b7ef045959c2aac494bc6fe46cafb0461dc) (Rob Gonnella)

- refactors java updater [_(a60c009)_](https://github.com/robgonnella/releasaurus/commit/a60c0097b7bd2c3e1fa61678404dce409a81ca6f) (Rob Gonnella)

- updates commands to take mockable params [_(a9acf4a)_](https://github.com/robgonnella/releasaurus/commit/a9acf4a7d6d0c3f181ecc35dea33220326b10115) (Rob Gonnella)

- moves commit*search_depth to config [*(7a0e09a)\_](https://github.com/robgonnella/releasaurus/commit/7a0e09a88a0fe811c4ce848a0d0f9534ddb4fed0) (Rob Gonnella)

- implements tag*commit method for each forge [*(6e09e37)\_](https://github.com/robgonnella/releasaurus/commit/6e09e372d82349c827ec3e29001630ddf53dbbaf) (Rob Gonnella)

- implements updaters in new flow [_(7ac135e)_](https://github.com/robgonnella/releasaurus/commit/7ac135e973f6eee6a511ff596fb188628e47ebd3) (Rob Gonnella)

- partially implement new flow for gitea [_(accddf5)_](https://github.com/robgonnella/releasaurus/commit/accddf5aeba13c99d93dc93712f3898ccb1614d6) (Rob Gonnella)

- partial implementation of new forge flow [_(2133018)_](https://github.com/robgonnella/releasaurus/commit/213301896fddb2e447ce54b51cedd73e64dd1ca9) (Rob Gonnella)

- stub out trait method and refactor types [_(82aec90)_](https://github.com/robgonnella/releasaurus/commit/82aec90fbecdb08c3430cb5454e0049e6fe90b93) (Rob Gonnella)

- gets latest tag directly from forge (#87) [_(2b9b0ff)_](https://github.com/robgonnella/releasaurus/commit/2b9b0ff413338c702549630b922d96c9453ca3e0) (Rob Gonnella)

### 🧪 Testing

- adds additional unit tests to release.rs command [_(4f98ad2)_](https://github.com/robgonnella/releasaurus/commit/4f98ad2a9f20b129d0194e3db225a4da3aa69157) (Rob Gonnella)

- adds unit tests for framework.rs [_(4dae2f9)_](https://github.com/robgonnella/releasaurus/commit/4dae2f9482fb688002be7d6b86a7093a63eac43f) (Rob Gonnella)

- adds back unit tests for release*pr.rs [*(a5914bd)\_](https://github.com/robgonnella/releasaurus/commit/a5914bd86f7b3b32c7f4f10398d4119387df7036) (Rob Gonnella)

- adds basic smoke tests for each of the updater entrypoints [_(85095ac)_](https://github.com/robgonnella/releasaurus/commit/85095acc758bb909c2ba698bc94313cb4110141b) (Rob Gonnella)

- adds back rust updater unit tests [_(f2e2b23)_](https://github.com/robgonnella/releasaurus/commit/f2e2b23b33ae2429e7f77570ac7b80f6658b793a) (Rob Gonnella)

- adds back ruby updater unit tests [_(26637de)_](https://github.com/robgonnella/releasaurus/commit/26637de2161c34316b9c1aa3ea6e6182116789de) (Rob Gonnella)

- adds back python updater unit tests [_(efd2d45)_](https://github.com/robgonnella/releasaurus/commit/efd2d4587568c711681171f78ad88bedfb10abc3) (Rob Gonnella)

- adds back php updater unit tests [_(e64557c)_](https://github.com/robgonnella/releasaurus/commit/e64557c300c10701e1bbfb4614158979fc9268fb) (Rob Gonnella)

- adds back node updater unit tests [_(40d82c7)_](https://github.com/robgonnella/releasaurus/commit/40d82c7260e7435070696f5a071cc0020c2f90de) (Rob Gonnella)

- adds back java updater unit tests [_(147ae21)_](https://github.com/robgonnella/releasaurus/commit/147ae21641aa6a9a968117a30ab2b4cfc76695b0) (Rob Gonnella)

- implements integration / e2e tests for each forge [_(93918cc)_](https://github.com/robgonnella/releasaurus/commit/93918cca26af64985ec2af3c7a87b70440e7c66f) (Rob Gonnella)

- creates common test*helpers module [*(e2bc383)\_](https://github.com/robgonnella/releasaurus/commit/e2bc383c649701b739713708df989bf86746e092) (Rob Gonnella)

- adds unit tests for src/command/release.rs [_(d08a211)_](https://github.com/robgonnella/releasaurus/commit/d08a211c1b1e686d947492686550b1c0cafef58e) (Rob Gonnella)

- adds unit tests for src/command/release*pr.rs [*(e1d9e9e)\_](https://github.com/robgonnella/releasaurus/commit/e1d9e9e8883b9f4986b64c0d79b8b098d504e577) (Rob Gonnella)

- adds unit tests for src/analyzer.rs [_(c636411)_](https://github.com/robgonnella/releasaurus/commit/c63641123ad6053f227fd01c12cc4fb34d5bfe1f) (Rob Gonnella)

- adds unit tests for src/updater/manager.rs [_(53e6ba9)_](https://github.com/robgonnella/releasaurus/commit/53e6ba91a06a857fcdc81fdca8d25e1c5ed39aaf) (Rob Gonnella)

- adds manual mock for PackageUpdater trait [_(c342101)_](https://github.com/robgonnella/releasaurus/commit/c342101c830f7aabb0be3d484119b6ce758997fd) (Rob Gonnella)

- adds back test for rust updater [_(2a7c5c0)_](https://github.com/robgonnella/releasaurus/commit/2a7c5c05085262a9d4465e486ef60ddab69816a0) (Rob Gonnella)

- adds back tests for python updater [_(8138803)_](https://github.com/robgonnella/releasaurus/commit/81388033e811d9336731914c0c823593e01d021f) (Rob Gonnella)

- adds back tests for php updater [_(1ffa734)_](https://github.com/robgonnella/releasaurus/commit/1ffa7342635b366329ae723ef9ba6c8e061d7520) (Rob Gonnella)

- adds back tests for node updater [_(9ddfd8d)_](https://github.com/robgonnella/releasaurus/commit/9ddfd8dd4295fda1624267ae8f6c2661c48b34f6) (Rob Gonnella)

- adds back java updater tests [_(94a6364)_](https://github.com/robgonnella/releasaurus/commit/94a636481f2ad2a4d9cfe2d979203c935b0c7359) (Rob Gonnella)

- add mocks for forge traits [_(570ccd2)_](https://github.com/robgonnella/releasaurus/commit/570ccd23c0373b23c6a600a5656d05fb37811335) (Rob Gonnella)

- adds unit tests for src/forge/config.rs [_(57c1c2a)_](https://github.com/robgonnella/releasaurus/commit/57c1c2aead34cc44bc8f6367ed11b18e5e87210a) (Rob Gonnella)

- adds unit tests for analyzer/helpers.rs [_(f61b8cd)_](https://github.com/robgonnella/releasaurus/commit/f61b8cd9f1eab912e98b6668be284d02e210082e) (Rob Gonnella)

- adds unit tests for analyzer/commit.rs [_(81776ad)_](https://github.com/robgonnella/releasaurus/commit/81776ad42d3c121164a8451c5fc7b59bbf527837) (Rob Gonnella)

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

## <!--releasaurus_footer_start-->

Generated by Releasaurus 🦕

<!--releasaurus_footer_end-->
