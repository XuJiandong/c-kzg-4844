# EIP-4844 verification on CKB-VM

The root crate's `verification-only` feature exposes a `KzgVerifier` with point, blob, and batch verification. It excludes the setup loader, commitment/proof generation, recovery, and EIP-7594. The normal host bindings remain available without this feature.

## Build and run

Prerequisites: Rust with `riscv64imac-unknown-none-elf` installed, Clang >= 19
with a RISC-V backend, LLVM's `llvm-ar`, Python 3, and the already installed
`ckb-debugger`.

From the repository root:

```sh
git submodule update --init --recursive
make ckb-test
```

## Library use on ckb-vm

```toml
c-kzg = { path = "..", default-features = false, features = ["verification-only"] }
```

```rust,ignore
let verifier = c_kzg::KzgVerifier::new(&authenticated_g2_x)?;
let valid = verifier.verify_kzg_proof(&commitment, &z, &y, &proof)?;
```

## RISC-V assembly
`make ckb-test` measures:

| Function | Cycles per call |
| --- | --- |
| `verify_kzg_proof` | 45 M cycles |
| `verify_blob_kzg_proof` | 75 M cycles |

Compared with the pure C baseline below, performance improves substantially.

## Cycle baseline (pure C)

Baseline with BLST's portable C backend (`__BLST_NO_ASM__`):

| Function | Cycles per call |
| --- | --- |
| `verify_kzg_proof` | 257 M cycles |
| `verify_blob_kzg_proof` | 365 M cycles |
