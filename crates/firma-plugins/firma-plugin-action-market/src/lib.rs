//! `action.market.standard` — the six §11.1 market-action `Rule` plugins
//! (manual §11.1, §10.1 phase 4; ADR 0022 hand-off, ADR 0023 lagged effects).
//!
//! Each rule **executes** the action the decision procedure already selected
//! (Stage 3 writes `selected_action`; this crate reads it). A rule acts for an
//! agent iff `view.agent_int(agent, keys::SELECTED_ACTION) == Some(my_index)`.
//! The §11.1 arithmetic and preconditions are **not** reimplemented here — every
//! rule calls [`firma_domain::dynamics::market_core`] and emits deltas from the
//! difference between the pre- and post-state.
//!
//! | idx | rule | id | emits |
//! |---|---|---|---|
//! | 0 | [`Hold`] | `action.market.standard.hold` | nothing |
//! | 1 | [`ProduceOrdinary`] | `…produce_ordinary` | `r^I−`, `r^L+` (paired with the env pool) |
//! | 2 | [`ProduceRegulated`] | `…produce_regulated` | `r^I−`, `r^L+` (paired) |
//! | 3 | [`AcquireInput`] | `…acquire_input` | `r^L−`, `r^I+` (paired) |
//! | 4 | [`InvestCapability`] | `…invest_capability` | `r^L−` (paired) + a lagged `CapabilityGain` |
//! | 5 | [`Deliver`] | `…deliver` | `q−`, `r^I−` (paired) |
//!
//! ## Conservation (§21.4)
//!
//! Production *creates* `r^L` and consumption *destroys* `r^I`. To keep the run
//! conserving, every stock change an agent makes is **paired with the opposite
//! change on the shared environment pool** (`DeltaTarget::Environment`) — the
//! pool *is* "the market". A run that uses these rules MUST seed the env pool
//! with enough `capital` / `input` that it never goes negative (the kernel
//! aborts on a negative pool, §19.5); the env-pool dynamics that would refill it
//! are a Stage-4 Resource-plugin concern. `q` (obligation) is a per-firm
//! counter, not a conserved resource (§8.1), so `deliver`'s `q−` is unpaired.
//!
//! ## Defensive behaviour
//!
//! If `market_core` returns `None` — the selected action's §11.1 precondition
//! is not met at the agent's phase-start state — the rule emits **nothing**. A
//! well-formed decision procedure (§12.3 step, admissibility §11.4) never
//! selects an inadmissible action, so this is a guard against a decide-bug or a
//! mis-seeded state, not a normal path. It is a silent no-op rather than an
//! error because one firm's bad selection must not abort a whole run's tick;
//! the empty action is still visible in the event log by the *absence* of this
//! firm's deltas that tick.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use firma_core::{
    ComponentId, ConflictClass, Delta, DeltaKind, DeltaKindTag, DeltaTarget, Phase, PluginId,
    ResourceKind, RngKey, Rule, View,
};
use firma_domain::dynamics::{market_core, ActionParams, EnvParams, MarketAction};
use firma_domain::{keys, ConstraintParams, Effect, FirmState, LaggedRecord};

/// This crate's build version (ADR 0022: new crate, `1.0.0`).
#[must_use]
pub fn version() -> semver::Version {
    semver::Version::new(1, 0, 0)
}

/// Plugin ids and declared content hashes (Phase-1 style — no artefact hashing).
pub mod catalog {
    /// `action.market.standard.hold`.
    pub const HOLD_ID: &str = "action.market.standard.hold";
    /// `action.market.standard.produce_ordinary`.
    pub const PRODUCE_ORDINARY_ID: &str = "action.market.standard.produce_ordinary";
    /// `action.market.standard.produce_regulated`.
    pub const PRODUCE_REGULATED_ID: &str = "action.market.standard.produce_regulated";
    /// `action.market.standard.acquire_input`.
    pub const ACQUIRE_INPUT_ID: &str = "action.market.standard.acquire_input";
    /// `action.market.standard.invest_capability`.
    pub const INVEST_CAPABILITY_ID: &str = "action.market.standard.invest_capability";
    /// `action.market.standard.deliver`.
    pub const DELIVER_ID: &str = "action.market.standard.deliver";

