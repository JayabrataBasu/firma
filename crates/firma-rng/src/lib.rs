//! `firma-rng` — counter-based RNG and hierarchical key derivation (manual
//! §21.2–21.3, ADR 0003).
//!
//! # The scheme
//!
//! ```text
//! key  = H(run_seed ‖ stream_id ‖ plugin_id ‖ phase_id ‖ tick ‖ agent_id ‖ purpose_tag)
//! draw = philox(key, counter)
//! ```
//!
//! The kernel builds an [`RngKey`] from the six fields it knows and passes it to
//! a rule. The rule calls [`open`] with a `purpose_tag` per independent draw
//! site; each `(RngKey, purpose_tag)` pair is a separate, isolated stream.
//!
//! # Byte encoding (open question OQ-1, `PROGRESS.md`)
//!
//! Manual §15.5 (Example E) shows the key input as a concatenation with
//! illustrative field widths and gives no reference draw value — only four
//! *properties* the fixture must satisfy. This implementation fixes a
//! **fixed-width big-endian** encoding that is injective in every field, which
//! is what properties 1–4 require:
//!
//! | field | bytes |
//! |---|---|
//! | `run_seed` | 8, big-endian |
//! | `stream_id` | 1 |
//! | `plugin_id` (`PluginId::numeric`) | 8, big-endian |
//! | `phase_id` (§10.1 number) | 1 |
//! | `tick` | 8, big-endian |
//! | `agent_present` | 1 (`0` = none, `1` = some) |
//! | `agent_id` | 8, big-endian (`0` when absent) |
//! | `purpose_tag` length | 8, big-endian |
//! | `purpose_tag` | UTF-8 bytes |
//!
//! `H` is SHA-256. The first 8 digest bytes seed the Philox key; the next 16
//! seed the initial counter.

#![forbid(unsafe_code)]

mod philox;

pub use philox::philox4x32_10;

use sha2::{Digest, Sha256};

use firma_core::{Phase, RngKey, StreamId};

/// The `purpose_tag` component of the RNG key (manual §21.2). Reusing one across
/// draw sites that should be independent silently correlates them — a review
/// checklist item (§21.2, ADR 0003).
pub type PurposeTag<'a> = &'a str;

/// A keyed, counter-based stream. Produced by [`open`]. Holds no entropy of its
/// own: every value is a pure function of `(key, counter)`, so two `KeyedRng`s
/// opened from equal inputs yield identical sequences forever (CRN, §21.2
/// property 3).
#[derive(Debug, Clone)]
pub struct KeyedRng {
    key: [u32; 2],
    counter: [u32; 4],
    buf: [u32; 4],
    /// Index into `buf`; `4` means "buffer spent, refill before next read".
    pos: usize,
}

impl KeyedRng {
    fn refill(&mut self) {
        self.buf = philox4x32_10(self.counter, self.key);
        self.pos = 0;
        // 128-bit little-endian increment of the counter for the next block.
        for word in &mut self.counter {
            let (v, carry) = word.overflowing_add(1);
            *word = v;
            if !carry {
                break;
            }
        }
    }

    /// The next 32-bit word.
    pub fn next_u32(&mut self) -> u32 {
        if self.pos >= 4 {
            self.refill();
        }
        let v = self.buf[self.pos];
        self.pos += 1;
        v
    }

    /// The next 64-bit word (two 32-bit draws, high word first).
    pub fn next_u64(&mut self) -> u64 {
        let hi = u64::from(self.next_u32());
        let lo = u64::from(self.next_u32());
        (hi << 32) | lo
    }

    /// A `f64` in `[0, 1)` with 53 bits of resolution. Non-conserved quantity —
    /// `f64` is fine here (§21.4).
    pub fn next_f64_unit(&mut self) -> f64 {
        // 53 high bits of a u64 divided by 2^53.
        let bits = self.next_u64() >> 11;
        (bits as f64) * (1.0 / 9_007_199_254_740_992.0)
    }

