# Blob Tree: design and security boundaries

This Rust proof of concept commits to a vector of byte strings sorted in ascending big-endian byte order using two levels of EIP-4844 blobs. It proves the values at one or more specified indices and supports vectors of length $1 \le n \le 4096^2$.

## 1. Terminology, definitions, and conventions

### Data and tree structure

| Term or symbol | Meaning |
|---|---|
| Element / `value` | An arbitrary byte string, including the empty byte string. Values must be unique; duplicate byte strings are rejected. |
| $B$ | The width of each blob: 4096 field elements. The tree capacity is $B^2 = 16{,}777{,}216$ elements. |
| $n$ | The actual vector length, with $1 \le n \le B^2$. |
| Global index $i$ | A zero-based position in the vector. Its child index is $\mathrm{row} = \lfloor i/B \rfloor$ and its position within that child is $\mathrm{column} = i \bmod B$. |
| Blob / slot | A blob holds 4096 polynomial evaluations; a slot is one evaluation position. A blob is not an array of polynomial coefficients. |
| Child / subtree | A bottom-level blob containing hashes of up to $B$ consecutive elements. There are $\lceil n/B \rceil$ used children. |
| Top-level blob | A blob containing hashes of child commitments, with unused slots set to zero. |
| $H_{\mathrm{leaf}}$, $H_{\mathrm{child}}$ | Domain-separated hashes of element bytes and canonical child-commitment bytes, respectively, reduced to the scalar field. |
| $R$, `Root` | $R$ is the top-level commitment. The complete public root is `Root { commitment: R, len: n }`; both fields must be trusted. |
| Claim / public statement | A claim pairs an index in `Proof.indices` with the corresponding separately supplied value after sorting values. The API is `verify(root, values, proof)`; both lists must have equal lengths. |
| $k$, $m$ | The number of queried elements and the number of distinct children containing them, respectively. |

### Algebra and proof objects

| Term or symbol | Meaning |
|---|---|
| $\mathbb{F}_r$, $r$ | The BLS12-381 scalar field and its prime modulus. Polynomial coefficients, evaluations, and challenges belong to this field. |
| $X$, $x$, $z$ | $X$ is a formal polynomial variable; $x$ is an evaluation point; $z$ is the transcript-derived checking point. |
| $\omega$, $x_s$ | $\omega = 7^{(r-1)/4096}$ is the domain root of unity; blob slot $s$ corresponds to $x_s = \omega^{\mathrm{bitreverse}_{12}(s)}$. |
| $\mathrm{bitreverse}_{12}(s)$ | Reversal of the 12 bits of the zero-based slot index $s$. |
| FFT | Fast Fourier transform, used here to convert between polynomial coefficients and evaluations on the blob domain. |
| KZG / $\mathrm{Commit}(f)$ | A polynomial commitment scheme; $\mathrm{Commit}(f)$ is the G1 commitment to polynomial $f$ under the fixed Ethereum setup. |
| Opening / witness | A proof that a committed polynomial has a claimed value at a specified point. A multipoint opening asserts several evaluations of one polynomial. |
| $f_i$, $C_i$, $S_i$ | In the reduction, opening statement $i$ has polynomial $f_i$, commitment $C_i$, and a set $S_i$ of distinct evaluation points with associated claimed values. Statement 0 is the top level; statements 1 through $m$ are the queried children in ascending child-index order. |
| $I_i$, $Z_i$ | The interpolation polynomial through statement $i$'s claimed point-value pairs and its vanishing polynomial $Z_i(X) = \prod_{x \in S_i}(X-x)$. |
| $q_i$, $Q$, $D$ | An individual quotient polynomial, the weighted aggregate of those quotients, and the commitment $D = \mathrm{Commit}(Q)$. |
| $T$, $\rho$ | The encoded transcript and its nonzero aggregation challenge. A transcript is the ordered byte sequence binding the public data used to derive challenges. |
| $a_i$, $g$, $y$, $C_g$ | Reduction weights, the combined polynomial, its claimed value at $z$, and its commitment. |
| $W$ | The single-point KZG opening witness for $g(z) = y$. |
| $G_1$, $G_2$, $e$ | The two source groups and the bilinear pairing on BLS12-381. In pairing equations, $G_1$ and $G_2$ denote their generators. |
| SRS, $\tau$ | The structured reference string (trusted setup) and its secret setup scalar. $[\tau]G_2$ is public; the secret scalar must remain unknown. |

### Notation and encoding conventions

All vector, child, and slot indices are zero-based. Queries are nonempty, strictly increasing, unique, and less than $n$. Child commitments follow ascending child-index order.

