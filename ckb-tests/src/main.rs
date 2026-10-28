#![no_std]
#![no_main]

use c_kzg::{Error, KzgVerifier, BYTES_PER_BLOB};
use ckb_std::{default_alloc, entry, syscalls::current_cycles};

entry!(main);
const HEAP_SIZE: usize = 2 * 1024 * 1024;
default_alloc!(4096, HEAP_SIZE, 64);

const G2: &[u8] =
    include_bytes!("../../bindings/rust/src/ethereum_kzg_settings/g2_monomial_bytes.bin");
macro_rules! fixture {
    ($name:literal) => {
        include_bytes!(concat!(env!("OUT_DIR"), "/", $name, ".bin"))
    };
}
static NONZERO_BLOB: [u8; BYTES_PER_BLOB] = *fixture!("blob_blob");

fn main() -> i8 {
    ckb_std::logger::init().expect("logger");
    // ckb-debugger forwards arguments after `--` directly, without argv[0].
    let mode = ckb_std::env::argv()
        .first()
        .map(|x| x.to_bytes())
        .unwrap_or(b"all");
    log::info!("EIP-4844 verification tests start");
    let key: &[u8; 96] = G2[96..192].try_into().unwrap();
    assert!(matches!(
        KzgVerifier::new(&[0; 96]),
        Err(Error::BadArguments)
    ));
    let verifier = KzgVerifier::new(key).expect("verification context");
    let mut infinity = [0u8; 48];
    infinity[0] = 0xc0;
    let zero = [0u8; 32];
    if mode == b"all" || mode == b"point" {
        assert_eq!(
            verifier.verify_kzg_proof(&infinity, &zero, &zero, &infinity),
            Ok(true)
        );
        let mut one = zero;
        one[31] = 1;
        assert_eq!(
            verifier.verify_kzg_proof(&infinity, &zero, &one, &infinity),
            Ok(false)
        );
        assert_eq!(
            verifier.verify_kzg_proof(&[0; 48], &zero, &zero, &infinity),
            Err(Error::BadArguments)
        );
        assert_eq!(
            verifier.verify_kzg_proof(&infinity, &[255; 32], &zero, &infinity),
            Err(Error::BadArguments)
        );
        // Measure one nonzero valid proof, excluding setup, assertions, and logging.
        let start_cycles = current_cycles();
        let result = verifier.verify_kzg_proof(
            fixture!("point_commitment"),
            fixture!("point_z"),
            fixture!("point_y"),
            fixture!("point_proof"),
        );
        let cycles = current_cycles() - start_cycles;
        assert_eq!(result, Ok(true));
        log::info!(
            "verify_kzg_proof: {} M cycles (one nonzero valid proof)",
            (cycles + 500_000) / 1_000_000
        );
        assert_eq!(
            verifier.verify_kzg_proof(
                fixture!("point_commitment"),
                fixture!("point_z"),
                &zero,
                fixture!("point_proof")
            ),
            Ok(false)
        );
        log::info!(
            "point: nonzero reference vector, valid, incorrect, malformed point and scalar passed"
        );
    }
    if mode == b"all" || mode == b"blob" {
        let zero_blob = alloc::vec![0u8; BYTES_PER_BLOB];
        let zero_blob: &[u8; BYTES_PER_BLOB] = zero_blob.as_slice().try_into().unwrap();
        assert_eq!(
            verifier.verify_blob_kzg_proof(zero_blob, &infinity, &infinity),
            Ok(true)
        );
        let start_cycles = current_cycles();
        let result = verifier.verify_blob_kzg_proof(
            &NONZERO_BLOB,
            fixture!("blob_commitment"),
            fixture!("blob_proof"),
        );
        let cycles = current_cycles() - start_cycles;
        assert_eq!(result, Ok(true));
        log::info!(
            "verify_blob_kzg_proof: {} M cycles (one nonzero valid blob)",
            (cycles + 500_000) / 1_000_000
        );
        assert_eq!(
            verifier.verify_blob_kzg_proof(&NONZERO_BLOB, &infinity, fixture!("blob_proof")),
            Ok(false)
        );
        log::info!("blob: zero and nonzero reference vector, incorrect commitment passed");
    }
    if mode == b"all" || mode == b"batch" {
        assert_eq!(
            verifier.verify_blob_kzg_proof_batch(&[], &[], &[]),
            Ok(true)
        );
        let mut blobs = alloc::vec::Vec::with_capacity(2);
        blobs.extend_from_slice(core::slice::from_ref(&NONZERO_BLOB));
        blobs.extend_from_slice(core::slice::from_ref(&NONZERO_BLOB));
        assert_eq!(
            verifier.verify_blob_kzg_proof_batch(&blobs, &[infinity], &[infinity]),
            Err(Error::MismatchLength)
        );
        let commitments = [*fixture!("blob_commitment"); 2];
        let proofs = [*fixture!("blob_proof"); 2];
        assert_eq!(
            verifier.verify_blob_kzg_proof_batch(&blobs, &commitments, &proofs),
            Ok(true)
        );
        assert_eq!(
            verifier.verify_blob_kzg_proof_batch(&blobs, &commitments, &[proofs[0], infinity]),
            Ok(false)
        );
        log::info!("batch: empty, mismatched, nonzero and incorrect batches passed");
    }
    assert!(
        matches!(mode, b"all" | b"point" | b"blob" | b"batch"),
        "unknown test"
    );
    log::info!("all selected tests passed");
    0
}
