//! Philox-4×32-10, a counter-based pseudo-random function (manual §21.2, ADR
//! 0003; Salmon, Moraes, Dror & Shaw, "Parallel Random Numbers: As Easy as
//! 1, 2, 3", SC'11).
//!
//! A counter-based generator is a pure function `f(counter, key) -> block`. It
//! has no mutable internal state to serialise, so it satisfies §21.2 properties
//! 1–4 (order invariance, random access, common random numbers, stream
//! isolation) by construction rather than by discipline.

/// Multiply-high / multiply-low: the 64-bit product of two `u32`s, split.
#[inline]
fn mulhilo(a: u32, b: u32) -> (u32, u32) {
    let p = u64::from(a) * u64::from(b);
    ((p >> 32) as u32, (p & 0xffff_ffff) as u32)
}

// The published Philox-4×32 constants. Not tunables — changing any of these
// makes this "not Philox" and would need an ADR and a golden-trace update.
const M0: u32 = 0xD251_1F53;
const M1: u32 = 0xCD9E_8D57;
const W0: u32 = 0x9E37_79B9; // golden ratio
const W1: u32 = 0xBB67_AE85; // sqrt(3) - 1
const ROUNDS: usize = 10;

#[inline]
fn round(ctr: [u32; 4], key: [u32; 2]) -> [u32; 4] {
    let (hi0, lo0) = mulhilo(M0, ctr[0]);
    let (hi1, lo1) = mulhilo(M1, ctr[2]);
    [hi1 ^ ctr[1] ^ key[0], lo1, hi0 ^ ctr[3] ^ key[1], lo0]
}

#[inline]
fn bumpkey(key: [u32; 2]) -> [u32; 2] {
    [key[0].wrapping_add(W0), key[1].wrapping_add(W1)]
}

/// One 128-bit output block for `(counter, key)`. Matches the Random123
/// reference: round 1 uses the key as given, then the key is bumped before each
/// subsequent round, for 10 rounds total.
#[must_use]
pub fn philox4x32_10(counter: [u32; 4], key: [u32; 2]) -> [u32; 4] {
    let mut c = counter;
    let mut k = key;
    for r in 0..ROUNDS {
        if r > 0 {
            k = bumpkey(k);
        }
        c = round(c, k);
    }
    c
}

#[cfg(test)]
mod tests {
    use super::philox4x32_10;

    // Known-answer tests from the Random123 reference distribution
    // (kat_vectors). If these fail, the generator is not Philox-4×32-10.
    #[test]
    fn kat_zero() {
        assert_eq!(
            philox4x32_10([0, 0, 0, 0], [0, 0]),
            [0x6627_e8d5, 0xe169_c58d, 0xbc57_ac4c, 0x9b00_dbd8]
        );
    }

    #[test]
    fn kat_all_ones() {
        assert_eq!(
            philox4x32_10(
                [0xffff_ffff, 0xffff_ffff, 0xffff_ffff, 0xffff_ffff],
                [0xffff_ffff, 0xffff_ffff]
            ),
            [0x408f_276d, 0x41c8_3b0e, 0xa20b_c7c6, 0x6d54_51fd]
        );
    }

    #[test]
    fn kat_pi_digits() {
        assert_eq!(
            philox4x32_10(
                [0x243f_6a88, 0x85a3_08d3, 0x1319_8a2e, 0x0370_7344],
                [0xa409_3822, 0x299f_31d0]
            ),
            [0xd16c_fe09, 0x94fd_cceb, 0x5001_e420, 0x2412_6ea1]
        );
    }
}
