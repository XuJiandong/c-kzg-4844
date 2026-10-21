# EIP-4844 verification on CKB VM

The root crate's `verification-only` feature exposes a `no_std + alloc`
`KzgVerifier` with point, blob, and batch verification. It excludes the setup
loader, commitment/proof generation, recovery, and EIP-7594. The normal host
bindings remain available without this feature.

## Build and run

Prerequisites: Rust with `riscv64imac-unknown-none-elf` installed, Clang >= 19
with a RISC-V backend, LLVM's `llvm-ar`, and the already installed
`ckb-debugger`.

From the repository root:

```sh
git submodule update --init --recursive
make ckb-test
# Select one group via the binary's argv:
make ckb-test CASE=point
make ckb-test CASE=blob
make ckb-test CASE=batch
# Override tool locations when necessary:
make ckb-test CC=/path/to/clang AR=/path/to/llvm-ar
```

`make ckb-build` builds without running. The ELF is
`ckb-tests/target/riscv64imac-unknown-none-elf/release/ckb-kzg-tests`.
Run Cargo from `ckb-tests` so it reads that directory's target configuration.
The Makefile resolves Clang to an absolute path before invoking Cargo.

C sources (including the vendored BLST backend and ckb-c-stdlib)
use `--target=riscv64 -march=rv64imc_zba_zbb_zbc_zbs`. Rust follows the ckb-std
guide: the installed IMAC target plus B extension flags and `lower-atomic`.
The target's ELF attributes may still advertise A because of Rust's prebuilt
core library; the executable is tested on CKB VM, which has no atomic extension.
The final executable was also disassembled to check for AMO/LR/SC instructions.

## Library use

```toml
c-kzg = { path = "..", default-features = false, features = ["verification-only"] }
```

```rust,ignore
let verifier = c_kzg::KzgVerifier::new(&authenticated_g2_x)?;
let valid = verifier.verify_kzg_proof(&commitment, &z, &y, &proof)?;
```

Inputs are byte arrays: blobs are 131072 bytes, commitments/proofs 48 bytes,
and field elements 32 bytes. Malformed encodings return an error; well-formed
incorrect proofs return `Ok(false)`. Batch slice lengths must match.

The constructor takes the compressed 96-byte **[s]G2** verification key.
The caller must authenticate it against the intended trusted setup; subgroup
validation alone does not establish trust. The tests use the second G2 point
from the existing Ethereum setup fixture. The context constructs only the 4096
bit-reversed evaluation roots and the two G2 points needed for verification.

Applications must provide a Rust global allocator. The C verifier's private
allocation functions use it and return allocation failures to C. The test
binary uses ckb-std's allocator with a 2 MiB heap. The library compiles the
pinned `deps/ckb-c-stdlib` runtime itself; consumers need not enable ckb-std's
`libc` feature. Contexts own their storage and release it on drop.

## Coverage and limits

The test build extracts two existing upstream reference cases:
`verify_kzg_proof_case_correct_proof_3_2` and
`verify_blob_kzg_proof_case_correct_proof_3`. Tests cover:

- Zero and nonzero valid point/blob proofs.
- Incorrect evaluations and commitments; malformed points/scalars and key.
- Empty batches, mismatched lengths, valid nonzero two-blob batches, and an
  incorrect proof in a batch.

This is a small portability smoke suite, not the entire reference suite.
The BLST backend uses RV64 assembly for Fp and Fp2 Montgomery multiplication
and 64-bit C fallbacks for the remaining operations. The combined suite uses
730,583,386 cycles. Individual groups can be run separately for profiling.

## RISC-V assembly

The two files in `blst/src/asm`, `blst_mul_mont_384.riscv.S` and
`blst_mul_mont_384x.riscv.S`, are copied unchanged from `ckb-blst-rs` commit
`64c5cc4beb912838de4eb7be378968a0bef7a446`. The root Rust build enables them
for RISC-V via `__BLST_RISCV_ASM__`. `__BLST_NO_ASM__` remains defined to
provide C implementations of the other operations; the partial assembly
backend explicitly selects 64-bit limbs and 128-bit intermediate arithmetic.

With Clang 19.1.7 and the same nonzero vectors, `make ckb-test` measures:

| Function | Cycles per call |
| --- | --- |
| `verify_kzg_proof` | 48 M cycles |
| `verify_blob_kzg_proof` | 91 M cycles |

These improvements include both the assembly and the switch to 64-bit C limbs.

## Cycle baseline (pure C)

Baseline with BLST's portable C backend (`__BLST_NO_ASM__`), measured in
`ckb-debugger` using ckb-std's `current_cycles()` around one successful call
with the nonzero reference vectors above:

| Function | Cycles per call |
| --- | --- |
| `verify_kzg_proof` | 257 M cycles |
| `verify_blob_kzg_proof` | 365 M cycles |

Values are rounded to the nearest million cycles. Measurements exclude context
initialization, assertions, and logging, but include minor measurement overhead.

Reference guides:
[ckb-std](https://github.com/nervosnetwork/ckb-std/blob/master/guide.md),
[ckb-c-stdlib](https://github.com/nervosnetwork/ckb-c-stdlib/blob/master/guide.md),
[ckb-debugger](https://github.com/nervosnetwork/ckb-standalone-debugger/blob/develop/ckb-debugger/guide.md).
