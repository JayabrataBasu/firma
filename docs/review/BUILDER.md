# FIRMA builder protocol
- At session start read docs/review/STATE.md and the newest file in
  docs/review/inbox/. Do only what the latest reviewer prompt authorises.
- Stay silent between tool calls. Chat output is a short digest; raw output
  goes to docs/review/evidence/R<nn>/ with sha256.
- On finishing write docs/review/outbox/R<nn>-<slug>-report.md:
  1 Task as understood. 2 Files changed (path, +/- lines). 3 Tests added,
  each with raw before/after (a failing test first, for every fix).
  4 Full suite summary (cargo test --workspace, clippy -D warnings, fmt
  --check, architecture lints, check_deps.py, pytest). 5 Claims made, each
  with an evidence path. 6 What I did NOT verify. 7 Anything outside scope I
  noticed. 8 Three things I am least sure about.
- Never edit docs/review/inbox/, STATE.md, or decisions/.
- If a manual-unspecified choice is needed or scope must grow, stop and put
  the question in the report. Never pick a default.
- Report negative results plainly. Never reword a failure as a pass. Never
  loosen a threshold, exclude a test, or revert to hide a symptom.
- Do not commit or push. Do not run reserved seeds 1-200. Diagnostics use
  seeds >= 910000 and DIAGNOSTIC-ONLY configs.
- Do not edit an Accepted ADR body. Do not edit sanity.rs or any test on the
  main checkout unless the prompt authorises exactly that.
