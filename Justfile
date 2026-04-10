all:
    @just -l

fmt:
    cargo fmt

lint:
    cargo clippy --all-targets --all-features

test *args:
    cargo test {{ args }}

test-cov *args:
    cargo llvm-cov \
      --ignore-filename-regex "(_test\.rs$)|(_tests\.rs$)" {{ args }}

test-all *args:
    cargo llvm-cov \
      --ignore-filename-regex "(_test\.rs$)|(_tests\.rs$)" \
      --features _integration_tests {{ args }}
