# Prints help menu for recipes
default: help

# Prints help menu for recipes
help:
    @just --list

# Builds releasaurus
build *args:
    cargo build {{ args }}

# Runs releasaurus cli
run *args:
    cargo run -p releasaurus -- {{ args }}

# Formats all rust code
fmt:
    cargo fmt

# Lints all rust code
lint:
    cargo clippy --all-targets --all-features

gen-schema:
    cargo run -p releasaurus-core --bin gen_schema

# Runs unit tests
test *args:
    cargo test {{ args }}

# Runs unit tests using llvm-cov for coverage
test-cov *args:
    cargo llvm-cov \
      --ignore-filename-regex "(_test\.rs$)|(_tests\.rs$)" {{ args }}

# Runs all tests, including integration tests which execute against real forges. You must have proper env vars set for this to work
test-all *args:
    cargo llvm-cov \
      --ignore-filename-regex "(_test\.rs$)|(_tests\.rs$)" \
      --features _integration_tests {{ args }} -- --nocapture

_test_integration test:
    @just test --features _integration_tests {{ test }} -- --nocapture

# Runs only the github integration tests
test-github-integration: (_test_integration "test_github_forge")

# Runs only the gitlab integration tests
test-gitlab-integration: (_test_integration "test_gitlab_forge")

# Runs only the gitea integration tests
test-gitea-integration: (_test_integration "test_gitea_forge")