    /// Declared content hash for the market-action build.
    pub const CONTENT_HASH: &str = "phase2s2-action-market-standard-v1";
}

/// `Δ_cap` — the fixed lag on `invest_capability`'s capability gain (§16.1,
/// "fixed"; **not** drawn, unlike shaping lags — ADR 0023 Decision 4).
pub const CAPABILITY_LAG: u64 = 3;

/// Parameters shared by the market-action rules: the §11.1 action parameters
/// (`y_0`, `η`, `γ_R`, `R^I_max`, `δ_c`, `κ_c`) and the fixed capability lag.
/// Defaults are the §16.1 "fixed" values.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct MarketParams {
    /// §11.1 action parameters. See [`ActionParams`].
    pub action: ActionParams,
    /// `Δ_cap` — the fixed `invest_capability` lag. §16.1 default `3`.
    pub capability_lag: u64,
}

impl Default for MarketParams {
    fn default() -> Self {
        MarketParams {
            action: ActionParams::default(),
            capability_lag: CAPABILITY_LAG,
        }
    }
}

fn capital() -> ResourceKind {
    ResourceKind(keys::CAPITAL.to_owned())
}
fn input() -> ResourceKind {
    ResourceKind(keys::INPUT.to_owned())
}

/// Read an agent's constraint-carrying state (§8.1) from the view. Missing
/// scalars default to the §8.1 domain floor (`c = 0`, `q = 0`); the conserved
/// stocks are always present (`0` if the agent holds none).
fn firm_state(view: &dyn View, agent: firma_core::AgentId) -> FirmState {
    FirmState {
        liquid_capital: view.agent_stock(agent, &capital()),
        input_stock: view.agent_stock(agent, &input()),
        capability: view.agent_real(agent, keys::CAPABILITY).unwrap_or(0.0),
        obligation: view.agent_int(agent, keys::OBLIGATION).unwrap_or(0),
    }
}

/// Read the global constraint parameters θ (§8.2). Only `θ_cap` is consulted by
/// [`market_core`] (the `produce_regulated` scope gate); an unseeded `θ_cap`
/// defaults to `0.0`, i.e. the gate is open — the orchestrator is expected to
/// seed θ for any run that uses `produce_regulated`.
fn theta(view: &dyn View) -> ConstraintParams {
    ConstraintParams {
        theta_limit: view.global_real(keys::THETA_LIMIT).unwrap_or(0.0),
        theta_cap: view.global_real(keys::THETA_CAP).unwrap_or(0.0),
        theta_q: view.global_int(keys::THETA_Q).unwrap_or(0),
    }
}

/// Read the environment prices `(π^I, π^O)` (§8.3). Unseeded ⇒ `0`.
fn env_params(view: &dyn View) -> EnvParams {
    EnvParams {
        input_price: view.global_int(keys::INPUT_PRICE).unwrap_or(0),
        output_price: view.global_int(keys::OUTPUT_PRICE).unwrap_or(0),
    }
}

/// A paired stock delta: `+amount` to `agent`, `−amount` to the env pool (or
/// vice-versa). Conserving by construction (§21.4). `ResourcePool` class — the
/// env pool is shared and a rationing resolver settles scarcity (§19.4).
fn paired_stock(
    agent: firma_core::AgentId,
    resource: &ResourceKind,
    amount: i64,
    origin: &PluginId,
) -> [Delta; 2] {
    [
        Delta {
            target: DeltaTarget::Agent(agent),
            kind: DeltaKind::AdjustStock {
                resource: resource.clone(),
                amount,
            },
            conflict_class: ConflictClass::ResourcePool,
            origin: origin.clone(),
        },
        Delta {
            target: DeltaTarget::Environment,
            kind: DeltaKind::AdjustStock {
                resource: resource.clone(),
                amount: -amount,
            },
            conflict_class: ConflictClass::ResourcePool,
            origin: origin.clone(),
        },
    ]
}

