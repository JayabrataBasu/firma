# ADR 0025 — `b_λ` / `b_κ` are required shaping config, not serde-defaulted — supersedes ADR-0023's parameter-default note

**Status:** Accepted (2026-09-04)
**Phase:** 2 (Model), Stage 2 review → recorded at Stage 3
**Supersedes:** the "Negative, accepted" bullet in ADR-0023 that read
"`b_λ = 0.2`, `b_κ = 0.1` default with a `[D]`-not-calibrated caveat". Nothing
else in ADR-0023 changes.
**Relates to:** manual §11.2 (`p_success`), §11.3 property 3 ("model … exposed
in config"), §16.1; ADR-0021 (§16.1's silent-`P_q` gap made an explicit
required parameter); ADR-0023

## Context

Paperwork for a shipped change (Stage-2 review, 2026-09-04). ADR-0023 flagged
that `p_success`'s coefficients `b_λ`, `b_κ` (§11.2) have **no §16.1 value**
and chose to give `firma-domain::shaping::SuccessModel` serde defaults of
`0.2` / `0.1`. The Stage-2 review found this inconsistent with how the same
situation was handled one Stage earlier: §16.1 gives no value for the
`obligation` penalty `P_q` either, and ADR-0021 made `P_q` a **required**
parameter with no default, precisely so a run cannot silently depend on an
un-calibrated number. `b_λ` / `b_κ` were silently defaulted even inside
`ContractParams` / `DiversifyParams`, whose entire design intent (ADR-0023)
was "every field required — no §16.1 values".

## Decision

`firma-domain::shaping::SuccessModel.b_lambda` and `b_kappa` have **no
`#[serde(default)]`** — a config `success` block that omits either is a
deserialization error naming the missing field, not `0.2` / `0.1`.

The `[D]`-not-calibrated values `0.2` / `0.1` survive only as the named
constants `shaping::LOBBY_B_LAMBDA` / `LOBBY_B_KAPPA`, referenced **explicitly**
inside `LobbyParams::default()` — the same way `default_lobby_success()`
already supplies `p0` / `p_max` / `κ_ℓ` explicitly. So a bare `lobby` (no
params) still builds from the full §16.1 block; a *partial* `success` object
in a `lobby` config is rejected like any other; and `contract` / `diversify`
configs must state `b_λ` / `b_κ`.

`SuccessModel::validate()` still checks `b_λ, b_κ ≥ 0` (§11.2 "non-negative
coefficients").

## Alternatives

- **Keep the serde default** (ADR-0023's choice). Rejected: it lets a run
  silently depend on an un-calibrated number, the exact failure ADR-0021
  avoided for `P_q`. §11.3 property 3 requires the success model be "exposed
  in config" — a hidden default is the opposite.
- **Make them required everywhere including a bare `lobby`.** Rejected:
  `lobby`'s *other* parameters have §16.1 values and `LobbyParams::default()`
  is used by VT-7's reference config and by any ablation that wants "standard
  lobby". The default block supplies `b_λ` / `b_κ` explicitly and visibly
  (with the `[D]` comment right there), which meets the "no silent default"
  bar without making `lobby` unusable without a config.

## Consequences

- **Positive.** No run can depend on `b_λ` / `b_κ` without those values being
  in the config file (or, for `lobby` specifically, in an
  explicitly-commented default constructor). Consistent with `P_q`.
- **Negative, accepted.** `contract` / `diversify` configs are two fields
  longer. Given they already require ~8 parameters with no §16.1 basis, two
  more is proportionate.
- **Neutral.** No numerical output changed — VT-7's reference params now state
  `b_λ = 0.2` / `b_κ = 0.1` explicitly (the values the deleted default gave),
  so VT-7's reported success rates are byte-identical.

## Compliance

- `firma-domain::shaping::SuccessModel` — `b_lambda` / `b_kappa` have no
  `#[serde(default)]`; `LOBBY_B_LAMBDA` / `LOBBY_B_KAPPA` constants; used only
  in `default_lobby_success()`.
- Tests:
  `firma-domain::shaping::tests::success_model_b_coefficients_have_no_serde_default`;
  `firma-plugin-action-shaping::tests::b_coefficients_are_required_when_a_success_block_is_given`.
- Manual PATCH (already queued in ADR-0023): §16.1 gains `b_λ` / `b_κ` — when
  it does, `LobbyParams::default()`'s constants cite the §16.1 row and the
  `[D]` caveat is removed.
