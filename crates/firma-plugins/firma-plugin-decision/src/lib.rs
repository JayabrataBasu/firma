//! `firma-plugin-decision` — the MVP `Decision`-category plugins (manual §12.3,
//! §12.1, §20.3).
//!
//! * [`Satisficing`] (`decision.satisficing`) — **the mechanism under test**
//!   (§12.3). A faithful transcription of Steps 1–5: Evaluate `h` and every
//!   `ς_j`; Attend (SURVIVAL / GOAL(argmax ς) / NONE); Narrow (`ψ(h)`,
//!   `w_eff`); Scan in focus-priority order; select the **first satisficing**
//!   action within `w_eff` (not the argmax — §12.3 rule 1), with inadmissible
//!   actions not consuming scan budget (§11.4 / §12.3 rule 2), falling back to
//!   the first admissible action in priority order. It is **fully
//!   deterministic** given `(x, θ, e, A, ς, focus)` — Steps 1–5 draw no random
//!   numbers; `apply()` never touches `firma_rng` (grep-checked by
//!   `no_rng_in_satisficing`).
//! * [`DecisionRandom`] (`decision.random`) — the structural null (ADR 0027):
//!   uniform over the **admissible set**, ignoring focus / narrowing / priority
//!   / satisficing entirely. Draws once per firm from the `mechanism` stream,
//!   `purpose_tag = "decision_random"`.
//! * [`AspirationUpdate`] (`decision.aspiration_update`) — the §12.1 adaptive
//!   update `A_{j,t+1} = A_j + α(v_j − A_j)`, run in the `record` phase (§10.1
//!   phase 9 — "Aspirations update"). A separate `Rule`: different phase,
//!   different concern, and it also persists `v_1`'s `r^L_{t-1}` baseline that
//!   [`Satisficing`] reads the next tick.
//!
//! All three read `θ` and the environment prices from the kernel's opaque
//! global store (`firma_domain::keys`); a run **MUST** seed `θ` (ADR 0015).
//! `Attention` (§8.4) stays the keyed-store interim (ADR 0022 Decision 1 /
//! ADR 0024): `focus` and `w_eff` are written by [`Satisficing`] under
//! `keys::FOCUS` / `keys::W_EFF` for offline R2 (§14.2), not read back by the
//! model.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use firma_core::{
    AgentId, ComponentId, ConflictClass, Delta, DeltaKind, DeltaKindTag, DeltaTarget, Phase,
    PluginId, ResourceKind, RngKey, Rule, View,
};
use firma_domain::dynamics::{
    market_core, shaping_cost_step, time_to_boundary, ActionParams, EnvParams, MarketAction,
    MARKET_ACTIONS,
};
use firma_domain::shaping::{ContractParams, LobbyParams};
use firma_domain::{
    keys, margin::standard_margin, Aspirations, ConstraintContext, ConstraintParams, FirmAuxState,
    FirmState, RelationGraph, ScaleFactors,
};

/// This crate's build version (new crate; `1.0.0` like the action plugins).
#[must_use]
pub fn version() -> semver::Version {
    semver::Version::new(1, 0, 0)
}

// ============================================================================
// TEMPORARY DIAGNOSTIC TRACE — H3 lobby/contract non-selection investigation.
//
// Added per the owner's instruction to observe, with direct evidence, why
// ADR-0047–0049's shaping-lookahead mechanism essentially never gets
// selected: whether the scan never reaches lobby/contract's checklist
// position, reaches it but the ADR-0048 time-margin gate never opens,
// reaches it with the gate open but still loses the payoff comparison, or a
// mix. This module and its call sites are **strictly observational**: every
// `trace::emit` call is a side effect only (an optional write to stderr or a
// file named by `FIRMA_TRACE_DECISION_PATH`), gated behind the
// `FIRMA_TRACE_DECISION=1` environment variable, and never changes a
// return value, a branch taken, or the `Vec<Delta>` `Satisficing::apply`
// produces. No threshold, cost, gate condition, or scan order is touched.
//
// Deliberately **not** `#[cfg(test)]`-gated: the diagnosis instruction asks
// for traces from full `sanity.rs` scenario runs (`cfg_arm_b_satisficing`,
// `cfg_wide_search`), which are ordinary `#[test]` functions compiled in the
// normal (non-cfg-test-attribute) build of this crate — a `#[cfg(test)]`
// gate here would not reach them from an external `tests/` crate. A runtime
// env-var gate was chosen instead of a Cargo feature so the "tracing off vs
// on, same binary, diff the hash" check the instruction asks for needs no
// rebuild between the two runs. This is temporary, diagnostic-only code —
// strip this module and its four call sites (search
// `FIRMA_TRACE_DECISION`/`trace::`) before merging this branch.
// ============================================================================
mod trace {
    use std::io::Write;
    use std::sync::{Mutex, OnceLock};

    /// `true` iff `FIRMA_TRACE_DECISION=1` is set. Checked once per process.
    pub(crate) fn is_enabled() -> bool {
        static ENABLED: OnceLock<bool> = OnceLock::new();
        *ENABLED.get_or_init(|| std::env::var("FIRMA_TRACE_DECISION").as_deref() == Ok("1"))
    }

    fn sink() -> &'static Mutex<Box<dyn Write + Send>> {
        static SINK: OnceLock<Mutex<Box<dyn Write + Send>>> = OnceLock::new();
        SINK.get_or_init(|| {
            let w: Box<dyn Write + Send> = match std::env::var("FIRMA_TRACE_DECISION_PATH") {
                Ok(path) if !path.is_empty() => Box::new(
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&path)
                        .unwrap_or_else(|e| panic!("FIRMA_TRACE_DECISION_PATH={path}: {e}")),
                ),
                _ => Box::new(std::io::stderr()),
            };
            Mutex::new(w)
        })
    }

    /// Write one newline-delimited JSON record. No-op unless `is_enabled()`.
    /// Never panics on a write failure (a full disk must not perturb a run
    /// that happens to be diagnosed) — the write error is silently dropped.
    pub(crate) fn emit(v: serde_json::Value) {
        if !is_enabled() {
            return;
        }
        if let Ok(mut w) = sink().lock() {
            let _ = writeln!(w, "{v}");
        }
    }
}

/// Plugin ids and declared content hashes (Phase-1 style — no artefact hashing).
pub mod catalog {
    /// `decision.satisficing`.
    pub const SATISFICING_ID: &str = "decision.satisficing";
    /// `decision.random`.
    pub const RANDOM_ID: &str = "decision.random";
    /// `decision.aspiration_update`.
    pub const ASPIRATION_UPDATE_ID: &str = "decision.aspiration_update";

    /// Declared content hash for the decision build.
    pub const CONTENT_HASH: &str = "phase2s3-decision-v1";
}

