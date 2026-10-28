//! Small, private blst boundary. All pointers refer to initialized fixed-size objects.
#[cfg(target_arch = "riscv64")]
use crate::blst_ffi::*;
#[cfg(feature = "prover")]
use crate::WIDTH;
use crate::{Error, Result};
use alloc::{vec, vec::Vec};
#[cfg(not(target_arch = "riscv64"))]
use blst::*;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) struct F(pub blst_fr);
impl F {
    pub fn zero() -> Self {
        Self::default()
    }
    pub fn one() -> Self {
        Self::from_u64(1)
    }
    pub fn from_u64(n: u64) -> Self {
        Self::from_limbs([n, 0, 0, 0])
    }
    fn from_limbs(limbs: [u64; 4]) -> Self {
        let mut out = Self::zero();
        unsafe {
            blst_fr_from_uint64(&mut out.0, limbs.as_ptr());
        }
        out
    }
    /// Interpret a big-endian integer and reduce modulo r (no rejection sampling).
    pub fn reduce(bytes: &[u8]) -> Self {
        let mut scalar = blst_scalar::default();
        let mut out = Self::zero();
        unsafe {
            // The bool reports nonzero, not whether reduction succeeded. Zero is valid.
            blst_scalar_from_be_bytes(&mut scalar, bytes.as_ptr(), bytes.len());
            blst_fr_from_scalar(&mut out.0, &scalar);
        }
        out
    }
    pub fn bytes(self) -> [u8; 32] {
        let mut scalar = blst_scalar::default();
        let mut out = [0; 32];
        unsafe {
            blst_scalar_from_fr(&mut scalar, &self.0);
            blst_bendian_from_scalar(out.as_mut_ptr(), &scalar);
        }
        out
    }
    pub fn inv(self) -> Result<Self> {
        if self == Self::zero() {
            return Err(Error::DegenerateChallenge);
        }
        let mut out = Self::zero();
        unsafe {
            blst_fr_inverse(&mut out.0, &self.0);
        }
        Ok(out)
    }
    #[cfg(any(feature = "prover", test))]
    pub fn pow(self, mut n: usize) -> Self {
        let (mut base, mut out) = (self, Self::one());
        while n > 0 {
            if n & 1 == 1 {
                out = out * base;
            }
            base = base * base;
            n >>= 1;
        }
        out
    }
    #[cfg(any(feature = "prover", test))]
    pub fn omega() -> Self {
        // 7^((r-1)/4096), in ordinary (not Montgomery) little-endian limbs.
        Self::from_limbs([
            0xe206da11a5d36306,
            0x0ad1347b378fbf96,
            0xfc3e8acfe0f8245f,
            0x564c0a11a0f704f4,
        ])
    }
}
macro_rules! field_op {
    ($trait:ident, $method:ident, $ffi:ident) => {
        impl core::ops::$trait for F {
            type Output = Self;
            fn $method(self, rhs: Self) -> Self {
                let mut out = Self::zero();
                unsafe {
                    $ffi(&mut out.0, &self.0, &rhs.0);
                }
                out
            }
        }
    };
}
field_op!(Add, add, blst_fr_add);
field_op!(Sub, sub, blst_fr_sub);
field_op!(Mul, mul, blst_fr_mul);

pub(crate) fn bit_reverse(i: usize) -> usize {
    i.reverse_bits() >> (usize::BITS - 12)
}

