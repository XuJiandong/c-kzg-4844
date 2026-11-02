use blob_tree::*;

fn rejected(root: &Root, values: &[Vec<u8>], proof: &Proof) {
    assert!(!matches!(verify(root, values, proof), Ok(true)));
}

#[test]
fn single_empty_value_and_wire_roundtrip() {
    let tree = Tree::new(&[vec![]]).unwrap();
    let (values, proof) = tree.prove(&[0]).unwrap();
    assert!(verify(tree.root(), &values, &proof).unwrap());
    assert_eq!(proof.to_bytes().len(), 146);
    assert_eq!(
        Proof::from_bytes(tree.root(), &proof.to_bytes()).unwrap(),
        proof
    );
    let bytes = proof.to_bytes();
    assert_eq!(&bytes[..2], &[1, 0]);
    for len in 0..bytes.len() {
        assert!(Proof::from_bytes(tree.root(), &bytes[..len]).is_err());
    }
    let mut oversized = bytes.clone();
    oversized[..4].copy_from_slice(&[0xff; 4]);
    assert!(Proof::from_bytes(tree.root(), &oversized).is_err());
    let mut bytes = bytes;
    bytes.push(0);
    assert!(Proof::from_bytes(tree.root(), &bytes).is_err());
    assert!(Proof::from_bytes(tree.root(), &bytes[..100]).is_err());
}

#[test]
fn multirow_and_adversarial_changes() {
    let values = (0..WIDTH + 3)
        .map(|i| (i as u64).to_be_bytes().to_vec())
        .collect::<Vec<_>>();
    let tree = Tree::new(&values).unwrap();
    let (values, proof) = tree
        .prove(&[0, 1, 77, WIDTH - 1, WIDTH, WIDTH + 2])
        .unwrap();
    assert_eq!(proof.indices, [0, 1, 77, WIDTH - 1, WIDTH, WIDTH + 2]);
    assert_eq!(values[2], 77u64.to_be_bytes());
    assert_eq!(
        Proof::from_bytes(tree.root(), &proof.to_bytes()).unwrap(),
        proof
    );
    for indices in [vec![], vec![0, 0], vec![1, 0], vec![WIDTH + 3]] {
        let mut bad = proof.clone();
        bad.indices = indices;
        rejected(tree.root(), &values, &bad);
        assert!(Proof::from_bytes(tree.root(), &bad.to_bytes()).is_err());
    }
    let mut extra = values.clone();
    extra.push(vec![]);
    rejected(tree.root(), &extra, &proof);
    assert_eq!(proof.children.len(), 2);
    assert_eq!(proof.to_bytes().len(), 202);
    assert!(verify(tree.root(), &values, &proof).unwrap());
    let mut bad = values.clone();
    bad[0].push(1);
    rejected(tree.root(), &bad, &proof);
    let mut bad = proof.clone();
    bad.indices[2] += 1;
    rejected(tree.root(), &values, &bad);
    let mut reordered = values.clone();
    reordered.reverse();
    assert!(verify(tree.root(), &reordered, &proof).unwrap());
    let mut bad = values.clone();
    bad[1] = bad[0].clone();
    rejected(tree.root(), &bad, &proof);
    rejected(tree.root(), &values[..values.len() - 1], &proof);
    let mut root = tree.root().clone();
    root.len += 1;
    rejected(&root, &values, &proof);
    let mut root = tree.root().clone();
    root.commitment = proof.children[0];
    rejected(&root, &values, &proof);
    let mut bad = proof.clone();
    bad.children.swap(0, 1);
    rejected(tree.root(), &values, &bad);
    let mut bad = proof.clone();
    bad.children.push(proof.children[0]);
    rejected(tree.root(), &values, &bad);
    let mut bad = proof.clone();
    bad.quotient = proof.opening;
    rejected(tree.root(), &values, &bad);
    let mut bad = proof.clone();
    bad.opening = proof.quotient;
    rejected(tree.root(), &values, &bad);
    let mut infinity = [0; 48];
    infinity[0] = 0xc0;
    let mut bad = proof.clone();
    bad.opening = infinity;
    rejected(tree.root(), &values, &bad);
    let mut bad = proof.clone();
    bad.quotient = [0xff; 48];
    rejected(tree.root(), &values, &bad);
    let mut bad = proof.clone();
    bad.children[0] = [0; 48];
    rejected(tree.root(), &values, &bad);
    for indices in [vec![], vec![0, 0], vec![2, 1], vec![WIDTH + 3]] {
        assert!(tree.prove(&indices).is_err());
    }
    assert!(Tree::new(&[]).is_err());
    let mut root = tree.root().clone();
    root.len = CAPACITY + 1;
    rejected(&root, &values, &proof);
}

#[test]
fn full_child_opening() {
    // Interpolation has degree <=4095, while Z has degree 4096: quotient is zero.
    let tree = Tree::new(
        &(0..WIDTH)
            .map(|i| (i as u64).to_be_bytes().to_vec())
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let indices = (0..WIDTH).collect::<Vec<_>>();
    let (values, proof) = tree.prove(&indices).unwrap();
    assert_eq!(proof.to_bytes().len(), 8210);
    assert!(verify(tree.root(), &values, &proof).unwrap());
    let mut bad = values;
    bad[WIDTH - 1] = b"changed".to_vec();
    rejected(tree.root(), &bad, &proof);
}

#[test]
fn canonical_big_endian_order_preserves_inputs() {
    let input = vec![vec![1, 0], vec![0xff], vec![0, 1], vec![], vec![1]];
    let original = input.clone();
    let sorted = vec![vec![], vec![0, 1], vec![1], vec![1, 0], vec![0xff]];
    let tree = Tree::new(&input).unwrap();
    let canonical = Tree::new(&sorted).unwrap();
    assert_eq!(input, original);
    assert_eq!(tree.root(), canonical.root());
    let (values, proof) = tree.prove(&[0, 1, 2, 3, 4]).unwrap();
    assert_eq!(values, sorted);
    assert_eq!(proof, canonical.prove(&[0, 1, 2, 3, 4]).unwrap().1);
    assert!(verify(tree.root(), &input, &proof).unwrap());
    assert_eq!(input, original);

    let (values, proof) = tree.prove(&[1, 3]).unwrap();
    assert_eq!(values, vec![vec![0, 1], vec![1, 0]]);
    assert!(verify(tree.root(), &[vec![1, 0], vec![0, 1]], &proof).unwrap());
}

#[test]
fn duplicate_values_are_rejected() {
    let tree = Tree::new(&[vec![], vec![1], vec![2]]).unwrap();
    let (_, proof) = tree.prove(&[0, 1, 2]).unwrap();
    // Include nonadjacent duplicates and duplicate empty values.
    for values in [
        vec![vec![1], vec![2], vec![1]],
        vec![vec![], vec![1], vec![]],
    ] {
        let original = values.clone();
        assert!(matches!(Tree::new(&values), Err(Error::DuplicateValue)));
        assert_eq!(
            verify(tree.root(), &values, &proof),
            Err(Error::DuplicateValue)
        );
        assert_eq!(values, original);
    }
}
