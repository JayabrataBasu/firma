# firma-plugin-decision

The MVP `Decision`-category plugins (manual §12.3, §12.1, §20.3), landed in
Phase 2 Stage 3.

| id | what | phase | manual |
|---|---|---|---|
| `decision.satisficing` | **the mechanism under test** — attend / narrow / scan / first-satisfice | `decide` | §12.3 |
| `decision.random` | structural null — uniform over the admissible set | `decide` | §20.3, §28.4, ADR-0027 |
| `decision.aspiration_update` | `A ← A + α(v − A)` | `record` | §12.1 |

`decision.satisficing` is **fully deterministic** — Steps 1–5 draw no random
numbers (grep-checked). `decision.random` makes exactly one `mechanism`-stream
draw per firm per tick (`purpose_tag = "decision_random"`).

`h` is computed via `firma_domain::margin::standard_margin` (ADR-0026), so there
is no cross-plugin dependency on `firma-plugin-constraint` and the `g_j`
formulas are not duplicated.

`Attention` (§8.4) stays the ADR-0022 / ADR-0024 keyed-store interim: `focus`
and `w_eff` are written to `keys::FOCUS` / `keys::W_EFF` for offline R2, not
read back by the model.
