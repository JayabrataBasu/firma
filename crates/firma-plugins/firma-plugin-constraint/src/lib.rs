//! `firma-plugin-constraint` — the four MVP constraint plugins (manual §9.1),
//! one per row of the §9.1 table, using the `Constraint` interface of ADR 0021.
//!
//! | plugin | `g_j` | violation semantic (§9.1) |
//! |---|---|---|
//! | [`Solvency`] | `g_1 = −r^L` | `Death` |
//! | [`Compliance`] | `g_2 = u − θ_limit` | `Graduated` (penalty + `λ` loss, then death) |
//! | [`Scope`] | `g_3 = θ_cap − c` | `AdmissibilityGate` (never lethal) |
//! | [`Obligation`] | `g_4 = q − θ_Q` | `Relational` (sever `supply` edges + penalty) |
//!
//! Scale factors default to §9.2 (`s_L=100`, `s_u=1.0`, `s_c=0.5`, `s_q=50`)
//! and are configurable. The `Relational` penalty `P_q` has **no §16.1 value**
//! (a manual gap, ADR 0021) — [`Obligation`] requires it as a parameter.

#![forbid(unsafe_code)]

mod phase_rules;

use serde::{Deserialize, Serialize};

use firma_domain::{Constraint, ConstraintContext, MarginTerm, PluginId, ViolationSemantic};

pub use phase_rules::{
    build_action_window, build_enforce, registered_rules, ActionWindow, ActionWindowParams, Ctor,
    Enforce, EnforceParams,
};

/// Shared build version of the constraint plugins.
#[must_use]
pub fn version() -> semver::Version {
    semver::Version::new(1, 0, 0)
}

/// Declared content hashes (Phase-1 style — no artefact hashing yet).
pub mod catalog {
    /// `constraint.solvency`.
    pub const SOLVENCY_ID: &str = "constraint.solvency";
    /// `constraint.compliance`.
    pub const COMPLIANCE_ID: &str = "constraint.compliance";
    /// `constraint.scope`.
    pub const SCOPE_ID: &str = "constraint.scope";
    /// `constraint.obligation`.
    pub const OBLIGATION_ID: &str = "constraint.obligation";
    /// `constraint.action_window` — the phase-7 `constrain` rule (ADR 0028).
    pub const ACTION_WINDOW_ID: &str = "constraint.action_window";
    /// `constraint.enforce` — the phase-8 `enforce` rule (ADR 0029).
    pub const ENFORCE_ID: &str = "constraint.enforce";

    /// Declared content hash for the phase-7/8 rules.
    pub const PHASE_RULES_HASH: &str = "phase2s4-constraint-phase-rules-v1";

    /// Declared content hash for [`Solvency`](super::Solvency).
    pub const SOLVENCY_HASH: &str = "phase2s1-constraint-solvency-v1";
    /// Declared content hash for [`Compliance`](super::Compliance).
    pub const COMPLIANCE_HASH: &str = "phase2s1-constraint-compliance-v1";
    /// Declared content hash for [`Scope`](super::Scope).
    pub const SCOPE_HASH: &str = "phase2s1-constraint-scope-v1";
    /// Declared content hash for [`Obligation`](super::Obligation).
    pub const OBLIGATION_HASH: &str = "phase2s1-constraint-obligation-v1";
}

fn check_scale(s: f64) -> Result<f64, String> {
    if s > 0.0 && s.is_finite() {
        Ok(s)
    } else {
        Err(format!("scale must be finite and > 0, got {s}"))
    }
}

// --------------------------------------------------------------------------
// solvency — §9.1, g_1 = -r^L, Death
// --------------------------------------------------------------------------

/// Parameters for [`Solvency`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolvencyParams {
    /// `s_L`. §9.2 default `100`.
    #[serde(default = "default_s_l")]
    pub scale: f64,
}
fn default_s_l() -> f64 {
    100.0
}
impl Default for SolvencyParams {
    fn default() -> Self {
        SolvencyParams {
            scale: default_s_l(),
        }
    }
}