// --------------------------------------------------------------------------
// §12.3 Step 2 — focus
// --------------------------------------------------------------------------

/// `focus` (§12.3 Step 2). `Goal` carries `j ∈ {1, 2, 3}` (§12.1 goal index).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// `h < h_crit`.
    Survival,
    /// `max_j ς_j > 0`; `j` = `argmax` (ties → lowest).
    Goal(u8),
    /// Neither — the inertia case.
    None,
}

impl Focus {
    /// Integer code stored under `keys::FOCUS` for offline analysis: `-1` NONE,
    /// `0` SURVIVAL, `1..=3` GOAL(j).
    #[must_use]
    pub fn code(self) -> i64 {
        match self {
            Focus::None => -1,
            Focus::Survival => 0,
            Focus::Goal(j) => i64::from(j),
        }
    }

    /// §12.3 **Step 2 (Attend)**, as a pure function of `h` and the three
    /// shortfalls `ς_j` — the two quantities VT-8 (§25.4) must show are
    /// independently manipulable (ADR 0040):
    ///
    /// ```text
    /// if h < h_crit                 -> SURVIVAL
    /// else if max_j ς_j > 0         -> GOAL(argmax_j ς_j, ties -> lowest j)
    /// else                         -> NONE
    /// ```
    ///
    /// **`h` and `shortfalls` are separate parameters and the body computes
    /// neither from the other** — it branches on `h`, then branches on
    /// `shortfalls`. This is the type-level half of VT-8 criterion (iii).
    #[must_use]
    pub fn attend(h: f64, shortfalls: [f64; 3], h_crit: f64) -> Focus {
        if h < h_crit {
            Focus::Survival
        } else {
            let mut best = 0usize;
            for j in 1..3 {
                if shortfalls[j] > shortfalls[best] {
                    best = j;
                }
            }
            if shortfalls[best] > 0.0 {
                Focus::Goal((best + 1) as u8)
            } else {
                Focus::None
            }
        }
    }

    /// §12.3 Step 4 priority order (action indices 0–8) for this focus.
    /// `NONE` has no scan order — it repeats the previous action.
    #[must_use]
    pub fn scan_order(self) -> &'static [u8] {
        match self {
            //                    §12.3 Step 4 table, transcribed exactly
            Focus::Survival => &[1, 3, 5, 2, 0, 4, 6, 7, 8],
            Focus::Goal(1) => &[2, 1, 3, 6, 8, 5, 4, 0, 7],
            Focus::Goal(2) => &[4, 1, 3, 2, 6, 5, 0, 8, 7],
            Focus::Goal(3) => &[5, 3, 1, 7, 2, 8, 6, 0, 4],
            Focus::Goal(_) => &[], // unreachable — j is 1..=3 by construction
            Focus::None => &[],
        }
    }
}

// --------------------------------------------------------------------------
// §12.3 Step 3 — narrowing
// --------------------------------------------------------------------------

/// `ψ(h)` (§12.3 Step 3): `1` when `h ≥ h_crit`; `(max(h,0)/h_crit)^β` when
/// `h < h_crit`.
///
/// The `max(h, 0)` handles `h < 0` (a firm past the viability boundary, under
/// SURVIVAL focus): for `β > 0` it yields `ψ = 0` ⇒ `w_eff = 1` (the tightest
/// possible narrowing — scan only the single highest-priority survival
/// action). For `β = 0`, `0.0_f64.powf(0.0) == 1.0` (IEEE 754), so `ψ ≡ 1` —
/// **no narrowing**, exactly as "β = 0 is the null" requires; there is no
/// special-cased `β == 0` branch, the formula produces it.
#[must_use]
pub fn psi(h: f64, h_crit: f64, beta: f64) -> f64 {
    if h >= h_crit {
        1.0
    } else {
        (h.max(0.0) / h_crit).powf(beta)
    }
}

/// `w_eff = max(1, ⌈w_max · ψ(h)⌉)` (§12.3 Step 3). `ψ ∈ [0, 1]` for every
/// input above, so the result is in `1..=w_max`.
#[must_use]
pub fn w_eff(psi_h: f64, w_max: u32) -> u32 {
    let raw = (f64::from(w_max) * psi_h).ceil().max(1.0);
    // raw is finite in [1, w_max]; the cast is exact.
    (raw as u32).max(1)
}

// --------------------------------------------------------------------------
// §12.3 Steps 2–5 as one pure function — the VT-8 seam (ADR 0040)
// --------------------------------------------------------------------------

/// The result of one run of §12.3 Steps 2–5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    /// The §12.3 Step 2 focus.
    pub focus: Focus,
    /// The selected action index `0..=8`.
    pub action: u8,
    /// `w_eff` — the effective search width used (`0` on a `NONE` / inertia
    /// tick, which does no scan).
    pub w_eff: u32,
}

/// §12.3 **Steps 2–5 (Attend, Narrow, Scan order, Satisficing selection)** as
/// a single pure function (ADR 0040).
///
/// **`h` and `shortfalls` enter as independent typed parameters and the body
/// contains no path from either to the other** — `Focus::attend` branches on
/// each in turn; `psi` / [`w_eff`] read **only** `h`; the scan reads only the
/// `admissible` / `satisfices` closures and `focus`. This makes VT-8 criterion
/// (iii) ("no path in the decision procedure computing one from the other") a
/// fact you can check by reading this signature and these ~15 lines, not by
/// code archaeology (§17 A3 posture applied to a validation test).
///
/// The **normal** [`Satisficing::apply`] path computes the real `h`
/// ([`standard_margin`](firma_domain::margin::standard_margin)) and the real
/// `ς_j` (`A_j − v_j`) from firm state and calls this; the **VT-8 harness**
/// calls it directly with grid-constructed `(h, ς)` pairs that never touch
/// firm state. `apply` has **no Step-2–5 logic of its own** — it computes the
/// two quantities, builds the two closures, and calls here — so the two paths
/// cannot drift.
///
/// * `prev_action` — the action to repeat on a `NONE` (inertia) tick.
/// * `admissible(a)` — §11.4 admissibility of action `a`.
/// * `satisfices(a, focus)` — §12.3 Step 5's `satisfices` test (never called
///   for `Focus::None`).
///
/// Every input is a distinct named parameter on purpose — that is what makes
/// VT-8 criterion (iii) checkable by reading this one signature (ADR 0040), so
/// `clippy::too_many_arguments` is silenced rather than bundling `h` and
/// `shortfalls` behind a struct that would hide the seam.
#[allow(clippy::too_many_arguments)]
pub fn select(
    h: f64,
    shortfalls: [f64; 3],
    beta: f64,
    h_crit: f64,
    w_max: u32,
    prev_action: u8,
    admissible: impl Fn(u8) -> bool,
    satisfices: impl Fn(u8, Focus) -> bool,
) -> Selection {
    // Step 2.
    let focus = Focus::attend(h, shortfalls, h_crit);
    if focus == Focus::None {
        // Inertia (§12.3 Step 4 NONE row): repeat the previous action, no scan.
        return Selection {
            focus,
            action: prev_action,
            w_eff: 0,
        };
    }

    // Step 3 — narrowing. Reads only `h`.
    let w = w_eff(psi(h, h_crit, beta), w_max);

    // Steps 4–5 — scan in focus-priority order for the first satisficing
    // action within `w`; inadmissible actions do not consume budget (§12.3
    // rule 2); fall back to the first admissible action in priority order.
    let order = focus.scan_order();
    let mut scanned = 0u32;
    let mut pick: Option<u8> = None;
    for &a in order {
        if scanned >= w {
            break;
        }
        if !admissible(a) {
            continue;
        }
        scanned += 1;
        if satisfices(a, focus) {
            pick = Some(a); // first satisficing, not argmax (§12.3 rule 1)
            break;
        }
    }
    let action =
        pick.unwrap_or_else(|| order.iter().copied().find(|&a| admissible(a)).unwrap_or(0));

    Selection {
        focus,
        action,
        w_eff: w,
    }
}

