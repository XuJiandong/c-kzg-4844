# EIP-4844 verification on CKB-VM

The root crate's `verification-only` feature exposes a `KzgVerifier` with point, blob, and batch verification. It excludes the setup loader, commitment/proof generation, recovery, and EIP-7594. The normal host bindings remain available without this feature.

## Build and run

Prerequisites: Rust with `riscv64imac-unknown-none-elf` installed, Clang >= 19
with a RISC-V backend, LLVM's `llvm-ar`, and the already installed
`ckb-debugger`.

From the repository root:

```sh
git submodule update --init --recursive
make ckb-test
```

This builds and runs two standalone binaries:

- `ckb-kzg-tests` (`src/main.rs`): EIP-4844 point, blob, and batch verification.
- `blob-tree` (`src/bin/blob-tree.rs`): all blob-tree VM tests, using its
  `verification-only` feature and native-generated membership proofs.

Run only the blob-tree binary with `make -C ckb-tests test-blob-tree`.
Run only the KZG binary with `make -C ckb-tests test-kzg`; `CASE` selects
`all` (default), `point`, `blob`, or `batch` for that binary.
Run the native blob-tree generation/verification tests and its VM binary
together with `make blob-tree-test`. The shared blob-tree cases
cover an empty leaf, queries across two children, wire decoding, changed
values/roots/commitments, invalid indices, and malformed or non-subgroup points.
Fixture regeneration is documented in [blob-tree](../blob-tree/README.md).

If Clang on your PATH lacks a RISC-V backend, pass the LLVM compiler and
archiver explicitly, for example:

```sh
make ckb-test CC=/opt/homebrew/opt/llvm@19/bin/clang AR=/opt/homebrew/opt/llvm@19/bin/llvm-ar
```

## Library use

```toml
c-kzg = { path = "..", default-features = false, features = ["verification-only"] }
```

```rust,ignore
let verifier = c_kzg::KzgVerifier::new(&authenticated_g2_x)?;
let valid = verifier.verify_kzg_proof(&commitment, &z, &y, &proof)?;
```

## RISC-V assembly

The two files in `blst/src/asm`, `blst_mul_mont_384.riscv.S` and
`blst_mul_mont_384x.riscv.S`, are copied unchanged from [ckb-blst-rs](https://github.com/nervosnetwork/ckb-blst-rs/).

`make ckb-test` measures:

| Function | Cycles per call |
| --- | --- |
| `verify_kzg_proof` | 48 M cycles |
| `verify_blob_kzg_proof` | 91 M cycles |
| `blob_tree::verify` (1 query, 1 child) | 80 M cycles |
| `blob_tree::verify` (6 queries, 2 children) | 89 M cycles |

The blob-tree measurements include creation of the compact verification
context and exclude proof decoding. They use the shared native-generated fixtures.

Compared with the pure C baseline below, performance improves substantially.

## Cycle baseline (pure C)

Baseline with BLST's portable C backend (`__BLST_NO_ASM__`):

| Function | Cycles per call |
| --- | --- |
| `verify_kzg_proof` | 257 M cycles |
| `verify_blob_kzg_proof` | 365 M cycles |