/// §9.1 `solvency`: `g_1 = −r^L`. Zero liquid capital ⇒ insolvency; violation ⇒
/// the agent is removed at the end of `enforce` (§10.1 phase 8).
pub struct Solvency {
    scale: f64,
}
impl Solvency {
    /// Build from parameters.
    ///
    /// # Errors
    /// If `scale` is not finite and `> 0`.
    pub fn new(p: SolvencyParams) -> Result<Solvency, String> {
        Ok(Solvency {
            scale: check_scale(p.scale)?,
        })
    }
}
impl MarginTerm for Solvency {
    fn g(&self, c: &ConstraintContext<'_>) -> f64 {
        firma_domain::margin::g_solvency(c) // §9.1 / ADR 0026 — single source
    }
    fn scale(&self) -> f64 {
        self.scale
    }
}
impl Constraint for Solvency {
    fn id(&self) -> PluginId {
        PluginId::new(catalog::SOLVENCY_ID)
    }
    fn version(&self) -> semver::Version {
        version()
    }
    fn violation(&self) -> ViolationSemantic {
        ViolationSemantic::Death
    }
    fn assumption(&self) -> &str {
        "Liquid capital below zero is fatal: g_1 = -r^L, and a firm that runs \
         out of spendable capital is removed from the population (§9.1 solvency)."
    }
}

// --------------------------------------------------------------------------
// compliance — §9.1, g_2 = u - theta_limit, Graduated
// --------------------------------------------------------------------------

/// Parameters for [`Compliance`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComplianceParams {
    /// `s_u`. §9.2 default `1.0`.
    #[serde(default = "default_s_u")]
    pub scale: f64,
    /// `P_c` — first-violation penalty. §16.1 default `30`.
    #[serde(default = "default_p_c")]
    pub penalty: i64,
    /// `T_c` — ticks within which a second violation is fatal. §16.1 default `4`.
    #[serde(default = "default_t_c")]
    pub window_ticks: u64,
    /// `δ_λ` — legitimacy lost on a first violation. §16.1 default `0.15`.
    #[serde(default = "default_delta_lambda")]
    pub legitimacy_loss: f64,
}
fn default_s_u() -> f64 {
    1.0
}
fn default_p_c() -> i64 {
    30
}
fn default_t_c() -> u64 {
    4
}
fn default_delta_lambda() -> f64 {
    0.15
}
impl Default for ComplianceParams {
    fn default() -> Self {
        ComplianceParams {
            scale: default_s_u(),
            penalty: default_p_c(),
            window_ticks: default_t_c(),
            legitimacy_loss: default_delta_lambda(),
        }
    }
}

/// §9.1 `compliance`: `g_2 = u − θ_limit` (with `u` per ADR 0014 — the simple
/// trailing-window mean of regulated production). Graduated: a first violation
/// costs `P_c` and `λ −= δ_λ`; a second within `T_c` ticks is fatal.
pub struct Compliance {
    scale: f64,
    penalty: i64,
    window_ticks: u64,
    legitimacy_loss: f64,
}
impl Compliance {
    /// Build from parameters.
    ///
    /// # Errors
    /// If `scale` is invalid, `penalty < 0`, or `legitimacy_loss` is not in
    /// `[0, 1]`.
    pub fn new(p: ComplianceParams) -> Result<Compliance, String> {
        if p.penalty < 0 {
            return Err(format!("penalty must be >= 0, got {}", p.penalty));
        }
        if !(0.0..=1.0).contains(&p.legitimacy_loss) {
            return Err(format!(
                "legitimacy_loss must be in [0, 1], got {}",
                p.legitimacy_loss
            ));
        }
        Ok(Compliance {
            scale: check_scale(p.scale)?,
            penalty: p.penalty,
            window_ticks: p.window_ticks,
            legitimacy_loss: p.legitimacy_loss,
        })
    }
}
impl MarginTerm for Compliance {
    fn g(&self, c: &ConstraintContext<'_>) -> f64 {
        firma_domain::margin::g_compliance(c) // §9.1 / ADR 0026 — single source
    }
    fn scale(&self) -> f64 {
        self.scale
    }
}
impl Constraint for Compliance {
    fn id(&self) -> PluginId {
        PluginId::new(catalog::COMPLIANCE_ID)
    }
    fn version(&self) -> semver::Version {
        version()
    }
    fn violation(&self) -> ViolationSemantic {
        ViolationSemantic::Graduated {
            penalty: self.penalty,
            window_ticks: self.window_ticks,
            legitimacy_loss: self.legitimacy_loss,
        }
    }
    fn assumption(&self) -> &str {
        "Regulated-activity intensity above the permitted limit is penalised \
         then, on repetition, fatal: g_2 = u - θ_limit, where u is the \
         trailing-window mean of regulated production (§9.1 compliance, ADR 0014)."
    }
}

