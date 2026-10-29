# Verification optimization TODO

Baseline profiled on 2026-10-28 using `ckb-debugger` and flamegraphs.
Item 1 was implemented on 2026-10-29; the other priorities retain baseline costs.

## Benchmark and profiling

```sh
make -C ckb-tests bench-blob-tree
```

The large fixture reproduced **1,000 values, 25 children: 334.36 M cycles**.
The symbolized release profile measured **334.33 M verification cycles**
(less than 0.01% difference). Fixture decoding is outside the verification timer;
the original full run consumed 403.75 M cycles.

Profiling used a separate release build with debug symbols. Because
ckb-debugger's full mode dropped command-line arguments, a temporary ELF entry
shim supplied the same benchmark inputs without changing verifier machine code.
Assembly symbols were also tagged as functions in the temporary ELF so the
flamegraph could attribute their costs.

Local profiling artifacts (temporary files, not committed):

- Flamegraph: `/private/tmp/blob-tree-profile/verify.svg`
- Symbolized folded stacks: `/private/tmp/blob-tree-profile/verify.folded`
- Cycle breakdown: `/private/tmp/blob-tree-profile/summary.txt`

## Optimization priorities

1. **Done: optimize Fr multiplication and Fp squaring for RV64.** Generic
   `mul_mont_n` consumes **180.71 M cycles (54.1%)** across verification.
   The current assembly covers 384-bit multiplication, while 256-bit Fr
   multiplication and 384-bit squaring previously used generic C. Added
   register-based 256-bit Montgomery multiplication and a 384-bit square with
   21 wide products instead of 36, followed by Montgomery reduction.
   Fr squaring also uses the new multiplication routine.

   The new assembly preserves adjacent `mulhu`/`mul`, `add`/`sltu`/`add`, and
   `sub`/`sltu` sequences for CKB-VM fusion. Compression is disabled around the
   carry additions because `c.add` can swap operands and prevent ADD3 matching.
   The VM v2 decoder confirmed WIDE_MULU, ADD3A/ADD3C, and SBBS matches.

   | Fixture | Before | After | Reduction |
   | --- | ---: | ---: | ---: |
   | 1 value, 1 child | 71.61 M | 61.13 M | 14.6% |
   | 6 values, 2 children | 77.93 M | 65.51 M | 15.9% |
   | 1,000 values, 25 children | 334.36 M | 228.70 M | 31.6% |

   Measured with `make -C ckb-tests bench-blob-tree`, Clang 19 and
   `ckb-debugger` 1.1.1 (VM v2). The symbolized after-profile measured
   228.67 M verification cycles; Fr multiplication used 38.50 M and Fp
   squaring 33.91 M. Temporary artifacts are in
   `/private/tmp/blob-tree-profile-after/{verify.svg,verify.folded,summary.txt}`.
   `make ckb-test` includes independent bit-serial arithmetic comparisons,
   carry-heavy and random inputs, and in-place output checks. These also
   passed on VM v1. All three host Cargo test suites passed.

2. **Evaluate interpolation directly at the challenge.** `weights()` costs
   **101.94 M cycles (30.5%)**, including **80.69 M** in Fr multiplication.
   Verification constructs complete interpolation and vanishing polynomials
   only to evaluate them at one point. Use barycentric evaluation and direct
   vanishing products; batch the required inversions. Existing Newton
   denominator inversions are already batched.

3. **Replace 26 separate G1 scalar multiplications with one MSM.** `G::scale`
   costs **90.32 M cycles (27.0%)**. Compute the weighted root/child commitment
   sum using a multi-scalar multiplication algorithm. Benchmark Pippenger
   against a joint-window/Straus implementation at this relatively small batch
   size.

4. **Reuse validated points across verification boundaries.** `G::decode`
   costs **68.59 M cycles (20.5%)**, including **55.01 M** in subgroup checks.
   Introduce an internal validated-point path so the opening and computed
   aggregate need not be serialized, decompressed, and subgroup-checked again
   by c-kzg. Preserve validation for untrusted inputs. Parser/verifier reuse can
   also reduce end-to-end costs, though parsing is outside this benchmark's
   timer.

5. **Provide a lightweight single-opening verifier context.**
   `KzgVerifier::new` costs **6.92 M cycles (2.1%)**. It constructs 4,096 roots
   of unity although single-opening verification does not use them. Initialize
   that table lazily, and reuse or precompute the authenticated fixed G2 key
   representation.

6. **Combine the two Miller loops.** Pairing verification costs **33.09 M
   cycles (9.9%)**: **19.31 M** in Miller loops and **13.51 M** in final
   exponentiation. Evaluate `blst_miller_loop_n` for the two pairs; the
   implementation already performs only one final exponentiation.

These are measured optimization targets, not predicted savings. Their costs
overlap, especially with item 1, and must not be added together.
