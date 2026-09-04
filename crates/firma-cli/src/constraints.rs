//! Stage-1 constraint wiring (manual §9.1; ADR 0021 decision 5).
//!
//! Mirrors [`standard_registry`](crate::standard_registry): a start-up helper
//! that builds the four MVP constraint plugins. Formal `firma-registry`
//! integration (a `RegisteredConstraint` table, config-driven
//! `constraints: [...]` resolution) lands in Stage 2 when the
//! `decide` / `act` / `enforce` phases run end-to-end.

use firma_domain::Constraint;
use firma_plugin_constraint::{
    Compliance, ComplianceParams, Obligation, ObligationParams, Scope, ScopeParams, Solvency,
    SolvencyParams,
};

/// The four MVP constraint plugins in canonical §9.1 order
/// `[solvency, compliance, scope, obligation]`, each with its §9.2 default scale
/// and §16.1 default penalties.
///
/// `obligation_penalty` (`P_q`) must be supplied by the caller — §16.1 gives no
/// value for it (a manual gap, ADR 0021), so there is no default to fall back
/// on.
///
/// # Panics
/// Only if a default parameter set is somehow invalid (it never is).
#[must_use]
pub fn standard_constraints(obligation_penalty: i64) -> [Box<dyn Constraint>; 4] {
    [
        Box::new(Solvency::new(SolvencyParams::default()).expect("default solvency params valid")),
        Box::new(
            Compliance::new(ComplianceParams::default()).expect("default compliance params valid"),
        ),
        Box::new(Scope::new(ScopeParams::default()).expect("default scope params valid")),
        Box::new(
            Obligation::new(ObligationParams {
                scale: 50.0,
                penalty: obligation_penalty,
            })
            .expect("obligation params valid"),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::standard_constraints;

    #[test]
    fn builds_four_in_canonical_order() {
        let cs = standard_constraints(30);
        let ids: Vec<_> = cs.iter().map(|c| c.id().0.clone()).collect();
        assert_eq!(
            ids,
            [
                "constraint.solvency",
                "constraint.compliance",
                "constraint.scope",
                "constraint.obligation"
            ]
        );
        for c in &cs {
            assert!(!c.assumption().trim().is_empty());
            assert!(c.scale() > 0.0);
        }
    }
}
