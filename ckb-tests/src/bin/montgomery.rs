//! Differential tests for the RV64 field backend, executed on CKB-VM.
#![no_std]
#![no_main]

extern crate c_kzg;

use ckb_std::{default_alloc, entry, syscalls::current_cycles};
use core::hint::black_box;

entry!(main);
default_alloc!();

unsafe extern "C" {
    fn blst_mul_mont_256(out: *mut u64, a: *const u64, b: *const u64, p: *const u64, n0: u64);
    fn blst_sqr_mont_384(out: *mut u64, a: *const u64, p: *const u64, n0: u64);
    fn blst_fr_mul(out: *mut u64, a: *const u64, b: *const u64);
    fn blst_fr_sqr(out: *mut u64, a: *const u64);
    fn blst_fp_sqr(out: *mut u64, a: *const u64);
}

const FR: [u64; 4] = [
    0xffffffff00000001,
    0x53bda402fffe5bfe,
    0x3339d80809a1d805,
    0x73eda753299d7d48,
];
const FP: [u64; 6] = [
    0xb9feffffffffaaab,
    0x1eabfffeb153ffff,
    0x6730d2a0f6b0f624,
    0x64774b84f38512bf,
    0x4b1ba7b6434bacd7,
    0x1a0111ea397fe69a,
];

fn subtract<const N: usize>(a: &mut [u64; N], b: &[u64; N]) -> bool {
    let mut borrow = false;
    for i in 0..N {
        let (x, b1) = a[i].overflowing_sub(b[i]);
        let (y, b2) = x.overflowing_sub(borrow as u64);
        a[i] = y;
        borrow = b1 || b2;
    }
    borrow
}

fn reduce<const N: usize>(mut a: [u64; N], p: &[u64; N]) -> [u64; N] {
    loop {
        let mut difference = a;
        if subtract(&mut difference, p) {
            return a;
        }
        a = difference;
    }
}

fn n0(p: u64) -> u64 {
    let mut inverse = 1u64;
    for _ in 0..6 {
        inverse = inverse.wrapping_mul(2u64.wrapping_sub(p.wrapping_mul(inverse)));
    }
    inverse.wrapping_neg()
}

// Independent bit-serial reference. Compute a*b mod p using modular doubling
// and addition, then divide by 2 modulo p once for every bit in the radix.
// This deliberately shares neither CIOS nor the Comba/REDC implementation.
fn reference<const N: usize>(a: &[u64; N], b: &[u64; N], p: &[u64; N]) -> [u64; N] {
    let mut result = [0; N];
    for bit in (0..N * 64).rev() {
        result = add_mod(&result, &result, p);
        if b[bit / 64] >> (bit % 64) & 1 != 0 {
            result = add_mod(&result, a, p);
        }
    }
    for _ in 0..N * 64 {
        let mut carry = 0u128;
        if result[0] & 1 != 0 {
            for i in 0..N {
                carry += result[i] as u128 + p[i] as u128;
                result[i] = carry as u64;
                carry >>= 64;
            }
        }
        let mut top = carry as u64;
        for limb in result.iter_mut().rev() {
            let next = *limb & 1;
            *limb = (*limb >> 1) | (top << 63);
            top = next;
        }
    }
    result
}

fn add_mod<const N: usize>(a: &[u64; N], b: &[u64; N], p: &[u64; N]) -> [u64; N] {
    let mut sum = [0; N];
    let mut carry = 0u128;
    for i in 0..N {
        carry += a[i] as u128 + b[i] as u128;
        sum[i] = carry as u64;
        carry >>= 64;
    }
    let mut difference = sum;
    let borrow = subtract(&mut difference, p);
    if carry != 0 || !borrow {
        difference
    } else {
        sum
    }
}

fn sample<const N: usize>(index: usize, p: &[u64; N], seed: &mut u64) -> [u64; N] {
    let mut a = [0; N];
    match index {
        0 => {}
        1 => a[0] = 1,
        2 | 3 => {
            a = *p;
            let mut small = [0; N];
            small[0] = (index - 1) as u64;
            subtract(&mut a, &small);
        }
        4 => a.fill(u64::MAX),
        5 => a.fill(0xaaaaaaaaaaaaaaaa),
        6 => a.fill(0x5555555555555555),
        7..=18 => {
            let limb = (index - 7) / 2;
            if limb < N {
                a[limb] = if index % 2 == 0 { u64::MAX } else { 1 };
            }
        }
        _ => {
            for limb in &mut a {
                *seed ^= *seed << 13;
                *seed ^= *seed >> 7;
                *seed ^= *seed << 17;
                *limb = *seed;
            }
        }
    }
    reduce(a, p)
}

