all:
  @just -l

test *args:
  cargo test {{args}}

test-cov *args:
  cargo llvm-cov \
    --workspace \
    --ignore-filename-regex "(_test\.rs$)|(_tests\.rs$)(tests\/)" {{args}}
