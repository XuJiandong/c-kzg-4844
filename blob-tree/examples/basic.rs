use blob_tree::{verify, Proof, Tree, WIDTH};

fn main() -> blob_tree::Result<()> {
    let values = (0..WIDTH + 2)
        .map(|i| format!("element-{i}").into_bytes())
        .collect::<Vec<_>>();
    let tree = Tree::new(&values)?;
    let (values, proof) = tree.prove(&[0, 7, WIDTH, WIDTH + 1])?;
    let bytes = proof.to_bytes();
    let received = Proof::from_bytes(tree.root(), &bytes)?;
    assert!(verify(tree.root(), &values, &received)?);
    println!(
        "Verified {} elements across {} children; proof = {} bytes",
        values.len(),
        proof.children.len(),
        bytes.len()
    );
    Ok(())
}
