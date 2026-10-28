mod support;

#[test]
fn native_proofs_and_adversarial_inputs() {
    support::run(|root, values, proof| {
        assert_eq!(blob_tree::verify(root, values, proof), Ok(true));
    });
}

#[test]
fn pinned_key_matches_native_ethereum_setup() {
    assert_eq!(
        include_bytes!("../src/ethereum_g2_x.bin"),
        &include_bytes!("../../bindings/rust/src/ethereum_kzg_settings/g2_monomial_bytes.bin")
            [96..192]
    );
}
