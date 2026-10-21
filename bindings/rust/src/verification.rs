//! Verification-only EIP-4844 API, usable with no_std + alloc.
use core::{ffi::c_void, ptr::NonNull};

pub const FIELD_ELEMENTS_PER_BLOB: usize = 4096;
pub const BYTES_PER_FIELD_ELEMENT: usize = 32;
pub const BYTES_PER_BLOB: usize = FIELD_ELEMENTS_PER_BLOB * BYTES_PER_FIELD_ELEMENT;
pub const BYTES_PER_COMMITMENT: usize = 48;
pub const BYTES_PER_PROOF: usize = 48;
pub type Blob = [u8; BYTES_PER_BLOB];
pub type Bytes32 = [u8; 32];
pub type Bytes48 = [u8; 48];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    BadArguments,
    AllocationFailed,
    MismatchLength,
    Internal,
}

/// An immutable EIP-4844 verification context. Requires a global allocator.
///
/// The caller must authenticate the compressed [s]G2 key against the intended
/// trusted setup. Checking that it is a valid subgroup point does not establish
/// trust. No setup files, G1 tables, or proving APIs are needed.
pub struct KzgVerifier(NonNull<c_void>);

extern "C" {
    fn ckzg_verifier_new(out: *mut *mut c_void, g2_x: *const u8) -> i32;
    fn ckzg_verifier_free(context: *mut c_void);
    fn verify_kzg_proof(
        ok: *mut bool,
        commitment: *const Bytes48,
        z: *const Bytes32,
        y: *const Bytes32,
        proof: *const Bytes48,
        settings: *const c_void,
    ) -> i32;
    fn verify_blob_kzg_proof(
        ok: *mut bool,
        blob: *const Blob,
        commitment: *const Bytes48,
        proof: *const Bytes48,
        settings: *const c_void,
    ) -> i32;
    fn verify_blob_kzg_proof_batch(
        ok: *mut bool,
        blobs: *const Blob,
        commitments: *const Bytes48,
        proofs: *const Bytes48,
        count: u64,
        settings: *const c_void,
    ) -> i32;
}

fn result(code: i32, valid: bool) -> Result<bool, Error> {
    match code {
        0 => Ok(valid),
        1 => Err(Error::BadArguments),
        3 => Err(Error::AllocationFailed),
        _ => Err(Error::Internal),
    }
}

impl KzgVerifier {
    pub fn new(g2_x: &[u8; 96]) -> Result<Self, Error> {
        let mut context = core::ptr::null_mut();
        // C owns the allocation, validates the key, and puts KZGSettings first.
        result(
            unsafe { ckzg_verifier_new(&mut context, g2_x.as_ptr()) },
            true,
        )?;
        NonNull::new(context).map(Self).ok_or(Error::Internal)
    }

    /// Verify an evaluation y = p(z). Malformed encodings return an error;
    /// well-formed but incorrect proofs return Ok(false).
    pub fn verify_kzg_proof(
        &self,
        commitment: &Bytes48,
        z: &Bytes32,
        y: &Bytes32,
        proof: &Bytes48,
    ) -> Result<bool, Error> {
        let mut valid = false;
        let code =
            unsafe { verify_kzg_proof(&mut valid, commitment, z, y, proof, self.0.as_ptr()) };
        result(code, valid)
    }

    pub fn verify_blob_kzg_proof(
        &self,
        blob: &Blob,
        commitment: &Bytes48,
        proof: &Bytes48,
    ) -> Result<bool, Error> {
        let mut valid = false;
        let code =
            unsafe { verify_blob_kzg_proof(&mut valid, blob, commitment, proof, self.0.as_ptr()) };
        result(code, valid)
    }

    pub fn verify_blob_kzg_proof_batch(
        &self,
        blobs: &[Blob],
        commitments: &[Bytes48],
        proofs: &[Bytes48],
    ) -> Result<bool, Error> {
        if blobs.len() != commitments.len() || blobs.len() != proofs.len() {
            return Err(Error::MismatchLength);
        }
        let mut valid = false;
        let code = unsafe {
            verify_blob_kzg_proof_batch(
                &mut valid,
                blobs.as_ptr(),
                commitments.as_ptr(),
                proofs.as_ptr(),
                blobs.len() as u64,
                self.0.as_ptr(),
            )
        };
        result(code, valid)
    }
}

impl Drop for KzgVerifier {
    fn drop(&mut self) {
        unsafe { ckzg_verifier_free(self.0.as_ptr()) }
    }
}
