//! Canonical string keys for the kernel's opaque domain-state store (ADR 0022).
//!
//! The kernel holds per-agent / global scalars and lists under `&str` keys it
//! never interprets (§17 A1). Every producer (`decision.*`, `action.*`,
//! `constraint.*`) and consumer of a given field MUST use the constant here, so
//! a typo is a compile error, not a silently-absent read. This module is the
//! single registry of those names.

// --- per-agent integer scalars (`AdjustAgentInt` / `SetAgentInt`) ---

/// The action index 0–8 the decision procedure selected this tick (§12.3 →
/// §10.1 phases 4–5; ADR 0022). Written with `DeltaKind::SetAgentInt { field:
/// SELECTED_ACTION, .. }`, read by the `act_*` rules via
/// `View::agent_int(_, SELECTED_ACTION)`. Absent ⇒ the firm has not decided ⇒
/// the `act_*` rules no-op.
pub const SELECTED_ACTION: &str = "selected_action";

/// `q` — outstanding (undelivered contracted) obligation (§8.1). Per-firm
/// counter, **not** a conserved resource: rises on `contract`, falls on
/// `deliver` (§11.1 / §11.2).
pub const OBLIGATION: &str = "obligation";

/// `focus` (§8.4 `Attention`, §12.3 Step 2) as an integer code, written by
/// `decision.satisficing` for inspectability / offline R2: `-1` = NONE,
/// `0` = SURVIVAL, `1`/`2`/`3` = GOAL(1..3). `decision.random` never writes it
/// (ADR 0027). Not read back by the model — offline analysis keys off the
/// manifest's decision-plugin id.
pub const FOCUS: &str = "focus";

/// `w_eff` (§12.3 Step 3) — the effective search width the last
/// `decision.satisficing` tick used. Written for offline R2 (search-width
/// rigidity); not read back by the model.
pub const W_EFF: &str = "w_eff";

/// `v_1` from the previous tick — realised capital growth `r^L_t − r^L_{t-1}`
/// (§12.1). Written by `decision.aspiration_update` in `record`, read by
/// `decision.satisficing` in the next `decide` for `ς_1 = A_1 − v_1`. `i64`
/// (`r^L` is `i64`).
pub const REALIZED_CAPITAL_GROWTH: &str = "realized_capital_growth";

/// `r^L` at the end of the previous tick — the `r^L_{t-1}` baseline for the
/// next tick's `v_1`. Written and read only by `decision.aspiration_update`.
pub const PREV_TICK_CAPITAL: &str = "prev_tick_capital";

/// The tick of this firm's most recent `compliance` violation (§9.1, ADR 0029).
/// Written by the `enforce` rule on a first strike; read by `enforce` to decide
/// whether a new violation is a second strike within `T_c` ticks. Absent ⇒ no
/// prior violation.
pub const COMPLIANCE_LAST_VIOLATION_TICK: &str = "compliance_last_violation_tick";

// --- per-agent real scalars (`AdjustAgentReal`) ---

/// `c` — capability (§8.1), `f64 ∈ [0, 1]`. Raised by a matured
/// `invest_capability` effect (§11.1 action 4). Monotone non-decreasing in the
/// MVP (ADR 0018).
pub const CAPABILITY: &str = "capability";

/// `λ` — legitimacy (§8.1), `f64 ∈ [0, 1]`. Feeds `p_success` for shaping
/// actions (§11.2); lowered on a first `compliance` violation (§9.1).
pub const LEGITIMACY: &str = "legitimacy";

/// `u` — regulated-activity intensity (§8.1, ADR 0014): the trailing-window
/// mean of the regulated-production indicator. Read by `g_2` (§9.1 compliance)
/// and by `decision.satisficing`'s `h`; written by the Stage-5 `constrain`-phase
/// `u`-recompute rule. Absent ⇒ `0.0`.
pub const REGULATED_INTENSITY: &str = "regulated_intensity";

/// `A_1` — capital-growth aspiration (§12.1). Realised value `v_1 = r^L_t −
/// r^L_{t-1}`. `f64`. Seeded by the orchestrator; adapted by
/// `decision.aspiration_update` (`A ← A + α(v − A)`).
pub const ASPIRATION_CAPITAL_GROWTH: &str = "aspiration_capital_growth";

/// `A_2` — capability aspiration (§12.1). Realised value `v_2 = c_t`.
pub const ASPIRATION_CAPABILITY: &str = "aspiration_capability";

/// `A_3` — obligation-clearance aspiration (§12.1). Realised value
/// `v_3 = −q_t`.
pub const ASPIRATION_OBLIGATION_CLEARANCE: &str = "aspiration_obligation_clearance";

// --- E1 Arm A direct-manipulation pins (ADR 0054), `Intervention::SetAgentReal`
// targets read by `decision.satisficing`'s Step 1. `None` (the default, no
// existing config sets these) ⇒ `h_t`/`ς_j` computed exactly as before this
// ADR — additive, opt-in, behaviour-preserving (§20.5), the same posture as
// ADR-0047's shaping-success fields. `decision.satisficing` requires either
// none of these four keys present, or all four together (ADR-0054 Part A
// item 1) — a partial pin is a config error, not a partial feature. ---

