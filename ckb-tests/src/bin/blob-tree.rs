#![no_std]
#![no_main]

use ckb_std::{default_alloc, entry, syscalls::current_cycles};

#[path = "../../../blob-tree/tests/support/mod.rs"]
mod blob_tree_cases;

entry!(main);
const HEAP_SIZE: usize = 2 * 1024 * 1024;
default_alloc!(4096, HEAP_SIZE, 64);

fn main() -> i8 {
    ckb_std::logger::init().expect("logger");
    log::info!("blob-tree verification tests start");
    blob_tree_cases::run(|root, values, proof| {
        let start = current_cycles();
        let result = blob_tree::verify(root, values, proof);
        let cycles = current_cycles() - start;
        assert_eq!(result, Ok(true));
        log::info!(
            "blob-tree: {} values, {} children: {} cycles",
            values.len(),
            proof.children.len(),
            cycles
        );
    });
    log::info!("blob-tree: native proof fixtures and adversarial cases passed");
    0
}