The index $i$ is local to its context: in the tree layout it denotes an element index; in the reduction it denotes an opening statement numbered $0, \ldots, m$. A layout commitment $C_j$ refers to child $j$, whereas reduction commitment $C_0$ is $R$. Sums over opening statements include the top level and all $m$ queried children.

Arithmetic on field elements is modulo $r$; division means multiplication by a nonzero field inverse. `||` denotes byte concatenation. In the proof wire format, query counts and absolute indices are canonical unsigned base-128 varints (least-significant seven-bit group first, no protobuf field tags); transcript integers and hash input lengths remain unsigned 64-bit big-endian values. G1 points use canonical 48-byte compressed encodings; sizes marked `B` in size tables mean bytes, whereas the standalone symbol $B$ in formulas denotes the blob width. Proof sizes include query indices and their count, and exclude the public root and original values.

## 2. Data structure and semantics

Each element is an arbitrary byte string; the empty byte string is valid. `Tree::new(values: &[Vec<u8>])` sorts a copy lexicographically by unsigned bytes before building the tree. Bytes are compared from first (most significant) to last; a shorter prefix sorts first. This agrees with unsigned big-endian integer order for equal-width values. Variable-length strings are compared as bytes without numeric normalization or removing leading zeros. Empty strings sort first. Construction and verification reject duplicate byte strings with `Error::DuplicateValue`. Indices refer to this sorted vector, not input positions. Input permutations therefore produce the same root. Global index $i$ maps to $\mathrm{row} = \lfloor i/B \rfloor$ and $\mathrm{column} = i \bmod B$.

- Each bottom-level blob stores $H_{\mathrm{leaf}}(\mathrm{value})$ for its elements, padding any remaining slots with scalar zero.
- `blob_to_kzg_commitment` produces a commitment $C_j$ for each bottom-level blob.
- Slot $j$ of the top-level blob stores $H_{\mathrm{child}}(C_j)$. Unused child slots are zero. Committing to this blob produces $R$.
- The public root is the complete **`Root { commitment: R, len: n }`**. Applications must obtain both fields from a trusted source. Authenticating only $R$ while allowing the prover to choose $n$ does not satisfy this protocol.

$H_{\mathrm{leaf}}$ and $H_{\mathrm{child}}$ use SHA-256 with different domains, then reduce the full 256-bit big-endian digest modulo $r$. Reducing raw element bytes directly would allow constructible collisions between integers $x$ and $x+r$. Child commitments are hashed in their canonical 48-byte compressed representation. Zero padding does not confer membership: verification first requires `index < n`.

Blob slots use c-kzg's ordering: $x_s = \omega^{\mathrm{bitreverse}_{12}(s)}$, where $\omega = 7^{(r-1)/4096}$. Each blob uniquely determines a polynomial of degree at most 4095. FFT conversion accounts for the difference between natural order and the blob's bit-reversed order. Tests compare evaluations at these points directly with c-kzg.

## 3. Reducing both levels to two proof points

Sort the queried children by index and build these multipoint opening statements:

1. For the top-level polynomial $f_0$, open $H_{\mathrm{child}}(C_j)$ at each queried child's position.
2. For each queried child's polynomial $f_i$, open $H_{\mathrm{leaf}}(\mathrm{value})$ at the queried column positions.

Each statement has a commitment $C_i$ and a set of distinct points $S_i$ with claimed values. Define the interpolation and vanishing polynomials:

$$
\begin{aligned}
I_i(x) &= \text{the claimed value at } x, \qquad x \in S_i, \\
Z_i(X) &= \prod_{x \in S_i} (X - x).
\end{aligned}
$$

