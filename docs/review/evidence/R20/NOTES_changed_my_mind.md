# R20 — changes of mind and judgement calls

1. **`a_spread`: absolute or relative.** "uniform within +/- a_spread of
   8.0" can mean 8.0 ± a_spread or 8.0 × (1 ± a_spread). I first leaned
   towards relative (the levels 0.10 / 0.25 / 0.50 look like fractions), then
   took the literal reading, absolute, and wrote that into the spec before
   any run. Under it the aspiration spread is small (7.5 to 8.5 at "high").
   The relative reading was not run.
2. **Whether to stop for the unspecified choices.** The prompt asks me to
   declare exact numbers; I declared every open choice in `hetero_spec.json`
   (`choices_made_by_the_builder_before_any_run`) rather than stopping. None
   was changed after a result.
3. **ADR wording.** The first draft's "what would falsify it" section listed
   conditions only. Since R20 already met two of them, I added one closing
   sentence saying so, so the DRAFT cannot be read as supported.
4. **Descriptive follow-up after the result.** `describe_results.py` reads
   the saved per-run tables (no new runs) to show which runs collapse, who
   dies in baselines, and what separates the Arm A action sequences.
5. Nothing crashed; the check ran once. Tests ran after step 0 (they execute
   5 runs per invocation).
