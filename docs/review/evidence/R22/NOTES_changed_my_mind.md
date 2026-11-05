# R22 — changes of mind and judgement calls

1. **How to score P1's "allow the grid resolution".** The band's edges on
   this grid (m = 0.150 and 0.250) are exactly the boundaries of the
   hypothesised open interval (theta_new = 0.75 and 0.65). I first thought of
   pooling the five levels in [0.150, 0.250]; I instead declared, before any
   run, that the two edge levels are reported but not counted, with per-level
   thresholds, and that the strict variant (edges counted as inside) is
   printed for information. Result: the declared rule holds; the strict
   variant does not (m = 0.150 has no compliance death, m = 0.250 has them in
   all 10 runs).
2. **"Deaths at the tick of first compliance strike."** A strike is a first
   violation, and death needs a second one, so deaths in the strike tick are
   expected to be 0. I report both that literal quantity and the number of
   compliance deaths in the first compliance-death tick.
3. **Fixed onset and magnitude via `shock.stochastic`** with degenerate
   ranges (onset 40..40, sd 0) rather than `shock.scheduled`, to keep the
   same plugin as every earlier round.
4. **Part 3 fork.** I replaced `resource.patchy` by `resource.constant` in
   the no-shock fork as well, so the pair differs only by the shock.
5. Nothing crashed; the diagnostic ran once. After the result I wrote one
   descriptive listing from the saved tables (`part2_compliance_runs.txt`),
   with no new runs.
