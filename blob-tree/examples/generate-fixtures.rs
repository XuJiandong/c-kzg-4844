//! Regenerate the native proofs consumed by both host and CKB-VM tests.
use blob_tree::{verify, Tree, WIDTH};
use std::{fs, path::Path};

fn write(name: &str, input: &[Vec<u8>], indices: &[usize]) {
    let tree = Tree::new(input).unwrap();
    let (values, proof) = tree.prove(indices).unwrap();
    assert_eq!(verify(tree.root(), &values, &proof), Ok(true));
    let mut bytes = tree.root().commitment.to_vec();
    bytes.extend_from_slice(&(tree.root().len as u64).to_be_bytes());
    bytes.extend_from_slice(&(values.len() as u64).to_be_bytes());
    for value in values {
        bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
        bytes.extend_from_slice(&value);
    }
    bytes.extend_from_slice(&proof.to_bytes());
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(format!("{name}.bin")), bytes).unwrap();
}

fn main() {
    write("single", &[vec![]], &[0]);
    let input = (0..WIDTH + 3)
        .map(|i| (i as u64).to_be_bytes().to_vec())
        .collect::<Vec<_>>();
    write("multirow", &input, &[0, 1, 77, WIDTH - 1, WIDTH, WIDTH + 2]);
}
