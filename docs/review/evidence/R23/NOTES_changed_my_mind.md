# R23 — changes of mind and judgement calls

1. **Reserved seeds.** Part 2 asks me to reproduce the gate's numbers
   "exactly", and the gate configs use seeds 55/2/3/4, 7/2/3/4 and 909/2/3/4,
   which fall in the reserved range 1-200 that BUILDER.md and STATE.md forbid
   running. I first assumed I would run them as written (an earlier round's
   ADR-0057 evidence did). I did not. Instead: the original numbers come from
   the existing tests' own output (`existing_test_output.txt`, part of the
   standard suite); the transcriptions run at seeds 910001..910004; a unit
   test rebuilds each config mechanically from the Rust source and requires
   equality; and each transcription is run at a second seed set to show the
   event log does not depend on the seeds. All three checks came out exact,
   so nothing was lost. Declared in the spec before any run.
2. **Which tick P4 is about.** "Preceded by a tick" could mean the tick
   before the strike or the decision in the strike tick itself (which
   precedes that tick's enforcement). I declared the latter and print T-1
   as well. In the event both give margin 0.20 and GOAL focus.
3. **`u` is not in the log.** I compute it in Python from the logged action
   window with the one-line formula of `margin::u_from_window`, and take the
   decision-time `h` from the plugin's existing diagnostic trace. I checked
   that switching the trace on leaves the event log byte-equal, and that
   `h_t < h_crit` agrees with SURVIVAL focus on every traced decision.
4. **P5 has three outcomes in my rule** (HOLDS / FALSIFIED / NEITHER),
   because the prompt's "holds" and "falsified" conditions are not
   complements. NEITHER did not occur.
5. **P6 per cell, not pooled.** Half the cells have no obligations and so
   cannot pass SC-3; a pooled share would cap at 0.5. I declared the per-cell
   reading and print the pooled share. Both are 0.
6. **Thin-capital firms** are ids 0 and 1, with capital 0.2 times the firm's
   own unrounded draw.
7. Nothing crashed; the diagnostic ran once.
