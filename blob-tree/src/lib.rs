//! Blob Tree: a two-level EIP-4844 vector commitment with a Shplonk-style multiproof.
//! See DESIGN.md for the protocol, transcript and security assumptions.
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(all(feature = "prover", feature = "verification-only"))]
compile_error!("verification-only requires --no-default-features (it excludes prover)");
#[cfg(not(any(feature = "prover", feature = "verification-only")))]
compile_error!("enable either prover or verification-only");

mod algebra;
#[cfg(any(target_arch = "riscv64", test))]
mod blst_ffi;
use algebra::*;
#[cfg(feature = "verification-only")]
use alloc::format;
use alloc::{collections::BTreeMap, string::String, vec::Vec};
#[cfg(feature = "prover")]
use alloc::{string::ToString, vec};
#[cfg(feature = "prover")]
use c_kzg::{ethereum_kzg_settings, Bytes32, Bytes48};
use sha2::{Digest, Sha256};

pub const WIDTH: usize = 4096;
pub const CAPACITY: usize = WIDTH * WIDTH;
pub type Commitment = [u8; 48];
pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidLength,
    DuplicateValue,
    InvalidQuery,
    InvalidClaim,
    InvalidPoint,
    InvalidProofShape,
    DegenerateChallenge,
    Kzg(String),
}
impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl core::error::Error for Error {}
impl From<c_kzg::Error> for Error {
    fn from(e: c_kzg::Error) -> Self {
        #[cfg(feature = "prover")]
        let message = e.to_string();
        #[cfg(feature = "verification-only")]
        let message = format!("{e:?}");
        Self::Kzg(message)
    }
}

/// The complete public root. Both fields must come from a trusted source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Root {
    pub commitment: Commitment,
    pub len: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proof {
    /// Nonempty, strictly ascending positions in the sorted tree.
    /// Paired with the separately supplied values after sorting those values.
    pub indices: Vec<usize>,
    /// One per queried child, in ascending child-index order. No redundant indices.
    pub children: Vec<Commitment>,
    /// Commitment to the aggregated quotient polynomial.
    pub quotient: Commitment,
    /// Single KZG opening witness at the transcript challenge.
    pub opening: Commitment,
}
impl Proof {
    /// Canonical wire format: u64 count || u64 indices || children || quotient || opening.
    /// Integers are big-endian; child commitments are inferred from the indices.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(self.indices.len() as u64).to_be_bytes());
        for &index in &self.indices {
            bytes.extend_from_slice(&(index as u64).to_be_bytes());
        }
        for point in self.children.iter().chain([&self.quotient, &self.opening]) {
            bytes.extend_from_slice(point);
        }
        bytes
    }
    /// Decode indices and points, rejecting malformed queries and noncanonical encodings.
    pub fn from_bytes(root: &Root, bytes: &[u8]) -> Result<Self> {
        let read_usize = |bytes: &[u8]| {
            usize::try_from(u64::from_be_bytes(bytes.try_into().unwrap()))
                .map_err(|_| Error::InvalidProofShape)
        };
        let count = read_usize(bytes.get(..8).ok_or(Error::InvalidProofShape)?)?;
        // Bound the count before allocating or computing an offset from untrusted bytes.
        if count == 0 || count > CAPACITY {
            return Err(Error::InvalidQuery);
        }
        let points_offset = 8 + 8 * count;
        let indices = bytes
            .get(8..points_offset)
            .ok_or(Error::InvalidProofShape)?
            .chunks_exact(8)
            .map(read_usize)
            .collect::<Result<Vec<_>>>()?;
        let groups = validate(root, &indices)?;
        if bytes.len() != points_offset + 48 * (groups.len() + 2) {
            return Err(Error::InvalidProofShape);
        }
        let mut points = bytes[points_offset..]
            .chunks_exact(48)
            .map(|c| c.try_into().unwrap())
            .collect::<Vec<Commitment>>();
        for p in &points {
            G::decode(p)?;
        }
        let opening = points.pop().unwrap();
        let quotient = points.pop().unwrap();
        Ok(Self {
            indices,
            children: points,
            quotient,
            opening,
        })
    }
}