/// Overrides `h_t` (the viability margin `decision.satisficing`'s Step 1
/// would otherwise compute via `standard_margin`) for Arm A's direct
/// manipulation (manual §30.4, ADR 0054). Read-only by `Satisficing::
/// apply()`; no other rule consults it — `constraint.enforce`'s real
/// violation detection is unaffected (ADR 0054 Q1/Q5).
pub const PINNED_MARGIN: &str = "pinned_margin";
/// Overrides `ς_1` (capital-growth shortfall) for Arm A (ADR 0054). Paired
/// with [`PINNED_SHORTFALL_CAPABILITY`]/[`PINNED_SHORTFALL_OBLIGATION_
/// CLEARANCE`] — all three or none.
pub const PINNED_SHORTFALL_CAPITAL_GROWTH: &str = "pinned_shortfall_capital_growth";
/// Overrides `ς_2` (capability shortfall) for Arm A (ADR 0054).
pub const PINNED_SHORTFALL_CAPABILITY: &str = "pinned_shortfall_capability";
/// Overrides `ς_3` (obligation-clearance shortfall) for Arm A (ADR 0054).
pub const PINNED_SHORTFALL_OBLIGATION_CLEARANCE: &str = "pinned_shortfall_obligation_clearance";

// --- global real scalars (`AdjustGlobalReal`) ---

/// `θ_limit` — permitted regulated-activity intensity (§8.2). Raised additively
/// by matured `lobby` effects (§11.2; ADR 0016).
pub const THETA_LIMIT: &str = "theta_limit";

/// `θ_cap` — minimum capability for regulated production (§8.2).
pub const THETA_CAP: &str = "theta_cap";

// --- global integer scalars (`AdjustGlobalInt`) ---

/// `θ_Q` — maximum permitted outstanding obligation (§8.2), `i64`. Raised by
/// matured `contract` effects (§11.2).
pub const THETA_Q: &str = "theta_q";

/// `π^I` — input price, also `acquire_input`'s cost (§8.3, §11.1). `i64` so
/// `r^L` arithmetic stays exact. Seeded by the orchestrator / environment.
pub const INPUT_PRICE: &str = "input_price";

/// `π^O` — output price (§8.3). `i64`. Seeded by the orchestrator / environment.
pub const OUTPUT_PRICE: &str = "output_price";

// --- per-agent lists (`PushAgentRecord` / `ReplaceAgentList`) ---

/// `Λ` — the lagged-effect queue (§8.1). Each entry is a canonical-JSON
/// [`LaggedRecord`](crate::LaggedRecord). Appended by `invest_capability` and
/// the shaping rules; drained by `resolve_lagged` (§10.1 phase 6).
pub const LAGGED_EFFECTS: &str = "lagged_effects";

/// `W` — the action window (§8.1). Each entry is a canonical-JSON
/// [`WindowEntry`](crate::WindowEntry). Appended and trimmed to `L_W` by the
/// `constrain` rule (§10.1 phase 7; ADR 0014, ADR 0028); read to compute `u`
/// via [`margin::u_from_window`](crate::margin::u_from_window).
pub const ACTION_WINDOW: &str = "action_window";

/// The firm's view of the environment this tick — a single canonical-JSON
/// [`EnvSnapshot`](crate::EnvSnapshot) written per agent by
/// `observation.{full,noisy,delayed}` in `observe` (§12.2; ADR 0035), read by
/// `decision.{satisficing,random}` in `decide`. Absent ⇒ the firm sees the
/// true global store (a run with no `Observation` plugin is byte-identical).
pub const OBSERVED_ENV: &str = "observed_env";

// --- global lists (`PushGlobalRecord`) ---

/// `G_t` — the relation graph's edge list (§8.3). Each entry is a
/// canonical-JSON [`Edge`](crate::Edge). Appended by matured `contract` /
/// `diversify` effects.
pub const RELATION_EDGES: &str = "relation_edges";

/// The global environment-observation history ring (§12.2, ADR 0035). Each
/// entry is a canonical-JSON [`EnvSnapshot`](crate::EnvSnapshot); appended once
/// per `observe` phase and trimmed by the `Observation` plugin so `delayed(k)`
/// can reach back `k` ticks. Absent unless an `Observation` plugin runs.
pub const ENV_HISTORY: &str = "env_history";

/// `Σ_t` — the active-shock set (§8.3). Each entry is a canonical-JSON
/// [`Shock`](crate::Shock) plus the plugin's applied-so-far bookkeeping,
/// maintained by `shock.{scheduled,stochastic}` in `environment` (§13.2; ADR
/// 0036). Logged for offline novelty N1/N2 (§13.3); the running model does not
/// read it.
pub const ACTIVE_SHOCKS: &str = "active_shocks";

// --- conserved resources (ledger, keyed by `ResourceKind`) ---

/// `r^L` — liquid capital, the run's first conserved resource (§7.2, §8.1).
pub const CAPITAL: &str = "capital";

/// `r^I` — input stock, the run's second conserved resource (§7.2, §8.1).
pub const INPUT: &str = "input";
