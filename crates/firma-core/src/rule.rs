//! The [`Rule`] and [`ConflictResolver`] plugin traits (manual §20.2, §19.4).
//!
//! Manual §20.2: "The trait is load-bearing for §20.7. Every method added now
//! must be marshalled across a WASM boundary later. **Resist growth.**" Phase 1
//! kept `Rule` at exactly the seven methods the manual specifies; Phase 2
//! Stage 5 adds one more, [`Rule::rng_stream`] (ADR 0034), because the §21.3
//! stream a rule draws from is not always derivable from its phase (two
//! different §21.3 stream needs — `shock` and `environment` — both live in
//! phase 1). It is a defaulted method (one enum byte over WASM); every Phase-1
//! rule is byte-identical without touching it.

use semver::Version;

use crate::delta::{ConflictClass, Delta, DeltaKindTag};
use crate::ids::{ComponentId, PluginId};
use crate::rng_key::{Phase, RngKey, StreamId};
use crate::view::View;

/// A unit of theoretical assumption (manual §7.2 primitive 8, §20.2).
///
/// Purity contract: `apply` performs **no I/O, no globals, no wall clock, no
/// ambient randomness**. All randomness comes from `key` via `firma-rng`. A
/// rule that violates this breaks reproducibility regardless of language
/// (§21.1).
pub trait Rule: Send + Sync {
    /// Stable id, `"<category>.<name>"`, unchanged across versions.
    fn id(&self) -> PluginId;

    /// This build's semantic version. A MAJOR change means numerical output
    /// changed (§20.5).
    fn version(&self) -> Version;

    /// The phase this rule runs in.
    fn phase(&self) -> Phase;

    /// Components this rule reads. Advisory in Phase 1; a scheduling/validation
    /// aid later.
    fn reads(&self) -> &[ComponentId];

    /// Delta kinds this rule may emit. Emitting an undeclared kind panics in the
    /// kernel (§20.2).
    fn writes(&self) -> &[DeltaKindTag];

    /// Which of the four §21.3 RNG streams this rule's draws come from, or
    /// `None` to take the kernel's phase-derived default (ADR 0034). Overridden
    /// only by rules whose §21.3 stream differs from their phase's default:
    /// `shock.stochastic` (phase `environment`, but draws are shock
    /// timing/magnitude ⇒ `Shock`) and `observation.noisy` / `resource.patchy`
    /// (phases `observe` / `environment`, drawing exogenous environment
    /// variation ⇒ `Environment`). A rule that never draws may leave this
    /// `None`. Marshals across a WASM boundary as one enum byte.
    fn rng_stream(&self) -> Option<StreamId> {
        None
    }

    /// Pure function from a read-only view + an RNG key to proposed deltas
    /// (§6.4). Never mutates; never touches the clock, the filesystem, the
    /// network, or a global generator.
    fn apply(&self, view: &dyn View, key: RngKey) -> Vec<Delta>;

    /// One plain-language sentence stating what about the world this rule
    /// claims. Written verbatim into the run manifest (§20.2, §22.3). **MUST NOT
    /// be empty** — enforced by the `assumption-nonempty` lint (§25.6) and a
    /// conformance test.
    fn assumption(&self) -> &str;
}

/// Settles contention within one [`ConflictClass`] group (manual §19.4 step 4,
/// §20.3). How scarce-resource contention is resolved is a theoretical
/// assumption, so this is a plugin and appears in the manifest.
pub trait ConflictResolver: Send + Sync {
    /// Stable id, `"conflict.<name>"`.
    fn id(&self) -> PluginId;

    /// This build's semantic version.
    fn version(&self) -> Version;

    /// The class this resolver handles.
    fn handles(&self) -> ConflictClass;

    /// Given every delta proposed in one phase for one `(class)` group, already
    /// sorted by the §19.4 order, return the deltas to actually apply. The
    /// result MUST conserve exactly what the inputs would have moved in
    /// aggregate under the resolver's policy (§21.4): a resolver may reallocate
    /// or scale, never create or destroy.
    fn resolve(&self, group: &[Delta]) -> Vec<Delta>;

    /// Plain-language statement of the allocation assumption (e.g. "contested
    /// claims are rationed proportionally, remainder to low ids"). MUST NOT be
    /// empty (§25.6).
    fn assumption(&self) -> &str;
}