    /// A uniformly distributed integer in `0..n` (`n > 0`), by rejection so the
    /// result is unbiased and fully deterministic.
    ///
    /// # Panics
    /// If `n == 0`.
    pub fn next_below(&mut self, n: u64) -> u64 {
        assert!(n > 0, "next_below(0) is undefined");
        // Largest multiple of n that fits in u64; reject the ragged tail.
        let zone = u64::MAX - (u64::MAX % n);
        loop {
            let x = self.next_u64();
            if x < zone {
                return x % n;
            }
        }
    }

    /// A uniformly distributed integer in `lo..=hi` (inclusive).
    ///
    /// # Panics
    /// If `lo > hi`.
    pub fn uniform_inclusive(&mut self, lo: u64, hi: u64) -> u64 {
        assert!(lo <= hi, "uniform_inclusive: lo > hi");
        lo + self.next_below(hi - lo + 1)
    }

    /// A draw from `Normal(mean, std_dev)` by the Box–Muller transform
    /// (ADR 0034). Consumes **two** `next_f64_unit()` draws; the sine companion
    /// is discarded so the mapping stays a pure function of the two consumed
    /// uniforms (no hidden per-`KeyedRng` cache — a cache would make the
    /// sequence position-dependent and break CRN forks). `ln` and `cos` are
    /// transcendental: bit-identity across libm versions is a goal not a
    /// guarantee (§21.4), exactly as for the existing `y_O` / `y_R` floors.
    ///
    /// `std_dev` may be `0.0` (⇒ returns `mean` exactly); negative `std_dev`
    /// is treated as its absolute value.
    pub fn next_normal(&mut self, mean: f64, std_dev: f64) -> f64 {
        // u1 in (0, 1]: shift the [0, 1) draw off zero so ln is finite.
        let u1 = 1.0 - self.next_f64_unit();
        let u2 = self.next_f64_unit();
        let z = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
        mean + std_dev.abs() * z
    }
}

/// Derive the 32-byte key digest for a full key tuple.
fn digest(key: &RngKey, purpose: PurposeTag<'_>) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(key.run_seed.to_be_bytes());
    h.update([key.stream.as_byte()]);
    h.update(key.plugin_id.to_be_bytes());
    h.update([key.phase.as_byte()]);
    h.update(key.tick.to_be_bytes());
    match key.agent_id {
        Some(a) => {
            h.update([1u8]);
            h.update(a.to_be_bytes());
        }
        None => {
            h.update([0u8]);
            h.update(0u64.to_be_bytes());
        }
    }
    h.update((purpose.len() as u64).to_be_bytes());
    h.update(purpose.as_bytes());
    h.finalize().into()
}

#[inline]
fn be32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Open the stream for `key` but with `agent_id` overridden — the faithful way
/// for a rule that is invoked once per phase to obtain a per-agent stream with
/// the agent in its proper key slot (manual §21.2), rather than smuggling it
/// into `purpose`.
///
/// The kernel hands a rule an [`RngKey`] with `agent_id: None` (phase scope); a
/// rule iterating agents calls this with `Some(agent)`.
#[must_use]
pub fn open_for(key: &RngKey, agent_id: Option<u64>, purpose: PurposeTag<'_>) -> KeyedRng {
    let mut k = key.clone();
    k.agent_id = agent_id;
    open(&k, purpose)
}

/// Open the counter-based stream for `(key, purpose)`. Pure: same inputs ⇒ same
/// stream, in any run, in any order, in any fork (manual §21.2 properties 1–4).
#[must_use]
pub fn open(key: &RngKey, purpose: PurposeTag<'_>) -> KeyedRng {
    let d = digest(key, purpose);
    KeyedRng {
        key: [be32(&d[0..4]), be32(&d[4..8])],
        counter: [
            be32(&d[8..12]),
            be32(&d[12..16]),
            be32(&d[16..20]),
            be32(&d[20..24]),
        ],
        buf: [0; 4],
        pos: 4, // force a refill on first read
    }
}

/// Build an [`RngKey`] from its parts. A thin constructor kept here so callers
/// do not hand-assemble the struct and risk a field mix-up.
#[must_use]
pub fn key(
    run_seed: u64,
    stream: StreamId,
    plugin_id: u64,
    phase: Phase,
    tick: u64,
    agent_id: Option<u64>,
) -> RngKey {
    RngKey {
        run_seed,
        stream,
        plugin_id,
        phase,
        tick,
        agent_id,
    }
}