// --------------------------------------------------------------------------
// shared: reading firm state from the View
// --------------------------------------------------------------------------

fn res(name: &str) -> ResourceKind {
    ResourceKind(name.to_owned())
}

/// §8.1 constraint-carrying state from the view.
fn firm_state(view: &dyn View, agent: AgentId) -> FirmState {
    FirmState {
        liquid_capital: view.agent_stock(agent, &res(keys::CAPITAL)),
        input_stock: view.agent_stock(agent, &res(keys::INPUT)),
        capability: view.agent_real(agent, keys::CAPABILITY).unwrap_or(0.0),
        obligation: view.agent_int(agent, keys::OBLIGATION).unwrap_or(0),
    }
}

/// The action window `W` (ADR 0014, ADR 0028), parsed. **ADR 0049**: also
/// the starting point for `time_to_boundary`'s projected window — one
/// parse, two consumers.
fn firm_window(view: &dyn View, agent: AgentId) -> Vec<firma_domain::WindowEntry> {
    view.agent_records(agent, keys::ACTION_WINDOW)
        .iter()
        .filter_map(|s| firma_domain::WindowEntry::from_json(s).ok())
        .collect()
}

/// `u` (regulated-activity intensity, §9.1 `g_2`) for one firm — derived on
/// demand from the action window `W` (ADR 0014, ADR 0028), falling back to a
/// seeded `keys::REGULATED_INTENSITY` while `W` is still empty (tick 0).
fn firm_u(view: &dyn View, agent: AgentId, l_w: usize) -> f64 {
    let w = firm_window(view, agent);
    if w.is_empty() {
        view.agent_real(agent, keys::REGULATED_INTENSITY)
            .unwrap_or(0.0)
    } else {
        firma_domain::margin::u_from_window(&w, l_w)
    }
}

/// **ADR 0049.** The firm's real, already-committed `Λ` queue at this tick
/// (not hypothetical) — `time_to_boundary`'s "already-pending effects"
/// input, parsed the same way `resolve_lagged`'s real `apply` parses it.
fn firm_pending_effects(view: &dyn View, agent: AgentId) -> Vec<firma_domain::LaggedRecord> {
    view.agent_records(agent, keys::LAGGED_EFFECTS)
        .iter()
        .filter_map(|s| firma_domain::LaggedRecord::from_json(s).ok())
        .collect()
}

/// §8.1 auxiliary state relevant to `g_j`: `λ` and `u` (aspirations are not a
/// `g_j` input, so they are zeroed here — `standard_margin` ignores them).
fn firm_aux(view: &dyn View, agent: AgentId, l_w: usize) -> FirmAuxState {
    FirmAuxState {
        legitimacy: view.agent_real(agent, keys::LEGITIMACY).unwrap_or(1.0),
        regulated_intensity: firm_u(view, agent, l_w),
        aspirations: Aspirations {
            capital_growth: 0.0,
            capability: 0.0,
            obligation_clearance: 0.0,
        },
    }
}

/// The firm's *observed* environment/θ this tick (ADR 0035): the single
/// [`EnvSnapshot`] an `Observation` plugin wrote under `keys::OBSERVED_ENV` in
/// `observe`, or `None` when no `Observation` plugin ran — in which case
/// [`theta`] / [`env_params`] fall back to the true global store and the run is
/// byte-identical to a pre-Stage-5 run.
fn observed_env(view: &dyn View, agent: AgentId) -> Option<firma_domain::EnvSnapshot> {
    view.agent_records(agent, keys::OBSERVED_ENV)
        .last()
        .and_then(|s| firma_domain::EnvSnapshot::from_json(s).ok())
}

/// §8.2 `θ` **as `agent` sees it** (§12.2 via ADR 0035): the observed snapshot
/// if present, else the true global store. **MUST be seeded** (ADR 0015); an
/// unseeded `θ_limit` / `θ_Q` of `0` makes `compliance` / `obligation` bind at
/// once.
fn theta(view: &dyn View, agent: AgentId) -> ConstraintParams {
    let o = observed_env(view, agent);
    ConstraintParams {
        theta_limit: o
            .map(|o| o.theta_limit)
            .or_else(|| view.global_real(keys::THETA_LIMIT))
            .unwrap_or(0.0),
        theta_cap: o
            .map(|o| o.theta_cap)
            .or_else(|| view.global_real(keys::THETA_CAP))
            .unwrap_or(0.0),
        theta_q: o
            .map(|o| o.theta_q)
            .or_else(|| view.global_int(keys::THETA_Q))
            .unwrap_or(0),
    }
}

/// §8.3 environment prices **as `agent` sees it** (ADR 0035): the observed
/// snapshot if present, else the true global store. Unseeded ⇒ `0`.
fn env_params(view: &dyn View, agent: AgentId) -> EnvParams {
    let o = observed_env(view, agent);
    EnvParams {
        input_price: o
            .map(|o| o.input_price)
            .or_else(|| view.global_int(keys::INPUT_PRICE))
            .unwrap_or(0),
        output_price: o
            .map(|o| o.output_price)
            .or_else(|| view.global_int(keys::OUTPUT_PRICE))
            .unwrap_or(0),
    }
}

// --------------------------------------------------------------------------
// shaping-scan configuration (shared by Satisficing and DecisionRandom)
// --------------------------------------------------------------------------