// omega^(2^k), k = 0..12, precomputed in BLST's Montgomery representation.
// Each entry is (7^(((r - 1) / 4096) * 2^k) * 2^256) mod r, stored
// as four least-significant-first u64 limbs. The 384-byte table is embedded
// in the binary, so point() needs neither runtime squaring nor conversion.
const OMEGA_POWERS: [F; 12] = [
    F(blst_fr {
        l: [
            0xf2df262c09458a39,
            0x048cdf5b99dff177,
            0x16857bc5c7cce57b,
            0x043b3dbca4a915ae,
        ],
    }),
    F(blst_fr {
        l: [
            0xf4a46fbb8dc619e5,
            0x82fa97dcb8e77487,
            0xaf909c5bf198f018,
            0x00edba7d83a038fa,
        ],
    }),
    F(blst_fr {
        l: [
            0xa4a6e37a36d7822a,
            0xfc83a2ce8787e5f1,
            0xe89553dae7ea8f80,
            0x6f6d38bb2a92ff1a,
        ],
    }),
    F(blst_fr {
        l: [
            0x1ff70724b651888b,
            0xcd8166cbe56a5af4,
            0xe4a992a398e1e256,
            0x5705ea3686fd3b03,
        ],
    }),
    F(blst_fr {
        l: [
            0xb4b87dd6a9d4956c,
            0x6a51d0bcd1482f03,
            0x7e8146f7e0dbcb1e,
            0x21f4b238cccb85ae,
        ],
    }),
    F(blst_fr {
        l: [
            0xdaf6401c4aed2336,
            0xa14c0cba41de4f6a,
            0xdfdc7d04fc7f2e5b,
            0x6f61ff4e296cff18,
        ],
    }),
    F(blst_fr {
        l: [
            0xbcf7e31ae8d6f162,
            0x15d1c587662fc7a4,
            0xb7ed5ecd71278a57,
            0x329d0d9340ccecb3,
        ],
    }),
    F(blst_fr {
        l: [
            0x30c78850dd04d52f,
            0xe27f8794adaddbcb,
            0x0a59f321ae22c531,
            0x1958b127ba7f8ca3,
        ],
    }),
    F(blst_fr {
        l: [
            0x1acf64e488202b65,
            0x5b18b637819f004f,
            0xee649cab3107fdf5,
            0x27a8543dce9012a0,
        ],
    }),
    F(blst_fr {
        l: [
            0x6ecf49952c42eff8,
            0x6ec1fa834d5dfa57,
            0x3ff5cf385656b15c,
            0x54b80267229e031b,
        ],
    }),
    F(blst_fr {
        l: [
            0xf3b05674aa89cfb1,
            0x072f01406006b9fe,
            0xce9a0dbf25667a26,
            0x4d2ce4052d598374,
        ],
    }),
    F(blst_fr {
        l: [
            0xfffffffd00000003,
            0xfb38ec08fffb13fc,
            0x99ad88181ce5880f,
            0x5bc8f5f97cd877d8,
        ],
    }),
];

pub(crate) fn point(i: usize) -> F {
    let mut exponent = bit_reverse(i);
    if exponent == 0 {
        return F::one();
    }
    // Start with the first selected power to avoid multiplying by one.
    let mut out = OMEGA_POWERS[exponent.trailing_zeros() as usize];
    exponent &= exponent - 1;
    while exponent != 0 {
        out = out * OMEGA_POWERS[exponent.trailing_zeros() as usize];
        exponent &= exponent - 1;
    }
    out
}

