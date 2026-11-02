# C-KZG-4844 on CKB-VM
This is a RISC-V port of c-kzg-4844 for CKB-VM. It adapts several core
algorithms from [ckb-blst-rs](https://github.com/nervosnetwork/ckb-blst-rs/).
It can perform a single pairing on BLS12-381 in 48M cycles. See Rust binding [here](./bindings/rust/README.md).
Other bindings (Python and Node) have not been ported to CKB-VM and will not be.

Building on a modified c-kzg-4844, we also designed and implemented a new tree to replace [SMT](https://github.com/nervosnetwork/sparse-merkle-tree).
See the [blob-tree tutorial](./blob-tree/tutorial/README.md) and the [code](./blob-tree/).

The original README.md is moved [here](./README.md).

