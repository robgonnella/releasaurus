all:
    @just -l

test *args:
    cargo test {{ args }}

test-cov *args:
    cargo llvm-cov \
      --ignore-filename-regex "(_test\.rs$)|(_tests\.rs$)" {{ args }}

test-all *args:
    cargo llvm-cov \
      --ignore-filename-regex "(_test\.rs$)|(_tests\.rs$)" \
      --features _integration_tests {{ args }}