/// Per-firm `ShapingCapability` (§8.4) for scan/admissibility purposes: the
/// minimum commitment `κ_a` of each shaping action the firm can attempt. A
/// shaping action with **no configured cost** is treated as **absent from the
/// repertoire** — inadmissible, and (per §12.3 rule 2) it does not consume
/// scan budget. `contract` / `diversify` have no §16.1 values (ADR 0023), so
/// they are opt-in; `lobby`'s `κ_ℓ = 25` (§16.1) is the default.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShapingScanParams {
    /// `κ_ℓ` — lobby cost (§16.1 default `25`).
    #[serde(default = "default_lobby_cost")]
    pub lobby_cost: i64,
    /// `κ_k` — contract cost. `None` ⇒ `contract` not in repertoire.
    #[serde(default)]
    pub contract_cost: Option<i64>,
    /// `κ_d` — diversify cost. `None` ⇒ `diversify` not in repertoire.
    #[serde(default)]
    pub diversify_cost: Option<i64>,
    /// **ADR 0047 (H3 revision).** `lobby`'s own declared success model and
    /// `δ_θ` payoff — the *same* `LobbyParams` shape
    /// `action.shaping.rdt_standard.lobby` is configured with (a config
    /// author sets both to the same values so the firm's belief matches
    /// what will actually happen). `None` (the default, and every config
    /// that predates this ADR) ⇒ `satisfices()`'s `SURVIVAL` test for
    /// `lobby` stays exactly the pre-ADR-0047 cost-only lookahead —
    /// additive, opt-in, behaviour-preserving by default (§20.5).
    #[serde(default)]
    pub lobby_success: Option<LobbyParams>,
    /// **ADR 0047 (H3 revision).** `contract`'s own declared success model
    /// and `(δ_Q, q_0)` payoff, mirroring `lobby_success` above. `None` ⇒
    /// cost-only, unchanged.
    #[serde(default)]
    pub contract_success: Option<ContractParams>,
    /// **ADR 0048 (H3 revision, round 2).** Whether `SURVIVAL`'s shaping
    /// evaluation must *also* clear a race against `time_to_boundary`
    /// (§14.3) before its expected-relief calculation (ADR 0047) is used at
    /// all — `Δ_min ≥ time_to_boundary` (the payoff cannot possibly arrive
    /// before the projected boundary) falls back to the pre-ADR-0047
    /// cost-only test, exactly implementing H3's "increases narrowing"
    /// direction as a real mechanical consequence, not an emergent one.
    /// **Defaults to `true`** (ADR 0048's Decision explains why the
    /// time-aware behaviour, not the payoff-only one, is the standard going
    /// forward). Set `false` to fall back to ADR-0047's payoff-only test
    /// (an explicit ablation / like-for-like comparison switch) — a config
    /// change, never a code change.
    #[serde(default = "default_require_time_margin")]
    pub require_time_margin: bool,
}

fn default_lobby_cost() -> i64 {
    25
}

fn default_require_time_margin() -> bool {
    true
}

impl Default for ShapingScanParams {
    fn default() -> Self {
        ShapingScanParams {
            lobby_cost: default_lobby_cost(),
            contract_cost: None,
            diversify_cost: None,
            lobby_success: None,
            contract_success: None,
            require_time_margin: default_require_time_margin(),
        }
    }
}

impl ShapingScanParams {
    /// `κ_a` for shaping action index `a ∈ {6, 7, 8}`, or `None` if `a` is not
    /// in the firm's repertoire.
    fn cost(&self, a: u8) -> Option<i64> {
        match a {
            6 => Some(self.lobby_cost),
            7 => self.contract_cost,
            8 => self.diversify_cost,
            _ => None,
        }
    }
}

// --------------------------------------------------------------------------
// admissibility (§11.4 / §12.3 `admissible(a, x, θ, e)`; ADR 0021 Decision 2)
// --------------------------------------------------------------------------

/// Snapshot of everything `admissible` / `satisfices` need for one firm this
/// tick — built once, reused across the whole scan.
struct DecideCtx {
    agent: AgentId,
    state: FirmState,
    theta: ConstraintParams,
    env: EnvParams,
    action: ActionParams,
    shaping: Option<ShapingScanParams>,
    graph: RelationGraph,
    /// **ADR 0048.** Last tick's `SELECTED_ACTION` (default `hold`, `0`) —
    /// `time_to_boundary`'s "current action" for the race check. The same
    /// value and the same default `select()`'s `NONE`-focus inertia
    /// fallback already uses (§12.3), not a second definition of "what the
    /// firm is currently doing."
    prev_action: u8,
    /// **ADR 0049.** The real action window `W`, for `time_to_boundary`'s
    /// projection to advance forward — the same raw entries `firm_u` reads.
    window: Vec<firma_domain::WindowEntry>,
    /// **ADR 0049.** `L_W`, so the projection's window trims exactly as
    /// `constraint.action_window`'s real `apply` would.
    l_w: usize,
    /// **ADR 0049.** The firm's real, already-committed `Λ` queue — facts,
    /// not hypotheticals, for `time_to_boundary` to apply at their real
    /// `maturity_tick`s.
    pending: Vec<firma_domain::LaggedRecord>,
    /// **ADR 0049.** The current tick — `time_to_boundary`'s `start_tick`,
    /// so a `pending` effect's absolute `maturity_tick` compares correctly.
    tick: u64,
}

impl DecideCtx {
    /// `admissible(a, x, θ, e)` — in repertoire ∧ affordable ∧ (scope-gated:
    /// `c ≥ θ_cap`) ∧ (contract: has a `supply` partner). For the six market
    /// actions this is exactly `market_core(a, …).is_some()` (ADR 0021 D2).
    fn admissible(&self, a: u8) -> bool {
        if a < 6 {
            let action = MARKET_ACTIONS[a as usize];
            market_core(action, &self.state, &self.theta, &self.env, &self.action).is_some()
        } else {
            let Some(scan) = &self.shaping else {
                return false; // no ShapingCapability configured
            };
            let Some(cost) = scan.cost(a) else {
                return false; // this shaping action not in repertoire
            };
            if shaping_cost_step(cost, &self.state).is_none() {
                return false; // cannot afford κ_a
            }
            // `contract` also needs an incoming `supply` edge (§11.2 action 7).
            a != 7 || self.graph.has_supply_partner(self.agent)
        }
    }

