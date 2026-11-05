# R21 — changes of mind and things that went wrong

1. **First run crashed in a control, before any part A–D result.** A worker
   function belonging to `hetero_check.py` (loaded by path) could not be sent
   to a worker process. Two controls had printed; nothing else. Output kept
   as `attempt1_crashed_in_controls_*.txt`. I added a wrapper in
   `diag_cliff.py` and re-ran once. `diag_spec.json` unchanged (same sha256;
   the run refuses otherwise).
2. **One test was wrong, not the code.** `attribution_reading(0.1, 0.3)`
   failed because `3 * 0.1 > 0.3` in floating point. I rewrote the test with
   binary-exact values. The rule in the spec and code did not change. This
   happened after step 0 and before any run.
3. **I did not believe parts B and D at first.** B reproduced R20's numbers
   to every digit and the resource channel changed nothing at all. I checked
   that the treatments reached the engine (`treatments_applied.txt`,
   `price_path.txt`) before reporting them as findings. They did.
4. **Part A: the predeclared reading says "mixed", and I report that.** The
   per-run table shows a cleaner structure than the collapse indicator does
   (`a_description.txt`); I report it separately and labelled as not the
   predeclared metric.
5. **"The 20 collapse-prone shock seeds".** I could not tell which 20 were
   meant, so the spec lists all 30 in seed order, with no selection.
6. **Part C and the aspiration reading.** The prompt did not say which; both
   were run.
