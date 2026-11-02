#![no_std]
#![no_main]

use ckb_std::{default_alloc, entry, env::argv, syscalls::current_cycles};

#[path = "../../../blob-tree/tests/support/mod.rs"]
mod blob_tree_cases;

entry!(main);
const HEAP_SIZE: usize = 2 * 1024 * 1024;
default_alloc!(4096, HEAP_SIZE, 64);

fn main() -> i8 {
    ckb_std::logger::init().expect("logger");
    let args = argv();
    let benchmark = args.first().is_some_and(|arg| arg.to_bytes() == b"--bench");
    let args = if benchmark { &args[1..] } else { args };
    assert!(!args.is_empty(), "usage: [--bench] <fixture hex chunks...>");
    let fixture = ckb_kzg_tests::decode(args);
    if benchmark {
        let (root, values, proof) = blob_tree_cases::decode(&fixture);
        let start = current_cycles();
        let result = blob_tree::verify(&root, &values, &proof);
        let cycles = current_cycles() - start;
        assert_eq!(result, Ok(true));
        let hundredths = (cycles + 5_000) / 10_000;
        log::info!(
            "blob-tree benchmark: {} values, {} children: {}.{:02} M cycles",
            values.len(),
            proof.children.len(),
            hundredths / 100,
            hundredths % 100
        );
    } else {
        log::info!("blob-tree verification tests start");
        blob_tree_cases::run([fixture.as_slice()], |root, values, proof| {
            assert_eq!(blob_tree::verify(root, values, proof), Ok(true));
        });
        log::info!("blob-tree: native proof fixtures and adversarial cases passed");
    }
    0
}