    /// The post-action `FirmState` a one-step lookahead expects (§12.3 Step 5,
    /// "the deterministic core with stochastic terms at expectation"):
    /// `market_core` for market actions (its own §9.3 lag-collapse
    /// approximation included, e.g. `invest_capability`), `shaping_cost_step`
    /// (cost only) for shaping. `None` iff `!admissible(a)`.
    fn lookahead(&self, a: u8) -> Option<FirmState> {
        if a < 6 {
            market_core(
                MARKET_ACTIONS[a as usize],
                &self.state,
                &self.theta,
                &self.env,
                &self.action,
            )
        } else {
            let cost = self.shaping.as_ref().and_then(|s| s.cost(a))?;
            shaping_cost_step(cost, &self.state)
        }
    }

    /// `h` for a given `FirmState` under this firm's `λ` / `u` / `θ` (the aux
    /// `u` is held at its current value — §9.3: `u` tracking is not part of the
    /// `FirmState → FirmState` core).
    fn margin_at(&self, s: &FirmState, aux: &FirmAuxState, scales: &ScaleFactors) -> f64 {
        Self::margin_with_theta(s, aux, &self.theta, scales)
    }

    fn margin_with_theta(
        s: &FirmState,
        aux: &FirmAuxState,
        theta: &ConstraintParams,
        scales: &ScaleFactors,
    ) -> f64 {
        let ctx = ConstraintContext {
            state: s,
            aux,
            theta,
        };
        standard_margin(&ctx, scales)
    }

    /// **ADR 0047/0048 (H3 revision).** `SURVIVAL`'s `Expected h_{t+1}` for a
    /// shaping action `a ∈ {6, 7}` whose success model is configured
    /// (`ShapingScanParams::lobby_success` / `contract_success`), taken as a
    /// proper expectation over the Bernoulli success draw — "stochastic
    /// terms at expectation" (§12.3), applied to shaping specifically:
    ///
    /// `E[h_{t+1}] = p_success · h(state_after_payoff, θ_after_payoff)
    ///             + (1 − p_success) · h(state_after_cost, θ)`
    ///
    /// `p_success` is `firma_domain::shaping::SuccessModel::p_success` —
    /// the exact function `action.shaping.rdt_standard` calls at
    /// commitment, with the exact same `legitimacy` input (nothing between
    /// `decide` and `act_shaping` writes it). Cost is paid in both branches
    /// (§11.3 property 1: "cost paid at commitment, not at success").
    ///
    /// **ADR 0048's race check, gating the above** (when
    /// `ShapingScanParams::require_time_margin`, the default): the payoff is
    /// evaluated at all only if `Δ_min` (the action's *fastest possible*
    /// lag draw — H3's "*can* arrive in time", not "is guaranteed to") is
    /// strictly less than `time_to_boundary` under the firm's current
    /// action (§14.3, `firma_domain::dynamics::time_to_boundary`). If
    /// `Δ_min ≥ time_to_boundary`, the payoff cannot possibly arrive before
    /// the projected boundary — this returns `None` (the caller's cost-only
    /// fallback), mechanically implementing H3's "increases narrowing"
    /// direction rather than leaving it to emerge downstream. A `None`
    /// (uncapped/practically-unbounded) `time_to_boundary` is treated as
    /// "arrives comfortably" — no finite lag can fail to beat an
    /// unbounded horizon.
    ///
    /// `None` if `a` is not `{6, 7}`, not admissible, has no configured
    /// success model, or loses the race — callers fall back to the
    /// cost-only comparison.
    fn shaping_expected_survival_margin(
        &self,
        a: u8,
        aux: &FirmAuxState,
        scales: &ScaleFactors,
    ) -> Option<f64> {
        // DIAGNOSTIC (read-only, see `trace` module doc comment): scan reached
        // this shaping action's SURVIVAL-branch evaluation at all.
        if trace::is_enabled() {
            trace::emit(serde_json::json!({
                "kind": "shaping_eval_entered",
                "tick": self.tick, "agent": self.agent.0, "action": a,
            }));
        }
        let scan = self.shaping.as_ref()?;
        let cost = scan.cost(a)?;
        let after_cost = shaping_cost_step(cost, &self.state)?;
        if trace::is_enabled()
            && (a == 6 && scan.lobby_success.is_none() || a == 7 && scan.contract_success.is_none())
        {
            trace::emit(serde_json::json!({
                "kind": "shaping_no_success_model_configured",
                "tick": self.tick, "agent": self.agent.0, "action": a,
            }));
        }
        let (lag_min, p, theta_success, state_success) = match a {
            6 => {
                let lp = scan.lobby_success?;
                let p = lp.success.p_success(aux.legitimacy, cost);
                let mut theta = self.theta;
                theta.theta_limit += lp.delta_theta;
                (lp.lag.min, p, theta, after_cost)
            }
            7 => {
                let cp = scan.contract_success?;
                let p = cp.success.p_success(aux.legitimacy, cost);
                let mut theta = self.theta;
                theta.theta_q += cp.delta_q;
                let mut state = after_cost;
                state.obligation += cp.q0;
                (cp.lag.min, p, theta, state)
            }
            _ => return None, // diversify (8): no payoff representable in (FirmState, θ)
        };

        if scan.require_time_margin {
            let current_action = if self.prev_action < 6 {
                MARKET_ACTIONS[self.prev_action as usize]
            } else {
                MarketAction::Hold // ADR 0048: prev was shaping ⇒ assume no organic change
            };
            let ttb = time_to_boundary(
                current_action,
                &self.state,
                &self.theta,
                &self.env,
                aux.legitimacy,
                aux.regulated_intensity,
                &self.window,
                self.l_w,
                &self.pending,
                self.tick,
                &self.action,
                scales,
            );
            if trace::is_enabled() {
                let gate_open = ttb.is_none_or(|t| lag_min < t);
                trace::emit(serde_json::json!({
                    "kind": "shaping_gate",
                    "tick": self.tick, "agent": self.agent.0, "action": a,
                    "lag_min": lag_min, "time_to_boundary": ttb, "gate_open": gate_open,
                }));
            }
            if let Some(ttb) = ttb {
                if lag_min >= ttb {
                    return None; // cannot possibly arrive in time ⇒ cost-only fallback
                }
            }
            // ttb == None ⇒ unbounded ⇒ arrives comfortably; fall through.
        }

        let h_success = Self::margin_with_theta(&state_success, aux, &theta_success, scales);
        let h_failure = Self::margin_with_theta(&after_cost, aux, &self.theta, scales);
        if trace::is_enabled() {
            trace::emit(serde_json::json!({
                "kind": "shaping_payoff_computed",
                "tick": self.tick, "agent": self.agent.0, "action": a,
                "p_success": p, "h_success": h_success, "h_failure": h_failure,
                "e_h": p * h_success + (1.0 - p) * h_failure,
            }));
        }
        Some(p * h_success + (1.0 - p) * h_failure)
    }
}

// --------------------------------------------------------------------------
// decision.satisficing
// --------------------------------------------------------------------------

