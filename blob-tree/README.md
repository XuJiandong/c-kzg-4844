# Blob Tree

A Rust membership proofs over two levels of EIP-4844 blobs, with capacity for up to 4096² elements.

- [Design, terminology, security assumptions, and encoding](DESIGN.md)
- [Tutorial](tutorial/README.md): illustrated tree structure, proof reduction, and security boundaries; readable directly on GitHub
- [Protocol implementation](src/lib.rs) / [Algebra operations](src/algebra.rs)
- [Example](examples/basic.rs) / [Tests](tests/membership.rs)

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