// --------------------------------------------------------------------------
// scope — §9.1, g_3 = theta_cap - c, AdmissibilityGate
// --------------------------------------------------------------------------

/// Parameters for [`Scope`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeParams {
    /// `s_c`. §9.2 default `0.5`.
    #[serde(default = "default_s_c")]
    pub scale: f64,
}
fn default_s_c() -> f64 {
    0.5
}
impl Default for ScopeParams {
    fn default() -> Self {
        ScopeParams {
            scale: default_s_c(),
        }
    }
}

/// §9.1 `scope`: `g_3 = θ_cap − c`. Never lethal — while violated, regulated
/// production is inadmissible (§11.1 precondition `c ≥ θ_cap`, §11.4). The
/// `Admissibility` service calls this constraint's `g` directly (ADR 0021
/// decision 2); `enforce` does nothing with it.
pub struct Scope {
    scale: f64,
}
impl Scope {
    /// Build from parameters.
    ///
    /// # Errors
    /// If `scale` is not finite and `> 0`.
    pub fn new(p: ScopeParams) -> Result<Scope, String> {
        Ok(Scope {
            scale: check_scale(p.scale)?,
        })
    }
}
impl MarginTerm for Scope {
    fn g(&self, c: &ConstraintContext<'_>) -> f64 {
        firma_domain::margin::g_scope(c) // §9.1 / ADR 0026 — single source
    }
    fn scale(&self) -> f64 {
        self.scale
    }
}
impl Constraint for Scope {
    fn id(&self) -> PluginId {
        PluginId::new(catalog::SCOPE_ID)
    }
    fn version(&self) -> semver::Version {
        version()
    }
    fn violation(&self) -> ViolationSemantic {
        ViolationSemantic::AdmissibilityGate
    }
    fn assumption(&self) -> &str {
        "Capability below the regulated-production threshold gates that action \
         but is never fatal: g_3 = θ_cap - c (§9.1 scope, §11.4)."
    }
}

// --------------------------------------------------------------------------
// obligation — §9.1, g_4 = q - theta_Q, Relational
// --------------------------------------------------------------------------

/// Parameters for [`Obligation`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObligationParams {
    /// `s_q`. §9.2 default `50`.
    #[serde(default = "default_s_q")]
    pub scale: f64,
    /// `P_q` — penalty applied on violation. **Required** — §16.1 gives no
    /// value for `P_q` (a manual gap; ADR 0021).
    pub penalty: i64,
}
fn default_s_q() -> f64 {
    50.0
}

