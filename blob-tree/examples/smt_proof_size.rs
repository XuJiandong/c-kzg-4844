//! Measure SMT compiled proof sizes alongside the blob-tree `proof_size` example.
//!
//! Matches sparse-merkle-tree/examples/proof_size.rs: 100,000 deterministic
//! hashed keys and values, querying the first M keys. Build the tree once using
//! the `trie` backend enabled in dev-dependencies to avoid slow construction.
//! N is the total number of leaves; M is the number of queried leaves covered
//! by one proof.
//!
//! Compiled bytes exclude the root, query keys, and values. In contrast,
//! blob-tree's serialized proof includes its query indices.
//!
//! Run from the repository root with:
//!   cargo run --manifest-path blob-tree/Cargo.toml --locked --example smt_proof_size --release

use blake2b_rs::Blake2bBuilder;
use sparse_merkle_tree::{
    blake2b::Blake2bHasher, default_store::DefaultStore, SparseMerkleTree, H256,
};

type Smt = SparseMerkleTree<Blake2bHasher, H256, DefaultStore<H256>>;

fn hash_at(prefix: &[u8], i: u64) -> H256 {
    let mut hasher = Blake2bBuilder::new(32).personal(b"smt-pfsize").build();
    hasher.update(prefix);
    hasher.update(&i.to_le_bytes());
    let mut bytes = [0u8; 32];
    hasher.finalize(&mut bytes);
    bytes.into()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const N: u64 = 100_000;
    let mut tree = Smt::default();
    for i in 0..N {
        tree.update(hash_at(b"", i), hash_at(b"value", i))?;
    }

    println!("N = total leaves stored in the tree; M = queried leaves covered by one proof.");
    println!("Queries select the first M deterministic hashed keys inserted into the tree.");
    println!("Compiled proof bytes exclude the root, query keys, and values.\n");
    println!(
        "{:>10} {:>12} {:>14}",
        "N (leaves)", "M (queries)", "proof size(bytes)"
    );
    for m in [1, 10, 100, 1_000] {
        let leaves = (0..m)
            .map(|i| (hash_at(b"", i), hash_at(b"value", i)))
            .collect::<Vec<_>>();
        let keys = leaves.iter().map(|(key, _)| *key).collect::<Vec<_>>();
        let proof = tree.merkle_proof(keys.clone())?;
        let compiled = proof.compile(keys)?;
        assert!(compiled.verify::<Blake2bHasher>(tree.root(), leaves)?);
        println!("{:>10} {:>12} {:>14}", N, m, compiled.0.len());
    }
    Ok(())
}
