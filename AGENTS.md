## Tests
The following tests must be passed after final changes:
```
make ckb-test
```

Also run following after big refactor:
```
cargo test --locked
cargo test --manifest-path blob-tree/Cargo.toml --locked
cargo test --manifest-path blob-tree/Cargo.toml --locked --no-default-features --features verification-only
```
