# Blob Tree

A Rust membership proofs over two levels of EIP-4844 blobs, with capacity for up to 4096² elements.

- [Design, terminology, security assumptions, and encoding](DESIGN.md)
- [Tutorial](tutorial/README.md): illustrated tree structure, proof reduction, and security boundaries; readable directly on GitHub
- [Protocol implementation](src/lib.rs) / [Algebra operations](src/algebra.rs)
- [Example](examples/basic.rs) / [Tests](tests/membership.rs)

## Native generation benchmarks

Run `make blob-tree-bench-native` for commitment generation (`Tree::new`) with
10,000, 100,000, and 1,000,000 elements, and multiproof generation (`Tree::prove`)
for 10, 100, and 1,000 evenly spaced leaves in a 100,000-element tree.
Inputs are sorted, unique eight-byte values. Timings exclude input preparation,
trusted setup initialization, output destruction, and, for proofs, tree construction
and verification. Criterion reports wall-clock time and elements per second.

## Verification on CKB-VM

Use `verification-only` with default features disabled:

```toml
blob-tree = { path = "../blob-tree", default-features = false, features = ["verification-only"] }
```

With the [CKB test prerequisites](../ckb-tests/README.md) installed, run:

```sh
# Native generation, both native verifier modes, and CKB-VM verification:
make blob-tree-test
# Native generation and CKB-VM verification benchmarks:
make blob-tree-bench
```

The CKB-VM benchmark reports cycles for `blob_tree::verify`, excluding fixture decoding
and VM startup. The debugger's `All cycles` includes that overhead.
The large case queries every 100th element of a 100,000-element tree: 1,000
queries spanning all 25 children. Regenerate fixtures with
`cargo run --release --manifest-path blob-tree/Cargo.toml --locked --example generate-fixtures`.
