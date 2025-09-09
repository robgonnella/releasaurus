all:
  @just -l

test *args:
  cargo test {{args}}

test-unit *args:
    cargo llvm-cov \
        --lib \
        --bins \
        --workspace \
        --ignore-filename-regex "(_test\.rs$)|(_tests\.rs$)(tests\/)" {{args}}

test-integration *args:
    cargo test --test '*' {{args}}

test-all-cov *args:
  cargo llvm-cov \
    --workspace \
    --ignore-filename-regex "(_test\.rs$)|(_tests\.rs$)(tests\/)" {{args}}