/// Deltas for `agent` executing `action`, given its pre-state `s` and the
/// `market_core` post-state `n`. `invest_capability`'s capability gain is
/// **not** taken from `n` (whose `c` change is `market_core`'s §9.3 collapse of
/// the lag); instead a lagged [`Effect::CapabilityGain`] is enqueued.
fn deltas_for(
    action: MarketAction,
    agent: firma_core::AgentId,
    s: &FirmState,
    n: &FirmState,
    origin: &PluginId,
    tick: u64,
    p: &MarketParams,
) -> Vec<Delta> {
    let mut out = Vec::new();
    let d_cap = n.liquid_capital - s.liquid_capital;
    let d_in = n.input_stock - s.input_stock;
    let d_q = n.obligation - s.obligation;

    if d_cap != 0 {
        out.extend(paired_stock(agent, &capital(), d_cap, origin));
    }
    if d_in != 0 {
        out.extend(paired_stock(agent, &input(), d_in, origin));
    }
    if d_q != 0 {
        out.push(Delta {
            target: DeltaTarget::Agent(agent),
            kind: DeltaKind::AdjustAgentInt {
                field: keys::OBLIGATION.to_owned(),
                delta: d_q,
            },
            conflict_class: ConflictClass::Independent,
            origin: origin.clone(),
        });
    }
    if action == MarketAction::InvestCapability {
        let rec = LaggedRecord::new(
            tick + p.capability_lag,
            Effect::CapabilityGain {
                delta: p.action.capability_step,
            },
        );
        out.push(Delta {
            target: DeltaTarget::Agent(agent),
            kind: DeltaKind::PushAgentRecord {
                list: keys::LAGGED_EFFECTS.to_owned(),
                record_json: rec.to_json(),
            },
            conflict_class: ConflictClass::Independent,
            origin: origin.clone(),
        });
    }
    out
}

/// One market-action rule. All six share this body; they differ only in the
/// [`MarketAction`] they execute, their [`PluginId`], and their `assumption()`.
pub struct MarketRule {
    action: MarketAction,
    id: PluginId,
    params: MarketParams,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
    assumption: &'static str,
}

impl MarketRule {
    fn new(action: MarketAction, id: &str, params: MarketParams, assumption: &'static str) -> Self {
        MarketRule {
            action,
            id: PluginId::new(id),
            params,
            reads: vec![ComponentId::ledger()],
            writes: vec![
                DeltaKindTag::AdjustStock,
                DeltaKindTag::AdjustAgentInt,
                DeltaKindTag::PushAgentRecord,
            ],
            assumption,
        }
    }
}

impl Rule for MarketRule {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> semver::Version {
        version()
    }
    fn phase(&self) -> Phase {
        Phase::ActMarket
    }
    fn reads(&self) -> &[ComponentId] {
        &self.reads
    }
    fn writes(&self) -> &[DeltaKindTag] {
        &self.writes
    }
    fn apply(&self, view: &dyn View, _key: RngKey) -> Vec<Delta> {
        let idx = i64::from(self.action.index());
        let theta = theta(view);
        let env = env_params(view);
        let tick = view.tick().0;
        let mut out = Vec::new();
        for &agent in view.live_agents() {
            if view.agent_int(agent, keys::SELECTED_ACTION) != Some(idx) {
                continue;
            }
            let s = firm_state(view, agent);
            let Some(n) = market_core(self.action, &s, &theta, &env, &self.params.action) else {
                // Defensive: see the crate docs. Silent no-op.
                continue;
            };
            out.extend(deltas_for(
                self.action,
                agent,
                &s,
                &n,
                &self.id,
                tick,
                &self.params,
            ));
        }
        out
    }
    fn assumption(&self) -> &str {
        self.assumption
    }
}

