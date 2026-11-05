# R14 — things I changed my mind on

1. **Bit-equality of the reconstructed shortfall.** I first asserted that the
   Rust value equals the plugin's exactly. It does not: 12,438 of 33,972
   firm-ticks differ, by at most 3.6e-15. I replaced the assertion with a
   measurement and then found the cause (`float_difference_cause.txt`): the
   same arithmetic replayed with an exact float parser is bit-equal on three
   runs, so the residue comes from how the Rust side parses floats from the
   event log (`serde_json` without `float_roundtrip`), not from the formula.
2. **Exit status.** R12 used 3 for "no verdict". Now 0 means both verdict
   lines were computed, 1 a failed run/control, 2 that the analysis refused
   an arm's data. This run exits 2 because the null arm is refused.
3. **Candidate h columns** from R12 (final-state h; fraction below h_crit)
   are removed from the per-run table; the owner fixed the regressor.
4. **Known-answer frames** now carry the real frame's column names and a
   covariate, and go through the identical H1c call as the real data.
5. **Bootstrap resamples** default to the manual's 10,000 for every fit
   (R12 used 400 for the known-answer controls only). Run time 396 s.
