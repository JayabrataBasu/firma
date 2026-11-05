# R12 — things I changed my mind on while building the slice

1. **Exit status.** First version exited 0 when everything ran. Changed to
   exit 3 ("ran to the stop point; no H1c verdict") so that a 0 can only ever
   mean a verdict was computed. Exit 1 is kept for a failed run or a failed
   known-answer control.
2. **How far to go before stopping.** I first considered stopping at once,
   because `firma_lab.runner` cannot express the prompt's seeds or the
   no-shock fork. I decided to build everything up to the H1c predictor and
   say plainly, in the script and the report, that the script launches the
   CLI itself. The stop the prompt names explicitly (h and shortfall) is
   honoured: no test is run on the real runs.
3. **Candidate h columns.** I first meant to leave them out. I kept two
   per-run h summaries that existing functions already return, as descriptive
   columns only, because the owner needs to see whether they vary before
   choosing one. No test uses them.
4. **Which firm-ticks are "live".** I first counted every decision. I switched
   to excluding the tick in which a firm dies, because that is what
   `firma_analysis::sanity` does, and the cross-check then agrees to 1e-15.
5. **The "h and shortfall covary" caveat** is printed as the task states it,
   marked "not checked here": I cannot check it without deriving shortfall.