/// Parameters for [`Satisficing`] (§12.3, §16.1). `β` is **config**, not
/// hard-coded — `β = 0` (the null) must fall out of `ψ`'s formula.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SatisficingParams {
    /// `h_crit` — survival-attention threshold. §16.1 default `0.15`.
    #[serde(default = "default_h_crit")]
    pub h_crit: f64,
    /// `β` — narrowing sharpness. §16.1 default `1.0`; sweep `{0, 0.5, 1, 2, 4}`.
    #[serde(default = "default_beta")]
    pub beta: f64,
    /// `w_max` — max search width. §16.1 default `6`; sweep `{3, 6, 9}`.
    #[serde(default = "default_w_max")]
    pub w_max: u32,
    /// `L_W` — action-window length, for `u` in `g_2` (ADR 0014, ADR 0028).
    /// §16.1 default `8`; sweep `{4, 8, 16}`.
    #[serde(default = "default_l_w")]
    pub l_w: u32,
    /// `s_j` — §9.2 scale factors for `h`. §9.2 defaults.
    #[serde(default)]
    pub scales: ScaleFactors,
    /// §11.1 action parameters for the one-step lookahead. §16.1 defaults.
    #[serde(default)]
    pub action: ActionParams,
    /// Which shaping actions the firm can scan. `None` ⇒ none (default).
    #[serde(default)]
    pub shaping: Option<ShapingScanParams>,
}

fn default_h_crit() -> f64 {
    0.15
}
fn default_beta() -> f64 {
    1.0
}
fn default_w_max() -> u32 {
    6
}
fn default_l_w() -> u32 {
    8
}

impl Default for SatisficingParams {
    fn default() -> Self {
        SatisficingParams {
            h_crit: default_h_crit(),
            beta: default_beta(),
            w_max: default_w_max(),
            l_w: default_l_w(),
            scales: ScaleFactors::default(),
            action: ActionParams::default(),
            shaping: None,
        }
    }
}

impl SatisficingParams {
    /// Validate against §12.3 / §16.1.
    ///
    /// # Errors
    /// `h_crit ≤ 0`, `β < 0`, `w_max == 0`, or a non-finite real.
    pub fn validate(&self) -> Result<(), String> {
        if !(self.h_crit.is_finite() && self.h_crit > 0.0) {
            return Err(format!(
                "h_crit must be finite and > 0, got {}",
                self.h_crit
            ));
        }
        if !(self.beta.is_finite() && self.beta >= 0.0) {
            return Err(format!("beta must be finite and >= 0, got {}", self.beta));
        }
        if self.w_max == 0 {
            return Err("w_max must be >= 1".to_string());
        }
        if self.l_w == 0 {
            return Err("l_w must be >= 1".to_string());
        }
        Ok(())
    }
}

/// `decision.satisficing` — §12.3. Runs in `decide` for every live agent (a
/// per-agent `Strategic` marker is a Phase-5 component-bag concern; every live
/// agent is treated as `Strategic`).
pub struct Satisficing {
    id: PluginId,
    params: SatisficingParams,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}

impl Satisficing {
    /// Build from parameters.
    ///
    /// # Errors
    /// Invalid [`SatisficingParams`].
    pub fn new(params: SatisficingParams) -> Result<Satisficing, String> {
        params.validate()?;
        Ok(Satisficing {
            id: PluginId::new(catalog::SATISFICING_ID),
            params,
            reads: vec![ComponentId::ledger()],
            writes: vec![DeltaKindTag::SetAgentInt],
        })
    }

    /// The `[ς_1, ς_2, ς_3]` shortfalls (§12.1: `ς_j = A_j − v_j`). An
    /// **absent** aspiration ⇒ `ς_j = 0` (no shortfall for that goal yet).
    fn shortfalls(view: &dyn View, agent: AgentId, s: &FirmState) -> [f64; 3] {
        let v1 = view
            .agent_int(agent, keys::REALIZED_CAPITAL_GROWTH)
            .unwrap_or(0) as f64;
        let v2 = s.capability;
        let v3 = -(s.obligation as f64);
        let v = [v1, v2, v3];
        let a_keys = [
            keys::ASPIRATION_CAPITAL_GROWTH,
            keys::ASPIRATION_CAPABILITY,
            keys::ASPIRATION_OBLIGATION_CLEARANCE,
        ];
        let mut sc = [0.0; 3];
        for j in 0..3 {
            if let Some(a) = view.agent_real(agent, a_keys[j]) {
                sc[j] = a - v[j];
            }
        }
        sc
    }

    /// `satisfices(a, focus)` (§12.3 Step 5 table).
    fn satisfices(
        dc: &DecideCtx,
        aux: &FirmAuxState,
        scales: &ScaleFactors,
        a: u8,
        focus: Focus,
        h_t: f64,
        sc: &[f64; 3],
    ) -> bool {
        let Some(n) = dc.lookahead(a) else {
            return false;
        };
        match focus {
            // ADR 0047 (H3 revision): a shaping action with a configured
            // success model gets its *expected* h, not just its cost-only
            // lookahead — additive; falls through to the pre-ADR-0047
            // comparison (`n`, cost-only) when unconfigured (`None`), so
            // every config that predates this ADR is unaffected.
            Focus::Survival => {
                if a >= 6 {
                    if let Some(e_h) = dc.shaping_expected_survival_margin(a, aux, scales) {
                        let result = e_h > h_t;
                        if trace::is_enabled() {
                            trace::emit(serde_json::json!({
                                "kind": "shaping_payoff_comparison",
                                "tick": dc.tick, "agent": dc.agent.0, "action": a,
                                "e_h": e_h, "h_t": h_t, "result": result,
                            }));
                        }
                        return result;
                    }
                }
                dc.margin_at(&n, aux, scales) > h_t
            }
            Focus::Goal(j) => {
                let dv = match j {
                    1 => (n.liquid_capital - dc.state.liquid_capital) as f64,
                    2 => n.capability - dc.state.capability,
                    3 => (dc.state.obligation - n.obligation) as f64,
                    _ => return false,
                };
                dv >= sc[(j - 1) as usize]
            }
            Focus::None => false,
        }
    }
}

