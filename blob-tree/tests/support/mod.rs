//! The same proof fixtures and rejection checks run natively and inside CKB-VM.
extern crate alloc;

use alloc::{vec, vec::Vec};
use blob_tree::{verify, Error, Proof, Root, CAPACITY};

fn decode(mut bytes: &[u8]) -> (Root, Vec<Vec<u8>>, Proof) {
    fn number(bytes: &mut &[u8]) -> usize {
        let (head, tail) = bytes.split_at(8);
        *bytes = tail;
        usize::try_from(u64::from_be_bytes(head.try_into().unwrap())).unwrap()
    }
    let commitment = bytes[..48].try_into().unwrap();
    bytes = &bytes[48..];
    let root = Root {
        commitment,
        len: number(&mut bytes),
    };
    let count = number(&mut bytes);
    let values = (0..count)
        .map(|_| {
            let len = number(&mut bytes);
            let (value, tail) = bytes.split_at(len);
            bytes = tail;
            value.to_vec()
        })
        .collect();
    let proof = Proof::from_bytes(&root, bytes).unwrap();
    (root, values, proof)
}

fn rejected(root: &Root, values: &[Vec<u8>], proof: &Proof) {
    assert!(!matches!(verify(root, values, proof), Ok(true)));
}

/// Check the supplied native-generated membership proofs.
/// The callback measures only verification of each valid, decoded proof.
pub fn run<'a>(
    fixtures: impl IntoIterator<Item = &'a [u8]>,
    mut check_valid: impl FnMut(&Root, &[Vec<u8>], &Proof),
) {
    for fixture in fixtures {
        let (root, values, proof) = decode(fixture);
        check_valid(&root, &values, &proof);
        let encoded = proof.to_bytes();
        assert_eq!(Proof::from_bytes(&root, &encoded), Ok(proof.clone()));
        for len in 0..encoded.len() {
            assert!(Proof::from_bytes(&root, &encoded[..len]).is_err());
        }
        let mut trailing = encoded.clone();
        trailing.push(0);
        assert!(Proof::from_bytes(&root, &trailing).is_err());
        let mut oversized = encoded;
        oversized[..8].copy_from_slice(&u64::MAX.to_be_bytes());
        assert!(Proof::from_bytes(&root, &oversized).is_err());

        let mut changed = values.clone();
        changed[0].push(1);
        rejected(&root, &changed, &proof);
        rejected(&root, &values[..values.len() - 1], &proof);
        let mut bad_root = root.clone();
        bad_root.len += 1;
        rejected(&bad_root, &values, &proof);
        bad_root.len = CAPACITY + 1;
        assert_eq!(
            verify(&bad_root, &values, &proof),
            Err(Error::InvalidLength)
        );
        bad_root = root.clone();
        bad_root.commitment = proof.children[0];
        rejected(&bad_root, &values, &proof);

        let mut bad = proof.clone();
        bad.indices[0] = root.len;
        rejected(&root, &values, &bad);
        assert!(Proof::from_bytes(&root, &bad.to_bytes()).is_err());
        bad = proof.clone();
        bad.children.push(proof.children[0]);
        assert_eq!(verify(&root, &values, &bad), Err(Error::InvalidProofShape));
        bad = proof.clone();
        bad.opening = proof.quotient;
        rejected(&root, &values, &bad);
        bad = proof.clone();
        bad.quotient = proof.opening;
        rejected(&root, &values, &bad);
        bad = proof.clone();
        bad.opening = [0; 48];
        bad.opening[0] = 0xc0; // Valid G1 encoding, false opening.
        rejected(&root, &values, &bad);
        for point in [[0; 48], [0xff; 48], {
            let mut torsion = [0; 48];
            torsion[0] = 0x80; // On curve but outside the prime-order subgroup.
            torsion
        }] {
            bad = proof.clone();
            bad.quotient = point;
            assert_eq!(verify(&root, &values, &bad), Err(Error::InvalidPoint));
            assert_eq!(
                Proof::from_bytes(&root, &bad.to_bytes()),
                Err(Error::InvalidPoint)
            );
        }
        if values.len() > 1 {
            let mut reversed = values.clone();
            reversed.reverse();
            assert_eq!(verify(&root, &reversed, &proof), Ok(true));
            let mut duplicate = values.clone();
            duplicate[1] = duplicate[0].clone();
            assert_eq!(
                verify(&root, &duplicate, &proof),
                Err(Error::DuplicateValue)
            );
            if proof.children.len() > 1 {
                bad = proof.clone();
                bad.children.swap(0, 1);
                rejected(&root, &values, &bad);
            }
            for indices in [vec![], vec![0, 0], vec![1, 0]] {
                bad.indices = indices;
                rejected(&root, &values, &bad);
                assert!(Proof::from_bytes(&root, &bad.to_bytes()).is_err());
            }
        }
    }
}