/// In-place radix-2 FFT. Input and output are in natural order.
#[cfg(feature = "prover")]
fn fft(a: &mut [F], inverse: bool) {
    assert_eq!(a.len(), WIDTH);
    for i in 0..WIDTH {
        let j = bit_reverse(i);
        if i < j {
            a.swap(i, j);
        }
    }
    let root = if inverse {
        F::omega().inv().unwrap()
    } else {
        F::omega()
    };
    let mut size = 2;
    while size <= WIDTH {
        let step = root.pow(WIDTH / size);
        for chunk in a.chunks_exact_mut(size) {
            let mut w = F::one();
            for j in 0..size / 2 {
                let (u, v) = (chunk[j], chunk[j + size / 2] * w);
                chunk[j] = u + v;
                chunk[j + size / 2] = u - v;
                w = w * step;
            }
        }
        size *= 2;
    }
    if inverse {
        let scale = F::from_u64(WIDTH as u64).inv().unwrap();
        for v in a {
            *v = *v * scale;
        }
    }
}
#[cfg(feature = "prover")]
pub(crate) fn coefficients(values: &[F]) -> Vec<F> {
    let mut a = (0..WIDTH)
        .map(|i| values[bit_reverse(i)])
        .collect::<Vec<_>>();
    fft(&mut a, true);
    a
}
#[cfg(feature = "prover")]
pub(crate) fn blob(poly: &[F]) -> c_kzg::Blob {
    assert!(poly.len() <= WIDTH);
    let mut a = poly.to_vec();
    a.resize(WIDTH, F::zero());
    fft(&mut a, false);
    let bytes = (0..WIDTH)
        .flat_map(|i| a[bit_reverse(i)].bytes())
        .collect::<Vec<_>>();
    c_kzg::Blob::from_bytes(&bytes).expect("fixed-size canonical field elements")
}
pub(crate) fn eval(poly: &[F], x: F) -> F {
    poly.iter().rev().fold(F::zero(), |v, c| v * x + *c)
}
pub(crate) fn add_scaled(out: &mut [F], poly: &[F], scale: F) {
    assert!(poly.len() <= out.len());
    for (a, b) in out.iter_mut().zip(poly) {
        *a = *a + *b * scale;
    }
}
/// Newton interpolation, plus the monic vanishing polynomial. Points must be distinct.
pub(crate) fn interpolate(points: &[(F, F)]) -> Result<(Vec<F>, Vec<F>)> {
    let Some((&(x, y), rest)) = points.split_first() else {
        return Ok((Vec::new(), vec![F::one()]));
    };
    // For the first point I(X) = y and Z(X) = X - x; no inversion is needed.
    let mut interpolation = vec![F::zero(); points.len()];
    interpolation[0] = y;
    let mut vanishing = vec![F::zero() - x, F::one()];
    for &(x, y) in rest {
        let delta = (y - eval(&interpolation, x)) * eval(&vanishing, x).inv()?;
        add_scaled(&mut interpolation, &vanishing, delta);
        let mut next = vec![F::zero(); vanishing.len() + 1];
        for (i, c) in vanishing.iter().enumerate() {
            next[i] = next[i] - *c * x;
            next[i + 1] = next[i + 1] + *c;
        }
        vanishing = next;
    }
    Ok((interpolation, vanishing))
}
/// Exact division by a monic polynomial, including the all-4096-points case.
#[cfg(feature = "prover")]
pub(crate) fn quotient(poly: &[F], interpolation: &[F], vanishing: &[F]) -> Result<Vec<F>> {
    let mut remainder = poly.to_vec();
    add_scaled(&mut remainder, interpolation, F::zero() - F::one());
    let degree = vanishing.len() - 1;
    let mut q = vec![F::zero(); poly.len().saturating_sub(degree)];
    for i in (degree..poly.len()).rev() {
        let c = remainder[i];
        q[i - degree] = c;
        for j in 0..=degree {
            remainder[i - degree + j] = remainder[i - degree + j] - c * vanishing[j];
        }
    }
    if remainder.iter().any(|x| *x != F::zero()) {
        return Err(Error::InvalidClaim);
    }
    Ok(q)
}

