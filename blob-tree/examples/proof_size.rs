//! Measure serialized membership proof sizes for a 100,000-element tree.
//!
//! Queries are evenly spaced across the sorted values, matching the generation
//! benchmark. N is the total number of leaves; M is the number of queried leaves
//! covered by one proof. Report the canonical wire size from `Proof::to_bytes()`,
//! including the query count and indices but excluding the public root and
//! original values. Build the tree once for all scenarios.
//!
//! Run from the repository root with:
//!   cargo run --manifest-path blob-tree/Cargo.toml --locked --example proof_size --release

use blob_tree::{verify, Proof, Tree};

fn main() -> blob_tree::Result<()> {
    const N: usize = 100_000;
    // Fixed-width big-endian values retain their numeric order when sorted.
    let input = (0..N as u64)
        .map(|i| i.to_be_bytes().to_vec())
        .collect::<Vec<_>>();
    let tree = Tree::new(&input)?;

    println!("N = total leaves stored in the tree; M = queried leaves covered by one proof.");
    println!("Queries are evenly spaced across the sorted values.");
    println!("Proof bytes include query indices and exclude the root and values.\n");
    println!(
        "{:>10} {:>12} {:>14}",
        "N (leaves)", "M (queries)", "proof size(bytes)"
    );
    for m in [1, 10, 100, 1_000] {
        let indices = (0..m).map(|i| i * N / m).collect::<Vec<_>>();
        let (values, proof) = tree.prove(&indices)?;
        let bytes = proof.to_bytes();
        let received = Proof::from_bytes(tree.root(), &bytes)?;
        assert!(verify(tree.root(), &values, &received)?);
        println!("{:>10} {:>12} {:>14}", N, m, bytes.len());
    }
    Ok(())
}
