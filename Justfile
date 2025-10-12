all:
  @just -l

test *args:
  cargo test {{args}}

test-unit *args:
    cargo test

test-e2e *args:
  cargo test --features _internal_e2e_tests

test-all *args:
    cargo llvm-cov \
      --features _internal_e2e_tests \
      --ignore-filename-regex "(_test\.rs$)|(_tests\.rs$)" {{args}}
