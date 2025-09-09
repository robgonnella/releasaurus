all:
  @just -l

test *args:
  cargo test {{args}}

test-unit *args:
    cargo llvm-cov --ignore-filename-regex "(_test\.rs$)|(_tests\.rs$)" {{args}}

test-integration *args:
    cargo llvm-cov \
      --features _internal_e2e_tests \
      --ignore-filename-regex "(_test\.rs$)|(_tests\.rs$)" {{args}}
