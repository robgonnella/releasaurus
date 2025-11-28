all:
  @just -l

test *args:
  cargo test {{args}}

test *args:
    cargo test {{args}}

test-cov *args:
    cargo llvm-cov \
      --ignore-filename-regex "(_test\.rs$)|(_tests\.rs$)" {{args}}