macro_rules! market_ctor {
    ($name:ident, $action:expr, $id:path, $assumption:expr) => {
        /// Construct this market-action rule from JSON parameters
        /// ([`MarketParams`]; all fields optional, §16.1 defaults).
        ///
        /// # Errors
        /// If `params` is not a valid [`MarketParams`] object.
        pub fn $name(params: &serde_json::Value) -> Result<Box<dyn Rule>, String> {
            let p: MarketParams = if params.is_null() {
                MarketParams::default()
            } else {
                serde_json::from_value(params.clone()).map_err(|e| e.to_string())?
            };
            Ok(Box::new(MarketRule::new($action, $id, p, $assumption)))
        }
    };
}

market_ctor!(
    hold,
    MarketAction::Hold,
    catalog::HOLD_ID,
    "The firm can always choose to do nothing in the market this tick; hold has \
     no precondition, no effect, and no cost (§11.1 action 0)."
);
market_ctor!(
    produce_ordinary,
    MarketAction::ProduceOrdinary,
    catalog::PRODUCE_ORDINARY_ID,
    "Consuming one unit of input, the firm produces unregulated output whose \
     revenue is π^O·⌊y_0(1+ηc)⌋, added to liquid capital (§11.1 action 1)."
);
market_ctor!(
    produce_regulated,
    MarketAction::ProduceRegulated,
    catalog::PRODUCE_REGULATED_ID,
    "With capability at or above θ_cap, the firm may consume one input to \
     produce regulated output at the γ_R premium, at the cost of compliance \
     headroom (§11.1 action 2, §11.4 scope gate)."
);
market_ctor!(
    acquire_input,
    MarketAction::AcquireInput,
    catalog::ACQUIRE_INPUT_ID,
    "The firm buys one unit of input from the market at price π^I, provided it \
     can afford it and has storage below R^I_max (§11.1 action 3)."
);
market_ctor!(
    invest_capability,
    MarketAction::InvestCapability,
    catalog::INVEST_CAPABILITY_ID,
    "The firm spends κ_c now for a capability increase of δ_c that arrives only \
     after a fixed lag of Δ_cap ticks; capability is built, not bought \
     instantly (§11.1 action 4; ADR 0023)."
);
market_ctor!(
    deliver,
    MarketAction::Deliver,
    catalog::DELIVER_ID,
    "The firm discharges one unit of outstanding obligation by consuming one \
     unit of input to fulfil it (§11.1 action 5)."
);

/// Registration entries for `firma-cli`'s registry — one per market action.
/// Returned as `(id, content_hash, ctor)` tuples so the binary can wrap them in
/// its own `RegisteredRule` without this crate depending on `firma-registry`.
#[allow(clippy::type_complexity)]
#[must_use]
pub fn registered() -> Vec<(
    &'static str,
    &'static str,
    fn(&serde_json::Value) -> Result<Box<dyn Rule>, String>,
)> {
    vec![
        (catalog::HOLD_ID, catalog::CONTENT_HASH, hold),
        (
            catalog::PRODUCE_ORDINARY_ID,
            catalog::CONTENT_HASH,
            produce_ordinary,
        ),
        (
            catalog::PRODUCE_REGULATED_ID,
            catalog::CONTENT_HASH,
            produce_regulated,
        ),
        (
            catalog::ACQUIRE_INPUT_ID,
            catalog::CONTENT_HASH,
            acquire_input,
        ),
        (
            catalog::INVEST_CAPABILITY_ID,
            catalog::CONTENT_HASH,
            invest_capability,
        ),
        (catalog::DELIVER_ID, catalog::CONTENT_HASH, deliver),
    ]
}

#[cfg(test)]
mod tests;
