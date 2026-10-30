Two important projects are maintained and developed in this workspace:
- blob-tree (in the blob-tree folder)
- c-kzg, the RISC-V version (in the bindings/rust folder)

Ignore all other bindings and do not update them.

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

## Documents
When updating documentation, keep [README.md](./README.md) and [blob-tree/README.md](./blob-tree/README.md) simple and easy to read.
Avoid letting them bloat.


## Benchmarking and optimization

After optimizing c-kzg-4844 or blob-tree, run the following command to collect benchmarks:
```
make -C ckb-tests bench-blob-tree
```

ckb-debugger supports [flamegraph](https://github.com/nervosnetwork/ckb-standalone-debugger/blob/develop/ckb-debugger/guide.md#performance-analysis-with-flamegraph). Before optimizing, use them to identify the bottleneck.

