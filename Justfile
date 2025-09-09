all:
  @just -l

test *args:
  cargo test {{args}}

test-unit:
    cargo llvm-cov \
        --lib \
        --bins \
        --workspace \
        --ignore-filename-regex "(_test\.rs$)|(_tests\.rs$)(tests\/)"

test-integration:
    cargo test --test '*'

test-all-cov *args:
  cargo llvm-cov \
    --workspace \
    --ignore-filename-regex "(_test\.rs$)|(_tests\.rs$)(tests\/)" {{args}}