#[derive(Clone, Copy, Default)]
pub(crate) struct G(blst_p1);
impl G {
    pub fn decode(bytes: &[u8; 48]) -> Result<Self> {
        let mut affine = blst_p1_affine::default();
        let mut out = Self::default();
        unsafe {
            if blst_p1_uncompress(&mut affine, bytes.as_ptr()) as u32 != 0 {
                return Err(Error::InvalidPoint);
            }
            blst_p1_from_affine(&mut out.0, &affine);
            if !blst_p1_in_g1(&out.0) {
                return Err(Error::InvalidPoint);
            }
        }
        if out.bytes() != *bytes {
            return Err(Error::InvalidPoint);
        }
        Ok(out) // Infinity is a valid commitment to the zero polynomial.
    }
    pub fn bytes(self) -> [u8; 48] {
        let mut out = [0; 48];
        unsafe {
            blst_p1_compress(out.as_mut_ptr(), &self.0);
        }
        out
    }
    pub fn add(self, rhs: Self) -> Self {
        let mut out = Self::default();
        unsafe {
            blst_p1_add_or_double(&mut out.0, &self.0, &rhs.0);
        }
        out
    }
    pub fn neg(mut self) -> Self {
        unsafe {
            blst_p1_cneg(&mut self.0, true);
        }
        self
    }
    pub fn scale(self, scalar: F) -> Self {
        let mut bytes = scalar.bytes();
        bytes.reverse();
        let mut out = Self::default();
        unsafe {
            blst_p1_mult(&mut out.0, &self.0, bytes.as_ptr(), 255);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_domain_points_match_exponentiation() {
        let omega = F::omega();
        for (k, &power) in OMEGA_POWERS.iter().enumerate() {
            assert_eq!(power, omega.pow(1 << k), "power {k}");
        }
        for i in 0..crate::WIDTH {
            assert_eq!(point(i), omega.pow(bit_reverse(i)), "point {i}");
        }
    }

    #[test]
    fn negation_matches_scalar_multiplication() {
        let commitment = include_bytes!("../tests/fixtures/single.bin")[..48]
            .try_into()
            .unwrap();
        let point = G::decode(commitment).unwrap();
        for p in [G::default(), point, point.scale(F::from_u64(7))] {
            assert_eq!(p.neg().bytes(), p.scale(F::zero() - F::one()).bytes());
            assert_eq!(p.neg().neg().bytes(), p.bytes());
            assert_eq!(p.add(p.neg()).bytes(), G::default().bytes());
        }
    }

    #[test]
    fn interpolation_handles_empty_singleton_and_multiple_points() {
        assert_eq!(interpolate(&[]).unwrap(), (vec![], vec![F::one()]));
        for count in 1..=5 {
            let poly = (1..=count).map(F::from_u64).collect::<Vec<_>>();
            let points = (0..count)
                .map(|i| {
                    let x = F::from_u64(i);
                    (x, eval(&poly, x))
                })
                .collect::<Vec<_>>();
            let (interpolation, vanishing) = interpolate(&points).unwrap();
            assert_eq!(interpolation, poly);
            for i in 0..=count {
                let x = F::from_u64(i);
                let expected = points.iter().fold(F::one(), |v, (root, _)| v * (x - *root));
                assert_eq!(eval(&vanishing, x), expected);
            }
            assert_eq!(
                interpolate(&[points[0], points[0]]),
                Err(Error::DegenerateChallenge)
            );
        }
    }

    #[cfg(feature = "prover")]
    #[test]
    fn field_domain_fft_and_ckzg_agree() {
        let r = [
            0x73, 0xed, 0xa7, 0x53, 0x29, 0x9d, 0x7d, 0x48, 0x33, 0x39, 0xd8, 0x08, 0x09, 0xa1,
            0xd8, 0x05, 0x53, 0xbd, 0xa4, 0x02, 0xff, 0xfe, 0x5b, 0xfe, 0xff, 0xff, 0xff, 0xff,
            0x00, 0x00, 0x00, 0x01,
        ];
        assert_eq!(F::reduce(&r), F::zero());
        assert_eq!(F::omega().pow(WIDTH), F::one());
        assert_ne!(F::omega().pow(WIDTH / 2), F::one());
        assert_eq!(point(WIDTH - 1), F::omega().pow(WIDTH - 1));
        let values = (0..WIDTH)
            .map(|i| F::from_u64((i * i + 9) as u64))
            .collect::<Vec<_>>();
        let poly = coefficients(&values);
        let blob = blob(&poly);
        for i in [0, 1, 2, 77, 2048, 4095] {
            assert_eq!(eval(&poly, point(i)), values[i]);
            let (_, y) = c_kzg::ethereum_kzg_settings(0)
                .compute_kzg_proof(&blob, &c_kzg::Bytes32::from(point(i).bytes()))
                .unwrap();
            assert_eq!(*y.as_ref(), values[i].bytes());
        }
        assert!(F::zero().inv().is_err());
    }
    #[cfg(feature = "prover")]
    #[test]
    fn interpolation_division_and_subgroup_validation() {
        let poly = vec![F::from_u64(3), F::from_u64(5), F::from_u64(7)];
        let points = [F::from_u64(2), F::from_u64(4)].map(|x| (x, eval(&poly, x)));
        let (i, z) = interpolate(&points).unwrap();
        let q = quotient(&poly, &i, &z).unwrap();
        for x in [F::zero(), F::one(), F::from_u64(17)] {
            assert_eq!(eval(&poly, x) - eval(&i, x), eval(&q, x) * eval(&z, x));
        }
        assert!(interpolate(&[points[0], points[0]]).is_err());
        // Compressed (x=0,y=2) is on the curve but has order 3, outside G1.
        let mut torsion = [0; 48];
        torsion[0] = 0x80;
        assert!(G::decode(&torsion).is_err());
        assert_eq!(
            G::decode(&G::default().bytes()).unwrap().bytes(),
            G::default().bytes()
        );
    }
}
