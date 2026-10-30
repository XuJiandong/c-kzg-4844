#![no_std]
#![no_main]

use c_kzg::{Error, KzgVerifier, BYTES_PER_BLOB};
use ckb_kzg_tests::{array, arrays, decode};
use ckb_std::{default_alloc, entry, env::argv, syscalls::current_cycles};

entry!(main);
const HEAP_SIZE: usize = 2 * 1024 * 1024;
default_alloc!(4096, HEAP_SIZE, 64);

fn main() -> i8 {
    ckb_std::logger::init().expect("logger");
    // ckb-debugger forwards arguments after `--` without an argv[0].
    let args = argv();
    assert!(
        args.len() >= 3,
        "usage: <key|point|blob|batch> <expected> <hex chunks...>"
    );
    let mode = args[0].to_bytes();
    let expected = match args[1].to_bytes() {
        b"true" => Ok(true),
        b"false" => Ok(false),
        b"bad-arguments" => Err(Error::BadArguments),
        b"mismatch-length" => Err(Error::MismatchLength),
        _ => panic!("unknown expected result"),
    };
    let data = decode(&args[2..]);
    let mut input = data.as_slice();
    let key = array(&mut input);
    let result = if mode == b"key" {
        KzgVerifier::new(key).map(|_| true)
    } else {
        let verifier = KzgVerifier::new(key).expect("verification context");
        let (result, cycles) = match mode {
            b"point" => {
                let commitment = array(&mut input);
                let z = array(&mut input);
                let y = array(&mut input);
                let proof = array(&mut input);
                let start = current_cycles();
                let result = verifier.verify_kzg_proof(commitment, z, y, proof);
                (result, current_cycles() - start)
            }
            b"blob" => {
                let blob = array(&mut input);
                let commitment = array(&mut input);
                let proof = array(&mut input);
                let start = current_cycles();
                let result = verifier.verify_blob_kzg_proof(blob, commitment, proof);
                (result, current_cycles() - start)
            }
            b"batch" => {
                let blobs = arrays::<BYTES_PER_BLOB>(&mut input);
                let commitments = arrays(&mut input);
                let proofs = arrays(&mut input);
                let start = current_cycles();
                let result = verifier.verify_blob_kzg_proof_batch(blobs, commitments, proofs);
                (result, current_cycles() - start)
            }
            _ => panic!("unknown operation"),
        };
        log::info!("{}: {} cycles", args[0].to_str().unwrap(), cycles);
        result
    };
    assert!(input.is_empty(), "trailing input");
    assert_eq!(result, expected);
    log::info!("expected result: {:?}", result);
    0
}