fn test_mul(p: &[u64; 4], seed: &mut u64) {
    let inverse = n0(p[0]);
    for i in 0..192 {
        let a = sample(i, p, seed);
        // Cross every boundary case with every other boundary case.
        for j in 0..if i < 19 { 19 } else { 1 } {
            let b = sample(if i < 19 { j } else { i }, p, seed);
            let expected = reference(&a, &b, p);
            let mut out = [0; 4];
            unsafe {
                blst_mul_mont_256(
                    out.as_mut_ptr(),
                    a.as_ptr(),
                    b.as_ptr(),
                    p.as_ptr(),
                    inverse,
                )
            };
            assert_eq!(out, expected);
            let mut inplace = a;
            unsafe {
                blst_mul_mont_256(
                    inplace.as_mut_ptr(),
                    inplace.as_ptr(),
                    b.as_ptr(),
                    p.as_ptr(),
                    inverse,
                )
            };
            assert_eq!(inplace, expected);
            inplace = b;
            unsafe {
                blst_mul_mont_256(
                    inplace.as_mut_ptr(),
                    a.as_ptr(),
                    inplace.as_ptr(),
                    p.as_ptr(),
                    inverse,
                )
            };
            assert_eq!(inplace, expected);
            if p == &FR {
                unsafe { blst_fr_mul(out.as_mut_ptr(), a.as_ptr(), b.as_ptr()) };
                assert_eq!(out, expected);
            }
        }
        let expected = reference(&a, &a, p);
        let mut inplace = a;
        unsafe {
            blst_mul_mont_256(
                inplace.as_mut_ptr(),
                inplace.as_ptr(),
                inplace.as_ptr(),
                p.as_ptr(),
                inverse,
            )
        };
        assert_eq!(inplace, expected);
        if p == &FR {
            inplace = a;
            unsafe { blst_fr_sqr(inplace.as_mut_ptr(), inplace.as_ptr()) };
            assert_eq!(inplace, expected);
        }
    }
    // Conversion to Montgomery form can pass an unreduced full-width input.
    // One reduced factor is sufficient for the final single subtraction.
    let a = [u64::MAX; 4];
    let mut b = *p;
    subtract(&mut b, &[1, 0, 0, 0]);
    let expected = reference(&reduce(a, p), &b, p);
    let mut out = [0; 4];
    unsafe {
        blst_mul_mont_256(
            out.as_mut_ptr(),
            a.as_ptr(),
            b.as_ptr(),
            p.as_ptr(),
            inverse,
        );
    }
    assert_eq!(out, expected);
    unsafe {
        blst_mul_mont_256(
            out.as_mut_ptr(),
            b.as_ptr(),
            a.as_ptr(),
            p.as_ptr(),
            inverse,
        );
    }
    assert_eq!(out, expected);
}

fn main() -> i8 {
    ckb_std::logger::init().expect("logger");
    let mut seed = 0x123456789abcdef0;
    test_mul(&FR, &mut seed);
    // Exercise the extra accumulator carry with a non-sparse odd modulus.
    test_mul(&[u64::MAX - 58, u64::MAX, u64::MAX, u64::MAX], &mut seed);
    for i in 0..384 {
        let a = sample(i, &FP, &mut seed);
        let expected = reference(&a, &a, &FP);
        let mut out = [0; 6];
        unsafe { blst_sqr_mont_384(out.as_mut_ptr(), a.as_ptr(), FP.as_ptr(), n0(FP[0])) };
        assert_eq!(out, expected);
        let mut inplace = a;
        unsafe {
            blst_sqr_mont_384(
                inplace.as_mut_ptr(),
                inplace.as_ptr(),
                FP.as_ptr(),
                n0(FP[0]),
            )
        };
        assert_eq!(inplace, expected);
        unsafe { blst_fp_sqr(out.as_mut_ptr(), a.as_ptr()) };
        assert_eq!(out, expected);
    }
    log::info!("RV64 Montgomery differential and aliasing tests passed");
    let mut fr = [1; 4];
    let mut fp = [1; 6];
    let start = current_cycles();
    for _ in 0..1000 {
        unsafe { blst_fr_mul(black_box(fr.as_mut_ptr()), fr.as_ptr(), fr.as_ptr()) };
    }
    log::info!(
        "Fr multiplication: {} cycles/iteration",
        (current_cycles() - start) / 1000
    );
    let start = current_cycles();
    for _ in 0..1000 {
        unsafe { blst_fp_sqr(black_box(fp.as_mut_ptr()), fp.as_ptr()) };
    }
    log::info!(
        "Fp squaring: {} cycles/iteration",
        (current_cycles() - start) / 1000
    );
    0
}
