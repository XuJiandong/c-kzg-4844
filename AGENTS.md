

## Tests
The following tests must be passed after final changes:
```
cargo test --locked
cargo test --manifest-path blob-tree/Cargo.toml --locked
cargo test --manifest-path blob-tree/Cargo.toml --locked --no-default-features --features verification-only
make ckb-test
```