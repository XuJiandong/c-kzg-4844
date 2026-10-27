# Blob Tree

A Rust proof of concept for membership proofs over two levels of EIP-4844 blobs, with capacity for up to 4096² elements.

- [Design, terminology, security assumptions, and encoding](DESIGN.md)
- [Tutorial](tutorial/README.md): illustrated tree structure, proof reduction, and security boundaries; readable directly on GitHub
- [Protocol implementation](src/lib.rs) / [Algebra operations](src/algebra.rs)
- [Example](examples/basic.rs) / [Tests](tests/membership.rs)

Run from the repository root:

```sh
cargo test --manifest-path blob-tree/Cargo.toml --locked
cargo run --manifest-path blob-tree/Cargo.toml --example basic --locked
```

For `k` queries spanning `m` child subtrees, the proof is exactly `8 + 8k + 48m + 96` bytes. Verification makes one KZG verification call, with two pairing terms and one final exponentiation. Indices are stored in `Proof`; original values are supplied separately to `verify(root, values, proof)` in any order. Verification sorts values lexicographically by unsigned bytes, then pairs them with `proof.indices`. Applications requiring specific positions must check these indices. The trusted root must authenticate both the commitment and the vector length.

`Tree::new(&values)` sorts a copy using the same big-endian byte order before committing. Shorter prefixes sort first; empty values are allowed, and duplicate values are rejected by both construction and verification with `Error::DuplicateValue`. Indices refer to the sorted tree, and caller inputs are never modified.

This implementation has not undergone an independent cryptographic audit. See the design document for its security assumptions and proof-of-concept limitations.

```rust
let (values, proof) = tree.prove(&[0, 7])?;
let decoded = Proof::from_bytes(tree.root(), &proof.to_bytes())?;
assert!(verify(tree.root(), &values, &decoded)?);
```
