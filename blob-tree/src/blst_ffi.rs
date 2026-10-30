//! Minimal no_std declarations from blst/bindings/blst.h.
//! RISC-V links the BLST implementation built by c-kzg, avoiding a second C build.
#![allow(non_camel_case_types, dead_code)]

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct blst_scalar {
    pub b: [u8; 32usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct blst_fr {
    pub l: [u64; 4usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct blst_fp {
    pub l: [u64; 6usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct blst_p1 {
    pub x: blst_fp,
    pub y: blst_fp,
    pub z: blst_fp,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct blst_p1_affine {
    pub x: blst_fp,
    pub y: blst_fp,
}

extern "C" {
    pub fn blst_fr_from_uint64(ret: *mut blst_fr, a: *const u64);
    pub fn blst_scalar_from_be_bytes(out: *mut blst_scalar, in_: *const u8, len: usize) -> bool;
    pub fn blst_fr_from_scalar(ret: *mut blst_fr, a: *const blst_scalar);
    pub fn blst_scalar_from_fr(ret: *mut blst_scalar, a: *const blst_fr);
    pub fn blst_bendian_from_scalar(out: *mut u8, a: *const blst_scalar);
    pub fn blst_fr_inverse(ret: *mut blst_fr, a: *const blst_fr);
    pub fn blst_fr_add(ret: *mut blst_fr, a: *const blst_fr, b: *const blst_fr);
    pub fn blst_fr_sub(ret: *mut blst_fr, a: *const blst_fr, b: *const blst_fr);
    pub fn blst_fr_mul(ret: *mut blst_fr, a: *const blst_fr, b: *const blst_fr);
    pub fn blst_p1_uncompress(out: *mut blst_p1_affine, in_: *const u8) -> u32;
    pub fn blst_p1_from_affine(out: *mut blst_p1, in_: *const blst_p1_affine);
    pub fn blst_p1_in_g1(p: *const blst_p1) -> bool;
    pub fn blst_p1_compress(out: *mut u8, in_: *const blst_p1);
    pub fn blst_p1_add_or_double(out: *mut blst_p1, a: *const blst_p1, b: *const blst_p1);
    pub fn blst_p1_cneg(p: *mut blst_p1, cbit: bool);
    pub fn blst_p1_mult(out: *mut blst_p1, p: *const blst_p1, scalar: *const u8, nbits: usize);
    pub fn blst_p1_is_inf(p: *const blst_p1) -> bool;
    pub fn blst_p1s_to_affine(
        dst: *mut blst_p1_affine,
        points: *const *const blst_p1,
        npoints: usize,
    );
    pub fn blst_p1s_mult_pippenger_scratch_sizeof(npoints: usize) -> usize;
    pub fn blst_p1s_mult_wbits_precompute_sizeof(wbits: usize, npoints: usize) -> usize;
    pub fn blst_p1s_mult_wbits_precompute(
        table: *mut blst_p1_affine,
        wbits: usize,
        points: *const *const blst_p1_affine,
        npoints: usize,
    );
    pub fn blst_p1s_mult_wbits_scratch_sizeof(npoints: usize) -> usize;
    pub fn blst_p1s_mult_wbits(
        ret: *mut blst_p1,
        table: *const blst_p1_affine,
        wbits: usize,
        npoints: usize,
        scalars: *const *const u8,
        nbits: usize,
        scratch: *mut u64,
    );
    pub fn blst_p1s_mult_pippenger(
        ret: *mut blst_p1,
        points: *const *const blst_p1_affine,
        npoints: usize,
        scalars: *const *const u8,
        nbits: usize,
        scratch: *mut u64,
    );
}

#[cfg(all(test, not(target_arch = "riscv64")))]
mod tests {
    use super::*;
    #[test]
    fn layouts_match_upstream_blst() {
        assert_eq!(
            core::mem::size_of::<blst_scalar>(),
            core::mem::size_of::<blst::blst_scalar>()
        );
        assert_eq!(
            core::mem::align_of::<blst_scalar>(),
            core::mem::align_of::<blst::blst_scalar>()
        );
        assert_eq!(
            core::mem::size_of::<blst_fr>(),
            core::mem::size_of::<blst::blst_fr>()
        );
        assert_eq!(
            core::mem::align_of::<blst_fr>(),
            core::mem::align_of::<blst::blst_fr>()
        );
        assert_eq!(
            core::mem::size_of::<blst_fp>(),
            core::mem::size_of::<blst::blst_fp>()
        );
        assert_eq!(
            core::mem::align_of::<blst_fp>(),
            core::mem::align_of::<blst::blst_fp>()
        );
        assert_eq!(
            core::mem::size_of::<blst_p1>(),
            core::mem::size_of::<blst::blst_p1>()
        );
        assert_eq!(
            core::mem::align_of::<blst_p1>(),
            core::mem::align_of::<blst::blst_p1>()
        );
        assert_eq!(
            core::mem::size_of::<blst_p1_affine>(),
            core::mem::size_of::<blst::blst_p1_affine>()
        );
        assert_eq!(
            core::mem::align_of::<blst_p1_affine>(),
            core::mem::align_of::<blst::blst_p1_affine>()
        );
    }
}
