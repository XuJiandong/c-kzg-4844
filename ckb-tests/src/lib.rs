#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use ckb_std::env::Arg;

/// Hex keeps binary input (including NUL bytes) safe for argv. Multiple chunks
/// avoid the host OS's per-argument size limit when passing blobs.
pub fn decode(args: &[Arg]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(args.iter().map(|arg| arg.to_bytes().len() / 2).sum());
    for arg in args {
        let hex = arg.to_bytes();
        assert_eq!(hex.len() % 2, 0, "odd hex length");
        for pair in hex.chunks_exact(2) {
            let digit = |c| match c {
                b'0'..=b'9' => c - b'0',
                b'a'..=b'f' => c - b'a' + 10,
                b'A'..=b'F' => c - b'A' + 10,
                _ => panic!("invalid hex digit"),
            };
            bytes.push(digit(pair[0]) * 16 + digit(pair[1]));
        }
    }
    bytes
}

pub fn take<'a>(bytes: &mut &'a [u8], len: usize) -> &'a [u8] {
    assert!(bytes.len() >= len, "truncated input");
    let (head, tail) = bytes.split_at(len);
    *bytes = tail;
    head
}

pub fn array<'a, const N: usize>(bytes: &mut &'a [u8]) -> &'a [u8; N] {
    take(bytes, N).try_into().unwrap()
}

pub fn arrays<'a, const N: usize>(bytes: &mut &'a [u8]) -> &'a [[u8; N]] {
    let count = usize::try_from(u64::from_be_bytes(*array(bytes))).unwrap();
    let len = count.checked_mul(N).expect("input length overflow");
    // Borrow the decoded payload instead of copying entire blobs again.
    take(bytes, len).as_chunks::<N>().0
}