/// Immutable PoC tree. Stores polynomial coefficients for straightforward proving.
#[cfg(feature = "prover")]
pub struct Tree {
    root: Root,
    top: Vec<F>,
    children: Vec<(Commitment, Vec<F>)>,
    values: Vec<Vec<u8>>,
}
fn hash_field(domain: &[u8], parts: &[&[u8]]) -> F {
    let mut hash = Sha256::new();
    hash.update(b"blob-tree/v1/");
    hash.update((domain.len() as u64).to_be_bytes());
    hash.update(domain);
    for part in parts {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part);
    }
    F::reduce(&hash.finalize())
}
fn leaf(value: &[u8]) -> F {
    hash_field(b"leaf", &[value])
}
fn child_hash(commitment: &Commitment) -> F {
    hash_field(b"child", &[commitment])
}
#[cfg(feature = "prover")]
fn commit(poly: &[F]) -> Result<Commitment> {
    Ok(ethereum_kzg_settings(0)
        .blob_to_kzg_commitment(&blob(poly))?
        .to_bytes()
        .into_inner())
}
#[cfg(feature = "prover")]
impl Tree {
    /// Build a tree from values sorted in ascending big-endian byte order.
    /// Comparison is lexicographic (unsigned bytes, shorter prefixes first).
    /// Duplicate values are rejected, and the caller's slice is not modified.
    pub fn new(values: &[Vec<u8>]) -> Result<Self> {
        if values.is_empty() || values.len() > CAPACITY {
            return Err(Error::InvalidLength);
        }
        let mut values = values.to_vec();
        values.sort_unstable();
        if values.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(Error::DuplicateValue);
        }
        let mut children = Vec::new();
        let mut top_values = vec![F::zero(); WIDTH];
        for (row, chunk) in values.chunks(WIDTH).enumerate() {
            let mut evaluations = vec![F::zero(); WIDTH];
            for (dst, value) in evaluations.iter_mut().zip(chunk) {
                *dst = leaf(value);
            }
            let poly = coefficients(&evaluations);
            let commitment = commit(&poly)?;
            top_values[row] = child_hash(&commitment);
            children.push((commitment, poly));
        }
        let top = coefficients(&top_values);
        let root = Root {
            commitment: commit(&top)?,
            len: values.len(),
        };
        Ok(Self {
            root,
            top,
            children,
            values,
        })
    }
    pub fn root(&self) -> &Root {
        &self.root
    }
    /// Return queried values in sorted order and a proof containing their positions.
    /// Indices refer to the sorted tree, not the original input order.
    /// Indices must be nonempty, unique, ascending and in range.
    pub fn prove(&self, indices: &[usize]) -> Result<(Vec<Vec<u8>>, Proof)> {
        if indices.is_empty()
            || indices.len() > self.root.len
            || indices.windows(2).any(|w| w[0] >= w[1])
            || indices.iter().any(|i| *i >= self.root.len)
        {
            return Err(Error::InvalidQuery);
        }
        let values = indices
            .iter()
            .map(|&index| self.values[index].clone())
            .collect::<Vec<_>>();
        let groups = validate(&self.root, indices)?;
        let children = groups
            .keys()
            .map(|&i| self.children[i].0)
            .collect::<Vec<_>>();
        let statements = statements(&self.root, indices, &values, &children)?;
        let transcript = transcript(&self.root, indices, &values, &children);
        let rho = rho(&transcript)?;
        let polys = core::iter::once(self.top.as_slice())
            .chain(groups.keys().map(|&i| self.children[i].1.as_slice()))
            .collect::<Vec<_>>();
        let mut q = vec![F::zero(); WIDTH];
        let mut power = F::one();
        for (poly, statement) in polys.iter().zip(&statements) {
            let (interpolation, vanishing) = interpolate(&statement.points)?;
            add_scaled(&mut q, &quotient(poly, &interpolation, &vanishing)?, power);
            power = power * rho;
        }
        let quotient = commit(&q)?;
        let z = challenge(&transcript, &quotient, &statements)?;
        let (weights, y) = weights(&statements, rho, z)?;
        let mut combined = vec![F::zero(); WIDTH];
        for (poly, weight) in polys.iter().zip(weights) {
            add_scaled(&mut combined, poly, weight);
        }
        add_scaled(&mut combined, &q, F::zero() - F::one());
        let (opening, actual_y) = ethereum_kzg_settings(0)
            .compute_kzg_proof(&blob(&combined), &Bytes32::from(z.bytes()))?;
        if *actual_y.as_ref() != y.bytes() {
            return Err(Error::InvalidClaim);
        }
        Ok((
            values,
            Proof {
                indices: indices.to_vec(),
                children,
                quotient,
                opening: opening.to_bytes().into_inner(),
            },
        ))
    }
}

