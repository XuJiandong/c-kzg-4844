/* Minimal EIP-4844 verification context. No setup loader or prover is linked. */
#include "ckzg.c"

typedef struct {
    KZGSettings settings;
    g2_t g2[2];
    fr_t roots[FIELD_ELEMENTS_PER_BLOB];
} CKZGVerifier;

/* g2_x must be the authenticated, compressed [s]G2 from the trusted setup.
 * All other verification parameters are fixed by EIP-4844. */
C_KZG_RET ckzg_verifier_new(CKZGVerifier **out, const uint8_t g2_x[96]) {
    *out = NULL;
    blst_p2_affine affine;
    if (blst_p2_uncompress(&affine, g2_x) != BLST_SUCCESS ||
        !blst_p2_affine_in_g2(&affine) || blst_p2_affine_is_inf(&affine)) {
        return C_KZG_BADARGS;
    }
    CKZGVerifier *v = calloc(1, sizeof(*v));
    if (v == NULL) return C_KZG_MALLOC;
    v->g2[0] = *blst_p2_generator();
    blst_p2_from_affine(&v->g2[1], &affine);
    /* 7^((r-1)/4096), canonical little-endian limbs. */
    const uint64_t root_words[4] = {
        0xe206da11a5d36306ULL, 0x0ad1347b378fbf96ULL,
        0xfc3e8acfe0f8245fULL, 0x564c0a11a0f704f4ULL
    };
    fr_t root, power = FR_ONE;
    blst_fr_from_uint64(&root, root_words);
    for (size_t i = 0; i < FIELD_ELEMENTS_PER_BLOB; i++) {
        size_t reversed = 0;
        for (size_t j = 0; j < 12; j++) reversed = (reversed << 1) | ((i >> j) & 1);
        v->roots[reversed] = power;
        fr_mul(&power, &power, &root);
    }
    v->settings.brp_roots_of_unity = v->roots;
    v->settings.g2_values_monomial = v->g2;
    *out = v;
    return C_KZG_OK;
}

void ckzg_verifier_free(CKZGVerifier *v) { free(v); }
