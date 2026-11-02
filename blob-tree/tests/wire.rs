use blob_tree::{Proof, Root, CAPACITY, WIDTH};

fn proof(indices: &[usize]) -> Proof {
    let mut infinity = [0; 48];
    infinity[0] = 0xc0;
    let children = indices
        .iter()
        .enumerate()
        .filter(|&(i, index)| i == 0 || index / WIDTH != indices[i - 1] / WIDTH)
        .map(|_| infinity)
        .collect();
    Proof {
        indices: indices.to_vec(),
        children,
        quotient: infinity,
        opening: infinity,
    }
}

fn root() -> Root {
    Root {
        commitment: proof(&[0]).children[0],
        len: CAPACITY,
    }
}

#[test]
fn exact_varint_index_boundaries() {
    // Fixed byte vectors catch endian and continuation-bit mistakes independently
    // of a round trip through our encoder and decoder.
    for (index, encoded) in [
        (0, vec![0]),
        (127, vec![0x7f]),
        (128, vec![0x80, 1]),
        (16383, vec![0xff, 0x7f]),
        (16384, vec![0x80, 0x80, 1]),
        (2097151, vec![0xff, 0xff, 0x7f]),
        (2097152, vec![0x80, 0x80, 0x80, 1]),
        (CAPACITY - 1, vec![0xff, 0xff, 0xff, 7]),
    ] {
        let proof = proof(&[index]);
        let bytes = proof.to_bytes();
        assert_eq!(bytes[0], 1);
        assert_eq!(&bytes[1..1 + encoded.len()], encoded);
        assert_eq!(bytes.len(), 1 + encoded.len() + 144);
        assert_eq!(Proof::from_bytes(&root(), &bytes), Ok(proof));
    }
}

#[test]
fn varint_count_boundaries() {
    for (count, encoded) in [
        (127, vec![0x7f]),
        (128, vec![0x80, 1]),
        (16383, vec![0xff, 0x7f]),
        (16384, vec![0x80, 0x80, 1]),
    ] {
        let proof = proof(&(0..count).collect::<Vec<_>>());
        let bytes = proof.to_bytes();
        assert_eq!(&bytes[..encoded.len()], encoded);
        assert_eq!(Proof::from_bytes(&root(), &bytes), Ok(proof));
    }
}

#[test]
fn tutorial_proof_sizes() {
    for (indices, size) in [
        (vec![7], 146),
        (vec![0, 7, 77, 4095], 150),
        (vec![0, 7, 4096, 4097], 199),
        (vec![0, 4096, 8192, CAPACITY - 1], 298),
        ((0..WIDTH).collect(), 8210),
    ] {
        let proof = proof(&indices);
        let bytes = proof.to_bytes();
        assert_eq!(bytes.len(), size);
        assert_eq!(Proof::from_bytes(&root(), &bytes), Ok(proof));
    }
}

#[test]
fn reject_out_of_range_and_legacy_wire_encodings() {
    let valid = proof(&[0]);
    let bytes = valid.to_bytes();
    for header in [
        vec![0x80, 0x80, 0x80, 8],       // Maximum count, but missing indices.
        vec![0x81, 0x80, 0x80, 8],       // Count exceeds capacity.
        vec![1, 0x80, 0x80, 0x80, 8],    // Index equals capacity.
        vec![1, 0xff, 0xff, 0xff, 0x7f], // Largest four-byte varint.
        vec![1, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 2],
        [1u64.to_be_bytes(), 0u64.to_be_bytes()].concat(),
    ] {
        let mut malformed = header;
        malformed.extend_from_slice(&bytes[2..]);
        assert!(Proof::from_bytes(&root(), &malformed).is_err());
    }
    let short_root = Root { len: 1, ..root() };
    assert!(Proof::from_bytes(&short_root, &proof(&[0, 1]).to_bytes()).is_err());
}
