# R18 — changes of mind and things that went wrong

1. **First grid-2 run crashed before printing or saving any result.** The
   Stage-1 table printer formatted SC-5 with `:.3f`; SC-5 is `None` when no
   shaping action was committed. Controls (a) and (b) had printed; no Stage-1
   row had. Output kept as `attempt1_crashed_before_any_result_*.txt`. I
   fixed the printer (`n/a`), added a test, and re-ran once. `grid2.json`
   was not touched (same sha256 before and after; the run refuses otherwise).
   I saw no cell result between the two attempts.
2. **`measure()` workaround removed.** R16's `regime_search.py` added two
   empty columns to dodge the slice script's no-deaths crash. With that
   crash fixed at its source this round, the workaround is gone. Control (b)
   shows grid-1 numbers are unchanged.
3. **Descriptive follow-up after the result.** Seeing that all six passing
   cells gave identical Stage-2 numbers, I wrote
   `describe_passing_cells.py` to list death records and the survivors
   distribution. It re-runs 15 configs from the declared grid; it changes no
   target, no grid and no verdict.
4. **"Passes both" for a cell with three magnitudes** is not defined in the
   prompt. I declared, in `grid2.json` before any run, that a cell passes
   both if any one magnitude row passes T2. In the event all 18 rows pass,
   so the choice did not matter.