/// §9.1 `obligation`: `g_4 = q − θ_Q`. Never lethal — on violation every
/// `supply` edge of the firm is severed and `P_q` is applied. (The
/// edge-severance handler lands in Stage 2 with the relation graph; Stage 1
/// defines the semantic tag only — ADR 0021.)
pub struct Obligation {
    scale: f64,
    penalty: i64,
}
impl Obligation {
    /// Build from parameters.
    ///
    /// # Errors
    /// If `scale` is invalid or `penalty < 0`.
    pub fn new(p: ObligationParams) -> Result<Obligation, String> {
        if p.penalty < 0 {
            return Err(format!("penalty must be >= 0, got {}", p.penalty));
        }
        Ok(Obligation {
            scale: check_scale(p.scale)?,
            penalty: p.penalty,
        })
    }
}
impl MarginTerm for Obligation {
    fn g(&self, c: &ConstraintContext<'_>) -> f64 {
        firma_domain::margin::g_obligation(c) // §9.1 / ADR 0026 — single source
    }
    fn scale(&self) -> f64 {
        self.scale
    }
}
impl Constraint for Obligation {
    fn id(&self) -> PluginId {
        PluginId::new(catalog::OBLIGATION_ID)
    }
    fn version(&self) -> semver::Version {
        version()
    }
    fn violation(&self) -> ViolationSemantic {
        ViolationSemantic::Relational {
            penalty: self.penalty,
        }
    }
    fn assumption(&self) -> &str {
        "Outstanding obligation above the permitted maximum severs the firm's \
         supply relationships and applies a penalty, but is never fatal: \
         g_4 = q - θ_Q (§9.1 obligation)."
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use firma_domain::{Aspirations, ConstraintParams, FirmAuxState, FirmState};

    fn ctx_15_1<'a>(
        s: &'a FirmState,
        a: &'a FirmAuxState,
        t: &'a ConstraintParams,
    ) -> ConstraintContext<'a> {
        ConstraintContext {
            state: s,
            aux: a,
            theta: t,
        }
    }

    #[test]
    fn each_g_matches_section_15_1() {
        let s = FirmState {
            liquid_capital: 40,
            input_stock: 12,
            capability: 0.55,
            obligation: 30,
        };
        let a = FirmAuxState {
            legitimacy: 1.0,
            regulated_intensity: 0.72,
            aspirations: Aspirations {
                capital_growth: 0.0,
                capability: 0.0,
                obligation_clearance: 0.0,
            },
        };
        let t = ConstraintParams {
            theta_limit: 0.90,
            theta_cap: 0.40,
            theta_q: 100,
        };
        let c = ctx_15_1(&s, &a, &t);

        let sol = Solvency::new(SolvencyParams::default()).unwrap();
        let com = Compliance::new(ComplianceParams::default()).unwrap();
        let sco = Scope::new(ScopeParams::default()).unwrap();
        let obl = Obligation::new(ObligationParams {
            scale: default_s_q(),
            penalty: 30,
        })
        .unwrap();

        assert!((sol.g(&c) - (-40.0)).abs() < 1e-12);
        assert!((com.g(&c) - (-0.18)).abs() < 1e-12);
        assert!((sco.g(&c) - (-0.15)).abs() < 1e-12);
        assert!((obl.g(&c) - (-70.0)).abs() < 1e-12);

        assert!((sol.g(&c) / sol.scale() - (-0.400)).abs() < 1e-12);
        assert!((com.g(&c) / com.scale() - (-0.180)).abs() < 1e-12);
        assert!((sco.g(&c) / sco.scale() - (-0.300)).abs() < 1e-12);
        assert!((obl.g(&c) / obl.scale() - (-1.400)).abs() < 1e-12);
    }

    #[test]
    fn violation_semantics_match_section_9_1() {
        assert_eq!(
            Solvency::new(SolvencyParams::default())
                .unwrap()
                .violation(),
            ViolationSemantic::Death
        );
        assert!(matches!(
            Compliance::new(ComplianceParams::default())
                .unwrap()
                .violation(),
            ViolationSemantic::Graduated {
                penalty: 30,
                window_ticks: 4,
                ..
            }
        ));
        assert_eq!(
            Scope::new(ScopeParams::default()).unwrap().violation(),
            ViolationSemantic::AdmissibilityGate
        );
        assert!(matches!(
            Obligation::new(ObligationParams {
                scale: 50.0,
                penalty: 12
            })
            .unwrap()
            .violation(),
            ViolationSemantic::Relational { penalty: 12 }
        ));
    }

    #[test]
    fn assumptions_non_empty() {
        for a in [
            Solvency::new(SolvencyParams::default())
                .unwrap()
                .assumption(),
            Compliance::new(ComplianceParams::default())
                .unwrap()
                .assumption(),
            Scope::new(ScopeParams::default()).unwrap().assumption(),
            Obligation::new(ObligationParams {
                scale: 50.0,
                penalty: 1,
            })
            .unwrap()
            .assumption(),
        ] {
            assert!(!a.trim().is_empty());
        }
    }

    #[test]
    fn obligation_penalty_is_required() {
        // §16.1 gives no P_q — the param has no serde default.
        assert!(serde_json::from_str::<ObligationParams>(r#"{"scale":50.0}"#).is_err());
        assert!(serde_json::from_str::<ObligationParams>(r#"{"scale":50.0,"penalty":25}"#).is_ok());
    }

    #[test]
    fn bad_params_rejected() {
        assert!(Solvency::new(SolvencyParams { scale: 0.0 }).is_err());
        assert!(Solvency::new(SolvencyParams { scale: -1.0 }).is_err());
        assert!(Compliance::new(ComplianceParams {
            legitimacy_loss: 1.5,
            ..Default::default()
        })
        .is_err());
        assert!(Obligation::new(ObligationParams {
            scale: 50.0,
            penalty: -1
        })
        .is_err());
    }
}
