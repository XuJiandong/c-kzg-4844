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

The dependency on `c-kzg` points to this repository's root manifest, which
builds the Rust bindings in `bindings/rust/`. The default `prover` feature
provides `Tree` construction, proof generation, and verification on native
machines.

## Verification on CKB-VM

Use `verification-only` with default features disabled:

```toml
blob-tree = { path = "../blob-tree", default-features = false, features = ["verification-only"] }
```

This exposes `Root`, `Proof`, and the same `verify(root, values, proof)` API
with `no_std + alloc`; a global allocator is required. `Tree` and proving
operations are excluded. The verifier embeds only the 96-byte Ethereum
`[s]G2` key, copied from the native bindings' trusted setup, and uses
`c_kzg::KzgVerifier`. Both builds use the same transcript and proof encoding.
On RISC-V, the private BLST declarations link to the implementation already
built by `c-kzg`.

With the [CKB test prerequisites](../ckb-tests/README.md) installed, run:

```sh
# Native generation, both native verifier modes, and CKB-VM verification:
make blob-tree-test
# Only the CKB-VM blob-tree tests:
make -C ckb-tests test-blob-tree
# Only the native verification-only tests:
cargo test --manifest-path blob-tree/Cargo.toml --locked --no-default-features --features verification-only
```

If the default compiler lacks RISC-V support (for example, Apple Clang),
pass explicit compiler paths, such as
`make blob-tree-test CC=/opt/homebrew/opt/llvm@19/bin/clang AR=/opt/homebrew/opt/llvm@19/bin/llvm-ar`.

The native and VM suites share fixtures for an empty leaf and six queries
across two children, plus malformed and tampered proof cases. Regenerate
the fixtures using the native prover:

```sh
cargo run --manifest-path blob-tree/Cargo.toml --locked --example generate-fixtures
```

## Proofs and ordering

For `k` queries spanning `m` child subtrees, the proof is exactly `8 + 8k + 48m + 96` bytes. Verification makes one KZG verification call, with two pairing terms and one final exponentiation. Indices are stored in `Proof`; original values are supplied separately to `verify(root, values, proof)` in any order. Verification sorts values lexicographically by unsigned bytes, then pairs them with `proof.indices`. Applications requiring specific positions must check these indices. The trusted root must authenticate both the commitment and the vector length.

`Tree::new(&values)` sorts a copy using the same big-endian byte order before committing. Shorter prefixes sort first; empty values are allowed, and duplicate values are rejected by both construction and verification with `Error::DuplicateValue`. Indices refer to the sorted tree, and caller inputs are never modified.

This implementation has not undergone an independent cryptographic audit. See the design document for its security assumptions and proof-of-concept limitations.

```rust
let (values, proof) = tree.prove(&[0, 7])?;
let decoded = Proof::from_bytes(tree.root(), &proof.to_bytes())?;
assert!(verify(tree.root(), &values, &decoded)?);
```
