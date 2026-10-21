The mission is to port the verification part of EIP-4844 Rust bindings to the RISC-V architecture (rv64imc with the B extension).
The setup, proof generation and recover parts are out of scope.  The EIP-7594 is out of scope.
The ported part(verifying part) should support `no_std` in Rust. 

## Compiler
Use only `clang` (version >= 19) as the C compiler; no other compilers are accepted. Report error if clang is missing.

## C Runtime
Use ckb-c-stdlib as the C runtime library. See the guide here: https://github.com/nervosnetwork/ckb-c-stdlib/blob/master/guide.md
The CFLAGS `--target=riscv64 -march=rv64imc_zba_zbb_zbc_zbs` are important and must be used.

## Tests
First, use ckb-std to build a RISC-V binary. See the guide here: https://github.com/nervosnetwork/ckb-std/blob/master/guide.md.

Then use ckb-debugger to run the RISC-V binary. See the ckb-debugger guide here: https://github.com/nervosnetwork/ckb-standalone-debugger/blob/develop/ckb-debugger/guide.md. Don't try to install ckb-debugger: it is already installed.

ckb-debugger accepts arguments, which are passed to the binary. ckb-std can print logs. These 2 features are enough for debugging and verifying.

There is no need to port all tests; a few simple cases will suffice. Place all tests under `ckb-tests` folder.