impl Rule for Satisficing {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> semver::Version {
        version()
    }
    fn phase(&self) -> Phase {
        Phase::Decide
    }
    fn reads(&self) -> &[ComponentId] {
        &self.reads
    }
    fn writes(&self) -> &[DeltaKindTag] {
        &self.writes
    }
    fn apply(&self, view: &dyn View, _key: RngKey) -> Vec<Delta> {
        let p = &self.params;
        let graph = RelationGraph::from_records(view.global_records(keys::RELATION_EDGES))
            .unwrap_or_default();
        let mut out = Vec::new();

        for &agent in view.live_agents() {
            let state = firm_state(view, agent);
            let aux = firm_aux(view, agent, p.l_w as usize);
            // `prev` (last tick's `SELECTED_ACTION`, defaulting to `hold`) is
            // needed *before* `dc` this round (ADR 0048): it is also
            // `time_to_boundary`'s "current action" input, the same default
            // `NONE`'s inertia fallback already uses — one definition, two
            // consumers, not invented twice.
            let prev = view
                .agent_int(agent, keys::SELECTED_ACTION)
                .and_then(|v| u8::try_from(v).ok())
                .filter(|a| *a <= 8)
                .unwrap_or(0);
            let dc = DecideCtx {
                agent,
                state,
                theta: theta(view, agent),
                env: env_params(view, agent),
                action: p.action,
                shaping: p.shaping,
                graph: graph.clone(),
                prev_action: prev,
                window: firm_window(view, agent),
                l_w: p.l_w as usize,
                pending: firm_pending_effects(view, agent),
                tick: view.tick().0,
            };

            // --- Step 1: Evaluate the two quantities VT-8 must show are
            //     independently manipulable (ADR 0040). `h_t` comes only from
            //     `standard_margin`; `sc` only from `A_j − v_j`. ---
            let h_t = dc.margin_at(&dc.state, &aux, &p.scales);
            let sc = Self::shortfalls(view, agent, &dc.state);

            // --- Steps 2–5: the pure decision function. `apply` carries no
            //     Step-2–5 logic of its own, so this path and the VT-8 harness
            //     cannot drift (ADR 0040). ---
            // DIAGNOSTIC (read-only, see `trace` module doc comment): the two
            // closures below are unchanged except for one `trace::emit` call
            // each, which is a no-op unless `FIRMA_TRACE_DECISION=1`. Neither
            // closure's return value is altered by the addition — `r` is
            // computed first, exactly as before, and returned unchanged.
            let sel = select(
                h_t,
                sc,
                p.beta,
                p.h_crit,
                p.w_max,
                prev,
                |a| {
                    let r = dc.admissible(a);
                    if trace::is_enabled() {
                        trace::emit(serde_json::json!({
                            "kind": "admissible_check",
                            "tick": dc.tick, "agent": dc.agent.0, "action": a, "result": r,
                        }));
                    }
                    r
                },
                |a, focus| {
                    let r = Self::satisfices(&dc, &aux, &p.scales, a, focus, h_t, &sc);
                    if trace::is_enabled() {
                        trace::emit(serde_json::json!({
                            "kind": "satisfices_check",
                            "tick": dc.tick, "agent": dc.agent.0, "action": a,
                            "focus": format!("{focus:?}"), "h_t": h_t, "result": r,
                        }));
                    }
                    r
                },
            );
            if trace::is_enabled() {
                trace::emit(serde_json::json!({
                    "kind": "selection",
                    "tick": dc.tick, "agent": dc.agent.0,
                    "focus": format!("{:?}", sel.focus), "w_eff": sel.w_eff,
                    "action": sel.action, "prev_action": prev, "h_t": h_t,
                    "shortfalls": sc, "scan_order": Focus::scan_order(sel.focus),
                }));
            }

            let set = |field: &str, value: i64| Delta {
                target: DeltaTarget::Agent(agent),
                kind: DeltaKind::SetAgentInt {
                    field: field.to_owned(),
                    value,
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            };
            out.push(set(keys::SELECTED_ACTION, i64::from(sel.action)));
            out.push(set(keys::FOCUS, sel.focus.code()));
            out.push(set(keys::W_EFF, i64::from(sel.w_eff))); // 0 on a NONE tick
        }
        out
    }
    fn assumption(&self) -> &str {
        "The firm attends to survival when its viability margin is thin and \
         otherwise to its largest unmet goal; it narrows its search as the \
         margin narrows (sharpness β); and it takes the first action it scans, \
         in a focus-dependent priority order, that it expects to be good enough \
         — not the best one (§12.3 satisficing, Cyert & March)."
    }
}

// --------------------------------------------------------------------------
// decision.random  (ADR 0027)
// --------------------------------------------------------------------------

/// Parameters for [`DecisionRandom`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RandomParams {
    /// §11.1 action parameters — for the admissibility check only. §16.1
    /// defaults.
    #[serde(default)]
    pub action: ActionParams,
    /// Which shaping actions are in the firm's repertoire. `None` ⇒ none.
    #[serde(default)]
    pub shaping: Option<ShapingScanParams>,
}

/// `decision.random` — the structural null (ADR 0027). Picks **uniformly among
/// the admissible actions**; ignores focus, narrowing, priority, and
/// satisficing entirely. One `mechanism`-stream draw per firm per `decide`.
pub struct DecisionRandom {
    id: PluginId,
    params: RandomParams,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}

impl DecisionRandom {
    /// Build from parameters.
    ///
    /// # Errors
    /// Never — `RandomParams` cannot be invalid; the signature matches the
    /// other constructors.
    pub fn new(params: RandomParams) -> Result<DecisionRandom, String> {
        Ok(DecisionRandom {
            id: PluginId::new(catalog::RANDOM_ID),
            params,
            reads: vec![ComponentId::ledger()],
            writes: vec![DeltaKindTag::SetAgentInt],
        })
    }
}

impl Rule for DecisionRandom {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> semver::Version {
        version()
    }
    fn phase(&self) -> Phase {
        Phase::Decide
    }
    fn reads(&self) -> &[ComponentId] {
        &self.reads
    }
    fn writes(&self) -> &[DeltaKindTag] {
        &self.writes
    }
    fn apply(&self, view: &dyn View, key: RngKey) -> Vec<Delta> {
        let graph = RelationGraph::from_records(view.global_records(keys::RELATION_EDGES))
            .unwrap_or_default();
        let mut out = Vec::new();

        for &agent in view.live_agents() {
            let dc = DecideCtx {
                agent,
                state: firm_state(view, agent),
                theta: theta(view, agent),
                env: env_params(view, agent),
                action: self.params.action,
                shaping: self.params.shaping,
                graph: graph.clone(),
                prev_action: 0, // unused here — `decision.random` never calls `satisfices`
                window: Vec::new(),
                l_w: 1,
                pending: Vec::new(),
                tick: 0,
            };

            // Admissible set, ascending index (deterministic). Never empty:
            // `hold` (0) is always admissible (§11.1).
            let adm: Vec<u8> = (0u8..=8).filter(|&a| dc.admissible(a)).collect();
            let n = adm.len().max(1) as u64;

            let mut rng = firma_rng::open_for(&key, Some(agent.0), "decision_random");
            let idx = rng.next_below(n) as usize;
            let chosen = adm.get(idx).copied().unwrap_or(0);

            out.push(Delta {
                target: DeltaTarget::Agent(agent),
                kind: DeltaKind::SetAgentInt {
                    field: keys::SELECTED_ACTION.to_owned(),
                    value: i64::from(chosen),
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            });
        }
        out
    }
    fn assumption(&self) -> &str {
        "The firm chooses uniformly at random among the actions it can currently \
         take, with no attention, no priority, and no notion of 'good enough' — \
         the structural null that shows the model's findings require its \
         decision mechanism (§20.3, §28.4)."
    }
}

// --------------------------------------------------------------------------
// decision.aspiration_update  (§12.1)
// --------------------------------------------------------------------------

/// Parameters for [`AspirationUpdate`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AspirationUpdateParams {
    /// `α` — aspiration adaptation rate, `α ∈ (0, 1)` (§12.1). §16.1 default
    /// `0.10`; sweep `{0.05, 0.1, 0.2}`.
    #[serde(default = "default_alpha")]
    pub alpha: f64,
}