fn validate(root: &Root, indices: &[usize]) -> Result<BTreeMap<usize, Vec<usize>>> {
    if root.len == 0 || root.len > CAPACITY {
        return Err(Error::InvalidLength);
    }
    if indices.is_empty() || indices.len() > root.len || indices.windows(2).any(|w| w[0] >= w[1]) {
        return Err(Error::InvalidQuery);
    }
    let mut groups = BTreeMap::<usize, Vec<usize>>::new();
    for (position, &index) in indices.iter().enumerate() {
        if index >= root.len {
            return Err(Error::InvalidQuery);
        }
        groups.entry(index / WIDTH).or_default().push(position);
    }
    Ok(groups)
}
struct Statement {
    commitment: Commitment,
    points: Vec<(F, F)>,
}
fn statements(
    root: &Root,
    indices: &[usize],
    values: &[Vec<u8>],
    children: &[Commitment],
) -> Result<Vec<Statement>> {
    let groups = validate(root, indices)?;
    if values.len() != indices.len() {
        return Err(Error::InvalidClaim);
    }
    if groups.len() != children.len() {
        return Err(Error::InvalidProofShape);
    }
    G::decode(&root.commitment)?;
    let mut top = Statement {
        commitment: root.commitment,
        points: Vec::new(),
    };
    let mut rows = Vec::new();
    for ((&row, positions), &commitment) in groups.iter().zip(children) {
        G::decode(&commitment)?;
        top.points.push((point(row), child_hash(&commitment)));
        rows.push(Statement {
            commitment,
            points: positions
                .iter()
                .map(|&p| (point(indices[p] % WIDTH), leaf(&values[p])))
                .collect(),
        });
    }
    Ok(core::iter::once(top).chain(rows).collect())
}
/// Length-prefixed, architecture-independent transcript; fixed Ethereum SRS and v1 domain.
fn transcript(
    root: &Root,
    indices: &[usize],
    values: &[Vec<u8>],
    children: &[Commitment],
) -> Vec<u8> {
    let mut t = b"blob-tree/v1/ethereum-4844/shplonk".to_vec();
    t.extend_from_slice(&(root.len as u64).to_be_bytes());
    t.extend_from_slice(&root.commitment);
    t.extend_from_slice(&(indices.len() as u64).to_be_bytes());
    for (&index, value) in indices.iter().zip(values) {
        t.extend_from_slice(&(index as u64).to_be_bytes());
        t.extend_from_slice(&(value.len() as u64).to_be_bytes());
        t.extend_from_slice(value);
    }
    t.extend_from_slice(&(children.len() as u64).to_be_bytes());
    for c in children {
        t.extend_from_slice(c);
    }
    t
}
// Preserve the original hash on the first attempt; append a nonce only on retries.
fn retry_challenge(
    domain: &[u8],
    parts: &[&[u8]],
    valid: impl Fn(F) -> bool,
    mut hash: impl FnMut(&[u8], &[&[u8]]) -> F,
) -> Result<F> {
    let candidate = hash(domain, parts);
    if valid(candidate) {
        return Ok(candidate);
    }
    for nonce in 1..=u64::MAX {
        let nonce_bytes = nonce.to_be_bytes();
        let mut retry_parts = parts.to_vec();
        retry_parts.push(&nonce_bytes);
        let candidate = hash(domain, &retry_parts);
        if valid(candidate) {
            return Ok(candidate);
        }
    }
    Err(Error::DegenerateChallenge)
}
fn rho(t: &[u8]) -> Result<F> {
    retry_challenge(b"rho", &[t], |r| r != F::zero(), hash_field)
}
fn challenge(t: &[u8], quotient: &Commitment, statements: &[Statement]) -> Result<F> {
    retry_challenge(
        b"z",
        &[t, quotient],
        |z| {
            statements
                .iter()
                .all(|s| s.points.iter().all(|(x, _)| *x != z))
        },
        hash_field,
    )
}
fn weights(statements: &[Statement], rho: F, z: F) -> Result<(Vec<F>, F)> {
    let mut weights = Vec::new();
    let mut y = F::zero();
    let mut power = F::one();
    for statement in statements {
        let (interpolation, vanishing) = interpolate(&statement.points)?;
        // Challenge derivation excludes queried points; keep inversion checked as well.
        let weight = power * eval(&vanishing, z).inv()?;
        y = y + weight * eval(&interpolation, z);
        weights.push(weight);
        power = power * rho;
    }
    Ok((weights, y))
}
/// Sort values in ascending big-endian byte order, then pair them with `proof.indices`.
/// Counts must match; duplicate values are rejected and the caller's slice is not modified.
/// Applications requiring specific positions must also check `proof.indices`.
/// Malformed input returns Err; a well-formed but false proof normally returns Ok(false).
/// Accept only Ok(true). Exactly one c-kzg single-opening verification is performed.
pub fn verify(root: &Root, values: &[Vec<u8>], proof: &Proof) -> Result<bool> {
    if values.len() != proof.indices.len() {
        return Err(Error::InvalidClaim);
    }
    let mut values = values.to_vec();
    values.sort_unstable();
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(Error::DuplicateValue);
    }
    let statements = statements(root, &proof.indices, &values, &proof.children)?;
    let q = G::decode(&proof.quotient)?;
    G::decode(&proof.opening)?;
    let t = transcript(root, &proof.indices, &values, &proof.children);
    let rho = rho(&t)?;
    let z = challenge(&t, &proof.quotient, &statements)?;
    let (weights, y) = weights(&statements, rho, z)?;
    let mut combined = q.scale(F::zero() - F::one());
    for (statement, weight) in statements.iter().zip(weights) {
        combined = combined.add(G::decode(&statement.commitment)?.scale(weight));
    }
    #[cfg(feature = "prover")]
    return Ok(ethereum_kzg_settings(0).verify_kzg_proof(
        &Bytes48::from(combined.bytes()),
        &Bytes32::from(z.bytes()),
        &Bytes32::from(y.bytes()),
        &Bytes48::from(proof.opening),
    )?);
    #[cfg(feature = "verification-only")]
    {
        // [s]G2 from the same fixed Ethereum setup used by the native prover.
        // Only these 96 bytes are needed; no setup loader or G1 tables are linked.
        let verifier = c_kzg::KzgVerifier::new(include_bytes!("ethereum_g2_x.bin"))?;
        Ok(verifier.verify_kzg_proof(&combined.bytes(), &z.bytes(), &y.bytes(), &proof.opening)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn nondegenerate_challenges_preserve_original_hashes() {
        let t = b"test transcript";
        let d = G::default().bytes();
        assert_eq!(rho(t).unwrap(), hash_field(b"rho", &[t]));
        let statements = [Statement {
            commitment: d,
            points: vec![(point(0), F::one())],
        }];
        assert_eq!(
            challenge(t, &d, &statements).unwrap(),
            hash_field(b"z", &[t, &d])
        );
    }

    #[test]
    fn zero_challenge_retries_with_big_endian_nonces() {
        // Force the negligible zero case and more than one retry.
        let mut calls = 0u64;
        let result = retry_challenge(
            b"rho",
            &[b"transcript"],
            |r| r != F::zero(),
            |domain, parts| {
                assert_eq!(domain, b"rho");
                assert_eq!(parts[0], b"transcript");
                if calls == 0 {
                    assert_eq!(parts.len(), 1);
                } else {
                    assert_eq!(parts.len(), 2);
                    assert_eq!(parts[1], calls.to_be_bytes());
                }
                calls += 1;
                if calls <= 2 {
                    F::zero()
                } else {
                    F::one()
                }
            },
        )
        .unwrap();
        assert_eq!(result, F::one());
        assert_eq!(calls, 3);
    }

    #[test]
    fn checking_point_retries_across_both_levels() {
        let t = b"test transcript";
        let d = G::default().bytes();
        // Put the first two real hash candidates in different statements.
        let first = hash_field(b"z", &[t, &d]);
        let second = hash_field(b"z", &[t, &d, &1u64.to_be_bytes()]);
        let expected = hash_field(b"z", &[t, &d, &2u64.to_be_bytes()]);
        assert_ne!(expected, first);
        assert_ne!(expected, second);
        let statements = [first, second].map(|x| Statement {
            commitment: d,
            points: vec![(x, F::one())],
        });
        let z = challenge(t, &d, &statements).unwrap();
        assert_eq!(z, expected);
        assert_eq!(challenge(t, &d, &statements).unwrap(), z);
        assert!(weights(&statements, rho(t).unwrap(), z).is_ok());
    }

    #[test]
    fn weights_still_reject_division_by_zero() {
        let statement = Statement {
            commitment: G::default().bytes(),
            points: vec![(point(0), F::one())],
        };
        assert!(matches!(
            weights(&[statement], F::one(), point(0)),
            Err(Error::DegenerateChallenge)
        ));
    }
}