#[cfg(test)]
mod tests {
    use super::{key, open};
    use firma_core::{Phase, PluginId, StreamId};

    /// Manual §15.5 Example E — the fixture. The four required properties.
    fn example_e_key(agent: u64) -> firma_core::RngKey {
        key(
            0x5EED_0001,
            StreamId::Mechanism,
            PluginId::new("decision.satisficing").numeric(),
            Phase::Decide,
            147,
            Some(agent),
        )
    }

    #[test]
    fn property_1_same_tuple_same_draw() {
        let k = example_e_key(12);
        let mut a = open(&k, "shaping_lag");
        let mut b = open(&k, "shaping_lag");
        for _ in 0..64 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn property_2_changing_agent_changes_draw() {
        let mut a = open(&example_e_key(12), "shaping_lag");
        let mut b = open(&example_e_key(13), "shaping_lag");
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn property_3_fork_reusing_tuple_is_identical() {
        // A "fork" simply rebuilds the same key tuple later; CRN means the
        // stream is bit-identical.
        let mut original = open(&example_e_key(12), "shaping_lag");
        let pre: Vec<u64> = (0..8).map(|_| original.next_u64()).collect();
        let mut forked = open(&example_e_key(12), "shaping_lag");
        let post: Vec<u64> = (0..8).map(|_| forked.next_u64()).collect();
        assert_eq!(pre, post);
    }

    #[test]
    fn property_4_independent_streams_do_not_perturb_each_other() {
        // Stand-in for DT-6 at the RNG layer: interleaving draws from a second
        // stream (a different purpose_tag — as an extra rule would use) does not
        // change the first stream's sequence.
        let k = example_e_key(12);
        let solo: Vec<u64> = {
            let mut r = open(&k, "shaping_lag");
            (0..16).map(|_| r.next_u64()).collect()
        };
        let mut r = open(&k, "shaping_lag");
        let mut other = open(&k, "some_other_rule_purpose");
        let interleaved: Vec<u64> = (0..16)
            .map(|_| {
                let _ = other.next_u64();
                r.next_u64()
            })
            .collect();
        assert_eq!(solo, interleaved);
    }

    #[test]
    fn different_purpose_is_a_different_stream() {
        let k = example_e_key(12);
        let mut a = open(&k, "shaping_lag");
        let mut b = open(&k, "capability_lag");
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn f64_unit_in_range() {
        let mut r = open(&example_e_key(1), "x");
        for _ in 0..10_000 {
            let v = r.next_f64_unit();
            assert!((0.0..1.0).contains(&v));
        }
    }

    #[test]
    fn uniform_inclusive_covers_and_stays_in_bounds() {
        let mut r = open(&example_e_key(1), "roll");
        let mut seen = [false; 6];
        for _ in 0..10_000 {
            let v = r.uniform_inclusive(1, 6);
            assert!((1..=6).contains(&v));
            seen[(v - 1) as usize] = true;
        }
        assert!(seen.iter().all(|s| *s));
    }

    #[test]
    fn next_normal_is_deterministic_and_roughly_standard() {
        // CRN: same key ⇒ identical sequence (ADR 0034).
        let seq = |tag: &str| {
            let mut r = open(&example_e_key(1), tag);
            (0..5).map(|_| r.next_normal(0.0, 1.0)).collect::<Vec<_>>()
        };
        assert_eq!(seq("obs_noise"), seq("obs_noise"));
        assert_ne!(seq("obs_noise"), seq("resource_patchy"));

        // Moments over a large sample: mean ≈ 0, sd ≈ 1.
        let mut r = open(&example_e_key(7), "obs_noise");
        let n = 50_000;
        let xs: Vec<f64> = (0..n).map(|_| r.next_normal(0.0, 1.0)).collect();
        let mean = xs.iter().sum::<f64>() / n as f64;
        let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        assert!(mean.abs() < 0.03, "mean {mean}");
        assert!((var.sqrt() - 1.0).abs() < 0.03, "sd {}", var.sqrt());

        // sigma = 0 ⇒ exactly the mean; scaling is linear in |sigma|.
        let mut r = open(&example_e_key(2), "z");
        assert_eq!(r.next_normal(4.0, 0.0), 4.0);
    }
}
