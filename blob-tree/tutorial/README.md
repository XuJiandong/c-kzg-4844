# Blob Tree: Two levels of blobs, one membership proof

At its core, it is a two-level path tree built on [c-kzg-4844](https://github.com/ethereum/c-kzg-4844), with [Shplonk](https://eprint.iacr.org/2020/081.pdf). Commit to as many as 16,777,216（4096*4096) elements in a tree with just two levels. Whether you query one element or many, the proof reduces to one KZG verification call. 

## 1. Tree structure

Every position has a two-level path.

Split an element's zero-based index $i$ into $\mathrm{row} = \lfloor \frac{i}{4096} \rfloor$ and $\mathrm{column} = i \bmod 4096$. Each bottom-level blob stores element hashes. The top-level blob stores hashes of child commitments. Both hashes are reduced modulo the BLS12-381 scalar field modulus $r$.

**Worked example:** index $4103 = 1 \times 4096 + 7$ belongs to child 1, slot 7. Its element hash is committed by $C_{1}$, and the hash of $C_{1}$ occupies top-level slot 1.

![The path from element 4103 through child slot 7 and top-level slot 1 to the public root (R, n). Both levels have 4096 slots.](assets/tree-structure.svg)

The root is constructed from the bottom up. Lines show dependencies; the diagram does not draw all 4096 slots to scale.

## 2. Proof reduction

Share child commitments, then combine the openings.

A proof includes an 8-byte query count and an 8-byte index per queried element, then one commitment for each queried child, followed by two points: the aggregate quotient commitment $D$ and the opening witness $W$. Each point occupies 48 bytes.

For queries `[0, 7, 4096, 4097]`, child 0 supplies columns 0 and 7, while child 1 supplies columns 0 and 1. The four queries share two child commitments.

| Queried elements $k$ | Queried children $m$ | Proof bytes | Pairing terms |
| ---: | ---: | ---: | ---: |
| 4 | 2 | 232 | 2 |

![The 232-byte proof: 40 bytes of count and indices, 96 bytes of child commitments, and 96 bytes for D and W.](assets/proof-layout.svg)

**Proof sizes for different query distributions**

| Queries | Children m | Proof bytes |
| --- | --- | --- |
| One element: [7] | 1 | 160 |
| Four elements in one child: [0, 7, 77, 4095] | 1 | 184 |
| Two children: [0, 7, 4096, 4097] | 2 | 232 |
| Four children: [0, 4096, 8192, 16777215] | 4 | 328 |
| At least one element in every child | 4096 | 196712 + 8k |

$\text{Proof size} = 8 + 8 k + 48 m + 96$ bytes, including query indices and their count but excluding the public root and original elements. Each example assumes the vector is long enough to contain every queried index. Verification always uses two pairing terms. 

Note: the part `8k` can be smaller, see below.

### The complete reduction, step by step

The reduction first commits to an aggregate quotient polynomial and then checks its relation to the claimed openings at a transcript-derived point. All five steps are shown below. Here, statement 0 is the top level, and statements 1 through m are the queried children in ascending child-index order.

> **Where Shplonk is used:** steps 2–4 perform the Shplonk-style reduction: aggregate the quotient polynomials, commit to the aggregate before deriving the checking point, then reduce all claimed openings to the single claim `g(z) = y`. Step 1 binds the public statement to the transcript; step 5 completes the proof with a standard single-point KZG opening and verification. This implementation defines its own transcript and wire format; it does not claim interoperability with other Shplonk implementations.

#### Step 1: Bind everything the proof claims.

$$
\begin{aligned}
T &= \mathrm{Encode}\bigl(\text{protocol domain, root, queries}, \\
    &\quad \text{values, child commitments}\bigr)
\end{aligned}
$$

The root contains both the commitment and vector length. The top-level opening claims child-commitment hashes; each child opening claims element hashes. Queries and child commitments use their canonical order. `Encode` includes the counts and lengths specified in the design document.

#### Step 2: Combine the quotient polynomials.

$$
\begin{aligned}
\rho & = \mathrm{hash\_field} ( \text{"rho"} , [ T ] ) \\
q_{i} ( X ) & = \frac{f_{i} ( X ) - I_{i} ( X )}{Z_{i} ( X )} \\
Q ( X ) & = \sum_{i = 0}^{m} \rho^{i} q_{i} ( X )
\end{aligned}
$$

$I_{i}$ interpolates the claimed point-value pairs, and $Z_{i}$ vanishes at their evaluation points. The division has no remainder when the opening claims are correct. The nonzero coefficient $\rho$ is derived from the complete transcript.

#### Step 3: Fix D before deriving the checking point.

$$
\begin{aligned}
D & = \mathrm{Commit} ( Q ) \\
z & = \mathrm{hash\_field} ( \text{"z"} , [ T , D ] )
\end{aligned}
$$

$D$ is the first of the two additional proof points. Deriving $z$ from both $T$ and $D$ binds the checking point to the quotient commitment. Each `hash_field` call uses domain-separated, length-prefixed SHA-256 followed by reduction modulo $r$.

#### Step 4: Reconstruct the combined commitment.

$$
\begin{aligned}
a_{i} & = \frac{\rho^{i}}{Z_{i} ( z )} \\
g ( X ) & = \sum_{i = 0}^{m} a_{i} f_{i} ( X ) - Q ( X ) \\
C_{g} & = \sum_{i = 0}^{m} a_{i} C_{i} - D \\
y & = \sum_{i = 0}^{m} a_{i} I_{i} ( z )
\end{aligned}
$$

The remaining claim is $g ( z ) = y$. The verifier derives every weight, the commitment $C_{g}$, and $y$ from the public statement and $D$. No additional evaluations need to be transmitted.

#### Step 5: Check one equation with two pairing terms.

$$
\begin{aligned}
W & = \text{KZG opening witness for} \; g ( z ) = y \\
e ( C_{g} - y \cdot G_{1} , G_{2} ) & = e ( W , [ \tau ] G_{2} - z \cdot G_{2} )
\end{aligned}
$$

$W$ is the second additional 48-byte point. The verifier calls c-kzg's `verify_kzg_proof` once. The pairing equation uses two Miller loops and one final exponentiation. Beyond the count, indices, and child commitments, the proof always contains exactly 96 bytes.

The order is $T \to \rho \to D \to z \to W$. This static tutorial explains the structure and proof size.

## 3. Further proof size optimization

The integers in this tutorial can be encoded more compactly. Encoding them as protobuf [varint](https://protobuf.dev/programming-guides/encoding/) would make the proof substantially smaller. The proof sizes here are therefore upper bounds, and an actual implementation will be considerably smaller.

## 4. Support non-membership
We require all elements in the tree to be ordered and unique. In a sorted tree, prove that a value $e$ is absent by authenticating its two adjacent neighbors:

$$
e_{\mathrm{before}} < e < e_{\mathrm{after}},
\qquad i_{\mathrm{after}} = i_{\mathrm{before}} + 1.
$$

Verify both neighbors with one batched membership proof against the same trusted root $(R, n)$.

At the boundaries, authenticate only the first or last element:

- If $e < e_0$, prove membership of $e_0$ at index $0$.
- If $e_{n-1} < e$, prove membership of $e_{n-1}$ at index $n-1$, using the trusted length $n$.

## 5. Security boundaries

Connect each element all the way to the root.

### Verify both levels

The child opening binds the element to its child commitment. The top-level opening binds that commitment's hash to the root. Both are needed to establish membership in this tree.

### Preserve the transcript order

The challenge ρ binds the complete statement; z also binds D. Letting the prover choose D after seeing z would let it tailor a false relation to that single checking point.

### Hash before reducing

Directly reducing raw integers would identify x and x+r. Instead, SHA-256 hashes domain-separated, length-prefixed byte strings before the digest is reduced to a field element.

### Authenticate the length

The trusted root is the complete pair (R, n). Out-of-range and padding positions are not members. Queries must be strictly increasing, unique, and less than n.

### Degenerate challenges and special curve points

If ρ is zero or z equals any queried evaluation point at either tree level, retry its hash with a big-endian u64 nonce starting at 1. Prover and verifier select the first valid candidate independently, so no nonce is sent. 


## Continues

Continue with the [design and terminology](../DESIGN.md)
