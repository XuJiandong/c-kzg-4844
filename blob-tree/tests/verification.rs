mod support;

#[test]
fn native_proofs_and_adversarial_inputs() {
    support::run(
        [
            include_bytes!("fixtures/single.bin").as_slice(),
            include_bytes!("fixtures/multirow.bin").as_slice(),
        ],
        |root, values, proof| {
            assert_eq!(blob_tree::verify(root, values, proof), Ok(true));
        },
    );
}

#[test]
fn large_benchmark_covers_every_child() {
    let (root, values, proof) = support::decode(include_bytes!("fixtures/large.bin"));
    assert_eq!(root.len, 100_000);
    assert_eq!(values.len(), 1_000);
    assert_eq!(
        proof.indices,
        (0..root.len).step_by(100).collect::<Vec<_>>()
    );
    let mut queries_per_child = vec![0; root.len.div_ceil(blob_tree::WIDTH)];
    for &index in &proof.indices {
        queries_per_child[index / blob_tree::WIDTH] += 1;
    }
    assert_eq!(proof.children.len(), queries_per_child.len());
    assert!(queries_per_child.iter().all(|&count| count > 0));
    assert_eq!(blob_tree::verify(&root, &values, &proof), Ok(true));
}

#[test]
fn pinned_key_matches_native_ethereum_setup() {
    assert_eq!(
        include_bytes!("../src/ethereum_g2_x.bin"),
        &include_bytes!("../../bindings/rust/src/ethereum_kzg_settings/g2_monomial_bytes.bin")
            [96..192]
    );
}
