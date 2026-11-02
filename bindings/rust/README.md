# c-kzg-4844 verification on CKB-VM

The `verification-only` feature exposes a `KzgVerifier` on ckb-vm. It excludes the setup loader, commitment/proof generation, recovery, and EIP-7594.

## Build and tests

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