This is a Shplonk-style reduction: aggregate quotient polynomials, check their relation at a transcript-derived point, and finish with a single-point KZG opening. For a related implementation, see [gnark-crypto's Shplonk code](https://github.com/Consensys/gnark-crypto/blob/master/ecc/bls12-381/shplonk/shplonk.go). The transcript and wire format are defined specifically for this implementation; no interoperability with other Shplonk implementations is claimed.

### Proving

Encode the complete root, queries, values, and child commitments into transcript $T$. Derive the nonzero challenge $\rho$, then compute:

$$
\begin{aligned}
q_i(X) &= \frac{f_i(X) - I_i(X)}{Z_i(X)}, \\
Q(X) &= \sum_{i=0}^{m} \rho^i q_i(X), \\
D &= \mathrm{Commit}(Q), \\
z &= H_z(T, D), \\
a_i &= \frac{\rho^i}{Z_i(z)}, \\
g(X) &= \sum_{i=0}^{m} a_i f_i(X) - Q(X), \\
y &= \sum_{i=0}^{m} a_i I_i(z).
\end{aligned}
$$

For honest claims, $g(z) = y$. Open $g$ at $z$ with `compute_kzg_proof` to obtain $W$. The proof is:

```text
[k, index_0, ..., index_(k-1), C_child_0, ..., C_child_(m-1), D, W]
```

Here the child list contains only queried children, in ascending child-index order. The prover does not send $I_i$, $Z_i$, $a_i$, $y$, or intermediate evaluations: the verifier can reconstruct them. Quotients have degree at most 4094, and $g$ has degree at most 4095, so both fit standard EIP-4844 blobs. If a statement covers all 4096 domain points, a correct claim has $f_i = I_i$ and a zero quotient. The implementation handles this boundary explicitly.

### Verification

`Tree::prove(indices)` returns `(values, proof)` with the selected values in index order. Verification sorts a copy of the supplied values using the same byte ordering, then pairs `values[p]` with `proof.indices[p]` and rejects mismatched counts. Callers may supply values in any order; their input is not modified. Applications requiring specific positions must check `proof.indices` against those positions.

Reconstruct $T$, $\rho$, $z$, $I_i$, $Z_i$, $a_i$, and $y$. Use G1 addition and scalar multiplication to compute:

$$
C_g = \sum_{i=0}^{m} a_i C_i - D.
$$

Make one call to `verify_kzg_proof(C_g,z,y,W)`, which checks:

$$
e\!\left(C_g - y \cdot G_1,\, G_2\right) = e\!\left(W,\, [\tau]G_2 - z \cdot G_2\right).
$$

This is **one equation with two pairing terms**, not a single pairing. The repository's `pairings_verify` in `src/common/utils.c` uses a combined Miller loop and one final exponentiation. Prover-side commitments and openings require no pairings. Verification performs this one check regardless of the number of queried elements or children.

## 4. Proof size and computational cost

A proof for $k$ queries at indices $i_j$ spanning $m$ children is $v(k) + \sum_{j=0}^{k-1} v(i_j) + 48m + 96$ **bytes** in canonical compressed form, where $v(x)$ is the unsigned base-128 varint length, with $v(0)=1$. Write $S=\sum_j v(i_j)$ below. Queries within one child share its commitment; each element adds a one- to four-byte index but no individual opening witness. Both the maximum count $4096^2$ and the maximum index $4096^2-1$ need at most four bytes, so $4 + 4k + 48m + 96$ is an upper bound.

| Query distribution | $m$ | Proof size | Verification pairing terms |
|---|---:|---:|---:|
| One element at index 7 | 1 | 146 B | 2 |
| All 4096 elements of child 0 | 1 | 8210 B | 2 |
| $k$ queries spanning two children | 2 | $192 + v(k) + S$ B | 2 |
| $k$ queries spanning all 4096 children | 4096 | $196704 + v(k) + S$ B | 2 |

These sizes include the index count and indices, and exclude the public root and original elements. The original elements are part of the statement being verified; they are not implicitly hidden or transmitted for free. This construction makes no claim of globally optimal proof size. It favors a readable two-point reduction that avoids individual opening witnesses. Child commitments still grow with $m$ because the top-level hashes require their bytes.

Proving is intentionally unoptimized, using naive Newton interpolation and polynomial long division. If statement $i$ contains $k_i$ points, interpolation costs $O\!\left(\sum_{i=0}^{m} k_i^2\right)$, long division costs $O\!\left(B \sum_{i=0}^{m} k_i\right)$, and FFTs cost $O(mB \log B)$. Verification also uses $O\!\left(\sum_{i=0}^{m} k_i^2\right)$ interpolation and $O(m)$ G1 operations, with a constant number of pairing terms. The structure supports full capacity, but memory and runtime have not been benchmarked at 16,777,216 elements. `Tree` retains the original values and polynomial coefficients for every used blob, with substantial memory overhead appropriate to a proof of concept.

## 5. Transcript and encoding

The protocol fixes the Ethereum trusted setup and uses the protocol domain `blob-tree/shplonk`. Callers cannot substitute another SRS.

$T$ concatenates the following fields in order. All integers are **u64 big-endian**:

```text
protocol domain bytes
n || R[48]
k || (index || value_length || value_bytes) for each query
m || C_j[48] for each queried child in ascending child-index order
```

The exact preimage of `hash_field(domain, parts)` is the fixed prefix `blob-tree/v1/`, followed by the domain's length and bytes, then each part's length and bytes. Every length is u64 big-endian. The hash is SHA-256, reduced modulo $r$. Leaf and child hashing use the domains `leaf` and `child`. Initial challenge candidates are `ρ=hash_field("rho",[T])` and `z=hash_field("z",[T,D])`. Since $\rho$ is a deterministic function of $T$, the input for $z$ also implicitly binds $\rho$.

If $\rho = 0$, retry `hash_field("rho", [T, nonce_be])`. If $z$ equals any queried evaluation point at either tree level, retry `hash_field("z", [T, D, nonce_be])`, since $Z_i(z)$ must be nonzero. Each challenge has its own nonce starting at 1 and increasing until the first valid candidate. `nonce_be` is u64 big-endian, included as an additional length-prefixed hash part. Keep $T$, $\rho$, and $D$ fixed while retrying $z$.

Prover and verifier run the same retries; no nonce is sent and proof sizes stay unchanged. The initial attempt keeps the original hash encoding. Counter exhaustion returns `DegenerateChallenge` without wrapping. SHA-256 reduction is biased: before rejection, each field element has probability less than $3/2^{256}$. Security analysis must account for the accepted challenge sets.

Queries must be nonempty, strictly increasing, unique, and in range. The child list is derived from the indices in the proof, and its commitment count must match exactly. The wire format is `k || index_0 || ... || index_(k-1) || C_child_0 || ... || C_child_(m-1) || D || W`, using minimal unsigned base-128 varints for counts and indices and 48-byte compressed points. Its length is $v(k) + \sum_j v(i_j) + 48m + 96$; trailing bytes are rejected. Varints longer than four bytes, redundant high zero groups, and truncated encodings are rejected. The decoder bounds the query count by the trusted root length and the available input before allocation, then validates decoded indices against the root. The transcript encoding above remains u64 big-endian. This replaces the previous Rust u64 proof wire format; existing serialized proofs must be regenerated or decoded with the old implementation and re-encoded. All input G1 points undergo decompression, subgroup checks, and re-encoding checks before arithmetic. The point at infinity is valid because zero polynomials can legitimately produce zero commitments and witnesses.

## 6. Why aggregation prevents simple forgeries

Suppose at least one opening claim is false. Deriving $\rho$ after fixing the statement makes deliberate cancellation between polynomial errors difficult. Fixing $D$ before deriving $z$ prevents the prover from choosing $Q$ after seeing the checking point. Multiplying the relation

$$
Q(z) = \sum_{i=0}^{m} \rho^i \frac{f_i(z) - I_i(z)}{Z_i(z)}
$$

by the product of all $Z_i(z)$ turns it into a polynomial identity check. Except when the aggregation coefficients cancel the errors, a false relation can hold only at finitely many checking points. Otherwise, acceptance requires breaking KZG opening binding. A formal analysis needs assumptions such as the random-oracle and algebraic-group models; this intuition is not an unconditional security proof.

The implementation also avoids an invalid shortcut: sending only $\sum_i \rho^i W_i$ and $\sum_i \rho^i z_i W_i$ and applying one pairing check. Without proving the relation between those two points, the second can be forged. This construction instead fixes a quotient commitment $D$ and produces an actual KZG opening of a single polynomial derived from $D$ and the statement.

## 7. Security assumptions, boundaries, and validation

Security depends on a trusted public root, secrecy of the Ethereum SRS's toxic waste, computational binding of KZG, and the collision-resistance and random-oracle assumptions for SHA-256. The construction is neither a hiding commitment nor a zero-knowledge proof. It has not undergone an independent cryptographic audit and makes no guarantee of being free of vulnerabilities. Its safeguards are a standard reduction structure, explicit assumptions, strict input validation, and negative tests; testing does not replace an audit.

Unsafe blst calls are confined to the private algebra module. Inputs and outputs are initialized, fixed-size objects, with no manual ownership transfer or external raw pointers. Public callers cannot construct internal field elements. Malformed structure or points return `Err`; well-encoded but incorrect proofs generally return `Ok(false)`. Applications must accept only `Ok(true)`. Applications must also limit resource consumption for large public statements according to their deployment needs; this proof of concept is not designed as a network service.

Tests cover canonical value ordering, permutation-invariant roots and verification, duplicate value rejection, same-child and cross-child openings, all 4096 slots, empty byte strings, tampered values/indices/roots/lengths/children/both proof points, duplicate and out-of-range queries, malformed and non-subgroup points, exact wire encoding, field reduction, nonce retries for zero and queried-point challenges, and FFT consistency with c-kzg. They neither exhaust all possible inputs nor constitute a security proof.