fn default_alpha() -> f64 {
    0.10
}

impl Default for AspirationUpdateParams {
    fn default() -> Self {
        AspirationUpdateParams {
            alpha: default_alpha(),
        }
    }
}

/// `decision.aspiration_update` — §12.1's `A_{j,t+1} = A_j + α(v_j − A_j)`, run
/// in the `record` phase (§10.1 phase 9). Also persists `v_1`'s `r^L_{t-1}`
/// baseline and the realised `v_1` that [`Satisficing`] reads next `decide`.
pub struct AspirationUpdate {
    id: PluginId,
    alpha: f64,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}

impl AspirationUpdate {
    /// Build from parameters.
    ///
    /// # Errors
    /// `α` not in `(0, 1)` (§12.1).
    pub fn new(params: AspirationUpdateParams) -> Result<AspirationUpdate, String> {
        if !(params.alpha.is_finite() && params.alpha > 0.0 && params.alpha < 1.0) {
            return Err(format!("alpha must be in (0, 1), got {}", params.alpha));
        }
        Ok(AspirationUpdate {
            id: PluginId::new(catalog::ASPIRATION_UPDATE_ID),
            alpha: params.alpha,
            reads: vec![ComponentId::ledger()],
            writes: vec![DeltaKindTag::AdjustAgentReal, DeltaKindTag::SetAgentInt],
        })
    }
}

impl Rule for AspirationUpdate {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> semver::Version {
        version()
    }
    fn phase(&self) -> Phase {
        Phase::Record
    }
    fn reads(&self) -> &[ComponentId] {
        &self.reads
    }
    fn writes(&self) -> &[DeltaKindTag] {
        &self.writes
    }
    fn apply(&self, view: &dyn View, _key: RngKey) -> Vec<Delta> {
        let mut out = Vec::new();
        for &agent in view.live_agents() {
            let r_l = view.agent_stock(agent, &res(keys::CAPITAL));
            let c = view.agent_real(agent, keys::CAPABILITY).unwrap_or(0.0);
            let q = view.agent_int(agent, keys::OBLIGATION).unwrap_or(0);
            let prev = view
                .agent_int(agent, keys::PREV_TICK_CAPITAL)
                .unwrap_or(r_l);
            let v1 = r_l - prev;
            let v = [v1 as f64, c, -(q as f64)];
            let a_keys = [
                keys::ASPIRATION_CAPITAL_GROWTH,
                keys::ASPIRATION_CAPABILITY,
                keys::ASPIRATION_OBLIGATION_CLEARANCE,
            ];

            for j in 0..3 {
                let delta = match view.agent_real(agent, a_keys[j]) {
                    // absent ⇒ seed: A := v_j  (store is 0, add v_j)
                    None => v[j],
                    // present ⇒ A += α(v_j − A)
                    Some(a) => self.alpha * (v[j] - a),
                };
                if delta != 0.0 {
                    out.push(Delta {
                        target: DeltaTarget::Agent(agent),
                        kind: DeltaKind::AdjustAgentReal {
                            field: a_keys[j].to_owned(),
                            delta,
                        },
                        conflict_class: ConflictClass::Independent,
                        origin: self.id.clone(),
                    });
                }
            }

            let set = |field: &str, value: i64| Delta {
                target: DeltaTarget::Agent(agent),
                kind: DeltaKind::SetAgentInt {
                    field: field.to_owned(),
                    value,
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            };
            out.push(set(keys::REALIZED_CAPITAL_GROWTH, v1));
            out.push(set(keys::PREV_TICK_CAPITAL, r_l));
        }
        out
    }
    fn assumption(&self) -> &str {
        "Each goal's aspiration level drifts toward the value the firm actually \
         realised, at rate α — so sustained underperformance eventually makes a \
         low outcome 'satisfactory' (§12.1 adaptive aspirations, Cyert & March)."
    }
}

// --------------------------------------------------------------------------
// registration
// --------------------------------------------------------------------------

/// A constructor signature shared by every rule this crate registers.
pub type Ctor = fn(&serde_json::Value) -> Result<Box<dyn Rule>, String>;

fn build_satisficing(p: &serde_json::Value) -> Result<Box<dyn Rule>, String> {
    let params: SatisficingParams = if p.is_null() {
        SatisficingParams::default()
    } else {
        serde_json::from_value(p.clone()).map_err(|e| e.to_string())?
    };
    Ok(Box::new(Satisficing::new(params)?))
}

fn build_random(p: &serde_json::Value) -> Result<Box<dyn Rule>, String> {
    let params: RandomParams = if p.is_null() {
        RandomParams::default()
    } else {
        serde_json::from_value(p.clone()).map_err(|e| e.to_string())?
    };
    Ok(Box::new(DecisionRandom::new(params)?))
}

fn build_aspiration_update(p: &serde_json::Value) -> Result<Box<dyn Rule>, String> {
    let params: AspirationUpdateParams = if p.is_null() {
        AspirationUpdateParams::default()
    } else {
        serde_json::from_value(p.clone()).map_err(|e| e.to_string())?
    };
    Ok(Box::new(AspirationUpdate::new(params)?))
}

/// Registration entries for `firma-cli` — `(id, content_hash, ctor)`.
#[must_use]
pub fn registered() -> Vec<(&'static str, &'static str, Ctor)> {
    vec![
        (
            catalog::SATISFICING_ID,
            catalog::CONTENT_HASH,
            build_satisficing,
        ),
        (catalog::RANDOM_ID, catalog::CONTENT_HASH, build_random),
        (
            catalog::ASPIRATION_UPDATE_ID,
            catalog::CONTENT_HASH,
            build_aspiration_update,
        ),
    ]
}

pub mod support;

#[cfg(test)]
mod tests;
