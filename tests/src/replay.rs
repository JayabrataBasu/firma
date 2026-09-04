//! Re-export shim (ADR 0046). The SC-1…SC-6 reconstruction that used to live
//! here was promoted to `firma-analysis::sanity` so `firma-tui` and
//! `firma-py` can reuse it instead of re-deriving `standard_margin`/`g_j`/
//! entropy a second and third time. Every existing call site
//! (`firma_conformance::replay::{sanity_from_run, SanityReport}`) is
//! unaffected.

pub use firma_analysis::sanity::*;
