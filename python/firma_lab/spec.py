"""firma_lab.spec -- build, validate, and hash ``ExperimentSpec``s (manual
Sec 23.1, Sec 28.1), and derive per-replicate ``StreamSeeds`` for the
matched-environment / common-random-numbers design (manual Sec 21.3,
Sec 30.5) -- ADR-00NN ("seed/stream semantics").

Pure Python, deliberately: this is index arithmetic over an experiment
design (cartesian products, JSON-path patches, SHA-256 over a canonical
serialization) -- no FIRMA formula, no simulation logic, so it does not
cross ADR 0046's load/metrics "no formula reimplementation" line. The
canonical Rust type this module's ``StreamSeeds`` mirrors is
``firma_config::StreamSeeds`` (four ``u64`` fields); the canonical
``RunIdentity``/``run_id()`` hashing this module's ``ExperimentSpec.
content_hash()`` parallels is ``firma_io::manifest::RunIdentity`` -- read
directly (``crates/firma-config/src/lib.rs``, ``crates/firma-io/src/
manifest.rs``) before changing either half of this pairing.
"""

from __future__ import annotations

import hashlib
import itertools
import json
from dataclasses import dataclass
from typing import Any, Iterator, Mapping

# ---------------------------------------------------------------------------
# Part B -- seed/stream semantics
# ---------------------------------------------------------------------------
#
# manual Sec 21.3: "Four independent, separately seeded streams: `mechanism`
# (decisions, action outcomes, shaping success), `environment` (resource
# dynamics, exogenous variation), `shock` (timing and magnitude), `init`
# (initial conditions). A matched-environment design -- same `environment`
# and `shock`, different `mechanism` configuration -- removes a large
# variance component at zero cost."
#
# manual Sec 30.5: "Counter-based RNG, four streams (Sec 21.2-21.3). Seeds
# 1-200 per cell. Matched-environment design. All condition comparisons use
# common random numbers."
#
# DESIGN DECISION (this ADR, not previously settled in the manual): for
# replicate index ``n``, every one of the four ``StreamSeeds`` fields is set
# to the *same* value ``n`` -- not just ``environment``/``shock`` as
# Sec 21.3's one illustrative sentence names. Reasoning:
#
# 1. Sec 21.3's sentence names environment/shock as the fields a
#    matched-environment design holds fixed while "mechanism configuration"
#    (the swept factor's *parameters* -- which h/ς/β cell, which decision
#    plugin) differs -- it is not phrased as an exhaustive list of which
#    fields must differ, and nothing else in the manual forbids also
#    matching `mechanism`'s and `init`'s own stream *seeds* (as opposed to
#    the run's *configuration*, a different thing).
# 2. The RNG key formula (Sec 21.2: `H(run_seed . stream_id . plugin_id .
#    phase_id . tick . agent_id . purpose_tag)`) does not depend on any
#    economic parameter value (h, ς, β, novelty, ...) -- confirmed by
#    reading `firma-rng`'s key derivation directly, not assumed. So two
#    cells sharing the same `mechanism` seed produce bit-identical draw
#    sequences at every `(tick, agent, plugin, phase, purpose)` regardless
#    of which factor-level cell they belong to; matching `mechanism`'s seed
#    (not just environment/shock) only *strengthens* the common-random-
#    numbers property Sec 30.5 requires ("all condition comparisons use
#    common random numbers") at zero cost, exactly as Sec 21.3 says the
#    matched design does for environment/shock.
# 3. "Seeds 1-200 per cell" (Sec 30.5) is read literally: every cell, across
#    every arm, draws from the *same* numbered range 1-200 -- not a
#    disjoint per-cell or per-arm sub-range. `derive_seeds` is a single
#    function of the replicate index alone, with no arm- or cell-specific
#    branching, which is the simplest scheme consistent with that reading.
# 4. `init` is not named in Sec 21.3's one sentence, but nothing in the
#    manual treats `init` as a *swept factor* the way h/ς/β/novelty/shock-
#    magnitude/decision-plugin are -- it is "initial conditions" (the same
#    third-person, non-condition-varying role environment/shock play), so
#    the reasoning in point 2 applies to it identically. Matching it too
#    keeps the design uniform and avoids inventing an unstated fifth rule
#    for exactly one field.
#
# CONSEQUENCE, STATED EXPLICITLY (the instruction's own requirement --
# "enforce in code, not just in a comment"): every job at replicate `n`
# shares one `StreamSeeds` tuple with every *other* job at replicate `n`,
# regardless of arm or cell -- `ExperimentSpec.jobs()` (below) calls
# `derive_seeds(replicate)` exactly once per replicate and reuses the
# result across every cell in every arm, rather than deriving anything
# per-cell. `derive_seeds` is deterministic and injective in `replicate`
# (trivially -- `replicate` is reproduced verbatim in the output) and is
# the *only* place in this codebase that maps a replicate index to seeds;
# `ExperimentSpec.jobs()` and `firma_lab.runner` both call it, never
# re-derive it (see the module docstring's "do not re-derive" note, and
# `firma_lab.runner`'s own docstring).


@dataclass(frozen=True)
class StreamSeeds:
    """Mirrors ``firma_config::StreamSeeds`` (``crates/firma-config/src/
    lib.rs``) field-for-field: four independent ``u64`` seeds, one per
    RNG stream (manual Sec 21.3)."""

    mechanism: int
    environment: int
    shock: int
    init: int

    def to_dict(self) -> dict[str, int]:
        """The exact ``{"mechanism": ..., "environment": ..., "shock": ...,
        "init": ...}`` shape the ``"seeds"`` key of a ``firma run --model``
        config JSON expects."""
        return {
            "mechanism": self.mechanism,
            "environment": self.environment,
            "shock": self.shock,
            "init": self.init,
        }


def derive_seeds(replicate: int) -> StreamSeeds:
    """The single, pure, documented replicate -> ``StreamSeeds`` mapping
    (manual Sec 21.3, Sec 30.5; see this module's header comment for the
    adopted matched/varying-field design and its reasoning).

    Deterministic: same ``replicate`` always returns an equal ``StreamSeeds``.
    Injective in ``replicate``: distinct replicate indices never produce
    equal ``StreamSeeds`` tuples, because ``replicate`` is reproduced
    verbatim into every field -- two different inputs cannot collide on all
    four fields simultaneously (they would have to be the same input).

    Deliberately **not** bounded to ``1..=200`` here: that is the
    *registered E1 design's* own seed-range choice (Sec 30.5/Sec 30.6),
    enforced by ``ExperimentSpec`` (its ``num_replicates`` field), not a
    property of this general index-to-seeds function -- a later, separate
    H3/E3 filing (ADR-0051 Point 6) needs a *different*, non-overlapping
    replicate range from the same function, not a different function.

    Raises
    ------
    ValueError
        If ``replicate < 1`` -- replicate indices are 1-based throughout
        this module and the manual ("Seeds 1-200 per cell"), never 0-based.
    """
    if replicate < 1:
        raise ValueError(f"replicate index must be >= 1, got {replicate}")
    return StreamSeeds(
        mechanism=replicate, environment=replicate, shock=replicate, init=replicate
    )


# ---------------------------------------------------------------------------
# Part C1 -- ExperimentSpec
# ---------------------------------------------------------------------------


def _require_nonempty_str(value: Any, field_name: str) -> None:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{field_name} must be a non-empty string, got {value!r}")


def _require_nonempty_mapping(value: Any, field_name: str) -> None:
    if not isinstance(value, Mapping) or len(value) == 0:
        raise ValueError(f"{field_name} must be a non-empty mapping, got {value!r}")


# Where a `Factor`'s level gets written into a job's resolved config JSON.
# All four kinds are implemented by `firma_lab.runner`'s job-config builder:
#
#   "top_level"    -- ``level`` overwrites ``config[json_path[0]][...][json_path[-1]]``,
#                      a plain nested-dict traversal from the config root.
#   "rule_param"    -- ``level`` overwrites ``params[json_path[0]][...][json_path[-1]]``
#                      inside the named rule's own ``params`` object.
#   "rule_swap"     -- ``level`` is itself a *complete* rule-entry dict
#                      (``{"id": ..., "version": ..., "params": {...}}``)
#                      that replaces whichever existing rule's id starts
#                      with ``rule_category`` (e.g. ``"decision."``).
#   "direct_intervention" -- Sec 30.4's "h and ς set directly by
#                      intervention at each measurement tick" (ADR-0054).
#                      ``json_path`` (exactly one segment) names the target
#                      ``Intervention::SetAgentReal`` field key (e.g.
#                      ``"pinned_margin"``); ``level`` is the value. Emits
#                      one ``TimedIntervention`` (``{"at": 0, "op":
#                      "set_agent_real", ...}``) per agent in the config,
#                      plus one per ``companions`` entry (ADR-0054's "all
#                      four together" contract -- Arm A's `varsigma` factor
#                      carries the two fixed-zero `PINNED_SHORTFALL_*`
#                      companions its own swept pin needs alongside it).
#                      `firma_core::Intervention::SetAgentReal`
#                      (`crates/firma-core/src/intervention.rs`) and the
#                      four `firma_domain::keys::PINNED_*` constants
#                      (`crates/firma-domain/src/keys.rs`) this targets were
#                      implemented alongside this wiring (ADR-0054, this
#                      instruction).
_APPLY_KINDS = frozenset(
    {"top_level", "rule_param", "rule_swap", "direct_intervention"}
)


@dataclass(frozen=True)
class Factor:
    """One swept design factor (a row of manual Sec 30.4's table)."""

    name: str
    levels: tuple[Any, ...]
    arm: str
    apply_kind: str = "top_level"
    json_path: tuple[str, ...] | None = None
    rule_id: str | None = None
    rule_category: str | None = None
    # ADR-0054: additional (field, value) pins to emit, for the same
    # agent(s), alongside a `direct_intervention` factor's own swept pin --
    # e.g. Arm A's `varsigma` factor's companions are the two fixed-zero
    # `PINNED_SHORTFALL_*` keys the "all four together" contract
    # (ADR-0054 Part A item 1 / `Satisficing::read_pins`) requires
    # alongside a swept `PINNED_SHORTFALL_CAPITAL_GROWTH`. Only meaningful
    # for `apply_kind="direct_intervention"`.
    companions: tuple[tuple[str, float], ...] = ()

    def __post_init__(self) -> None:
        _require_nonempty_str(self.name, "Factor.name")
        _require_nonempty_str(self.arm, "Factor.arm")
        if not isinstance(self.levels, tuple) or len(self.levels) == 0:
            raise ValueError(f"Factor {self.name!r}: levels must be a non-empty tuple")
        if self.apply_kind not in _APPLY_KINDS:
            raise ValueError(
                f"Factor {self.name!r}: apply_kind must be one of {sorted(_APPLY_KINDS)}, "
                f"got {self.apply_kind!r}"
            )
        if self.apply_kind == "top_level" and not self.json_path:
            raise ValueError(f"Factor {self.name!r}: apply_kind=top_level requires json_path")
        if self.apply_kind == "rule_param" and (not self.json_path or not self.rule_id):
            raise ValueError(
                f"Factor {self.name!r}: apply_kind=rule_param requires both "
                "rule_id and json_path"
            )
        if self.apply_kind == "rule_swap" and not self.rule_category:
            raise ValueError(
                f"Factor {self.name!r}: apply_kind=rule_swap requires rule_category"
            )
        if self.apply_kind == "direct_intervention" and (
            not self.json_path or len(self.json_path) != 1
        ):
            raise ValueError(
                f"Factor {self.name!r}: apply_kind=direct_intervention requires "
                "json_path to be exactly one segment (the target "
                "Intervention::SetAgentReal field key, e.g. 'pinned_margin')"
            )
        if self.companions and self.apply_kind != "direct_intervention":
            raise ValueError(
                f"Factor {self.name!r}: companions is only meaningful for "
                "apply_kind=direct_intervention"
            )

    def to_dict(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "levels": list(self.levels),
            "arm": self.arm,
            "apply_kind": self.apply_kind,
            "json_path": list(self.json_path) if self.json_path else None,
            "rule_id": self.rule_id,
            "rule_category": self.rule_category,
            "companions": [list(c) for c in self.companions],
        }


@dataclass(frozen=True)
class ArmDesign:
    """One arm's full factorial design (manual Sec 30.4): the cartesian
    product of its factors' levels is its cell set."""

    name: str
    factors: tuple[Factor, ...]

    def __post_init__(self) -> None:
        _require_nonempty_str(self.name, "ArmDesign.name")
        if not isinstance(self.factors, tuple) or len(self.factors) == 0:
            raise ValueError(f"ArmDesign {self.name!r}: factors must be non-empty")
        names_seen: set[str] = set()
        for f in self.factors:
            if f.arm != self.name:
                raise ValueError(
                    f"ArmDesign {self.name!r}: factor {f.name!r} declares "
                    f"arm={f.arm!r}, expected {self.name!r}"
                )
            if f.name in names_seen:
                raise ValueError(
                    f"ArmDesign {self.name!r}: duplicate factor name {f.name!r}"
                )
            names_seen.add(f.name)

    def cells(self) -> list[dict[str, Any]]:
        """Every cell as a ``{factor_name: level}`` dict, in the deterministic
        order ``itertools.product`` over the factors as declared."""
        names = [f.name for f in self.factors]
        out = []
        for combo in itertools.product(*(f.levels for f in self.factors)):
            out.append(dict(zip(names, combo)))
        return out

    @property
    def cell_count(self) -> int:
        n = 1
        for f in self.factors:
            n *= len(f.levels)
        return n

    def to_dict(self) -> dict[str, Any]:
        return {"name": self.name, "factors": [f.to_dict() for f in self.factors]}


@dataclass(frozen=True)
class Hypothesis:
    """One row of manual Sec 30.3 (hypotheses with predicted signs)."""

    id: str
    text: str
    predicted_sign: str
    arm: str
    primary_dv: str

    def __post_init__(self) -> None:
        for name, value in (
            ("id", self.id),
            ("text", self.text),
            ("predicted_sign", self.predicted_sign),
            ("arm", self.arm),
            ("primary_dv", self.primary_dv),
        ):
            _require_nonempty_str(value, f"Hypothesis.{name}")

    def to_dict(self) -> dict[str, Any]:
        return {
            "id": self.id,
            "text": self.text,
            "predicted_sign": self.predicted_sign,
            "arm": self.arm,
            "primary_dv": self.primary_dv,
        }


@dataclass(frozen=True)
class Job:
    """One concrete unit of work: an arm, a cell within it, a replicate, and
    the ``StreamSeeds`` that replicate derives to (Sec B above)."""

    arm: str
    cell_index: int
    factor_levels: Mapping[str, Any]
    replicate: int
    seeds: StreamSeeds


@dataclass(frozen=True)
class ExperimentSpec:
    """A typed, validated, content-hashable pre-registration design (manual
    Sec 28.1). Required fields enumerated there are all present and
    non-silently-defaulted, per this project's "no silent defaults"
    discipline (the same posture ``firma-domain``'s required-config types
    take, applied here in Python): a malformed or incomplete spec fails at
    construction (``__post_init__``), not at execution time.
    """

    id: str
    schema_version: str
    engine_requirement: str
    question_id: str
    hypotheses: tuple[Hypothesis, ...]
    arms: tuple[ArmDesign, ...]
    model_config_template: Mapping[str, Any]
    num_replicates: int
    interventions: tuple[Mapping[str, Any], ...]
    observation: tuple[str, ...]
    analysis_plan: Mapping[str, Any]
    stopping_rules: str
    alternative_explanation: str

    def __post_init__(self) -> None:
        _require_nonempty_str(self.id, "id")
        _require_nonempty_str(self.schema_version, "schema_version")
        _require_nonempty_str(self.engine_requirement, "engine_requirement")
        _require_nonempty_str(self.question_id, "question_id")
        if not isinstance(self.hypotheses, tuple) or len(self.hypotheses) == 0:
            raise ValueError("hypotheses must be a non-empty tuple")
        if not isinstance(self.arms, tuple) or len(self.arms) == 0:
            raise ValueError("arms must be a non-empty tuple")
        arm_names = [a.name for a in self.arms]
        if len(arm_names) != len(set(arm_names)):
            raise ValueError(f"duplicate arm names: {arm_names}")
        hyp_arms = {h.arm for h in self.hypotheses}
        if not hyp_arms.issubset(set(arm_names)):
            raise ValueError(
                f"hypotheses reference arm(s) not present in arms: "
                f"{hyp_arms - set(arm_names)}"
            )
        _require_nonempty_mapping(self.model_config_template, "model_config_template")
        if not isinstance(self.num_replicates, int) or self.num_replicates < 1:
            raise ValueError(
                f"num_replicates must be a positive int, got {self.num_replicates!r}"
            )
        if not isinstance(self.observation, tuple) or len(self.observation) == 0:
            raise ValueError("observation must be a non-empty tuple of DV names")
        _require_nonempty_mapping(self.analysis_plan, "analysis_plan")
        _require_nonempty_str(self.stopping_rules, "stopping_rules")
        _require_nonempty_str(self.alternative_explanation, "alternative_explanation")

    # -- cells / jobs --------------------------------------------------

    def arm(self, name: str) -> ArmDesign:
        for a in self.arms:
            if a.name == name:
                return a
        raise KeyError(f"no such arm: {name!r}")

    def total_cells(self) -> int:
        return sum(a.cell_count for a in self.arms)

    def total_jobs(self) -> int:
        return self.total_cells() * self.num_replicates

    def jobs(self) -> Iterator[Job]:
        """Expand every (arm, cell, replicate) combination. ``derive_seeds``
        is called exactly once per replicate (not per cell) and the result
        is reused across every cell at that replicate -- the matched/CRN
        property enforced in code, not merely documented (see this
        module's Part B header comment)."""
        seeds_by_replicate = {
            n: derive_seeds(n) for n in range(1, self.num_replicates + 1)
        }
        for a in self.arms:
            for cell_index, levels in enumerate(a.cells()):
                for n in range(1, self.num_replicates + 1):
                    yield Job(
                        arm=a.name,
                        cell_index=cell_index,
                        factor_levels=levels,
                        replicate=n,
                        seeds=seeds_by_replicate[n],
                    )

    # -- hashing ---------------------------------------------------------

    def to_canonical_dict(self) -> dict[str, Any]:
        return {
            "id": self.id,
            "schema_version": self.schema_version,
            "engine_requirement": self.engine_requirement,
            "question_id": self.question_id,
            "hypotheses": [h.to_dict() for h in self.hypotheses],
            "arms": [a.to_dict() for a in self.arms],
            "model_config_template": json.loads(json.dumps(self.model_config_template)),
            "num_replicates": self.num_replicates,
            "interventions": [json.loads(json.dumps(i)) for i in self.interventions],
            "observation": list(self.observation),
            "analysis_plan": json.loads(json.dumps(self.analysis_plan)),
            "stopping_rules": self.stopping_rules,
            "alternative_explanation": self.alternative_explanation,
        }

    def content_hash(self) -> str:
        """SHA-256 hex over this spec's canonical JSON serialization (manual
        Sec 23.1/Sec 28.1: "Its hash is computed before any run executes and
        is the experiment's identity"). Canonical = ``sort_keys=True`` and
        the tightest separators, so two specs with identical content always
        hash equal regardless of Python dict insertion order."""
        canonical = json.dumps(
            self.to_canonical_dict(), sort_keys=True, separators=(",", ":")
        )
        return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


# ---------------------------------------------------------------------------
# Builders
# ---------------------------------------------------------------------------


def build_narrowed_e1_spec(
    model_config_template: Mapping[str, Any],
    *,
    num_replicates: int = 200,
) -> ExperimentSpec:
    """The ADR-0051 **narrowed** E1 design -- 195 cells (Arm A 125, Arm B 60,
    Arm C 10), *not* the manual's original 315-cell table (ADR-0051 excludes
    Arm B's "Shaping lag" factor and H3/E3 from this filing's registered
    design). Factor names/levels read directly from manual Sec 30.4 and
    cross-checked against ADR-0051's own cell-count table.

    ``model_config_template`` is a **required** parameter with no default:
    the manual's Sec 30.4 table specifies factor *levels*, not a base agent
    population / environment configuration for the parts of each cell's
    config that are *not* swept -- inventing one silently here would be
    exactly the kind of silent default this project's discipline forbids.
    The caller must supply a complete base ``firma run --model`` config
    (see ``firma_lab.runner`` for the shape); this function only patches
    each cell's swept factors into a copy of it, once, at execution time
    (``firma_lab.runner.build_job_config`` -- not here, since a spec's
    validity should not depend on execution-time patching succeeding).
    """
    arm_a = ArmDesign(
        "A",
        (
            Factor(
                "h",
                (0.02, 0.05, 0.10, 0.20, 0.40),
                arm="A",
                apply_kind="direct_intervention",
                json_path=("pinned_margin",),
            ),
            # ADR-0054's accepted shortfall-mapping decision (Option B,
            # matching `vt8_orthogonal_manipulation_of_h_and_shortfall`'s
            # own construction exactly): sweep only ς_1 (capital growth);
            # ς_2/ς_3 are fixed at 0.0 via `companions`, not left organic
            # -- required alongside this factor's own pin by the "all four
            # together" contract (ADR-0054 Part A item 1).
            Factor(
                "varsigma",
                (0.0, 0.25, 0.50, 0.75, 1.0),
                arm="A",
                apply_kind="direct_intervention",
                json_path=("pinned_shortfall_capital_growth",),
                companions=(
                    ("pinned_shortfall_capability", 0.0),
                    ("pinned_shortfall_obligation_clearance", 0.0),
                ),
            ),
            Factor(
                "beta",
                (0, 0.5, 1, 2, 4),
                arm="A",
                apply_kind="rule_param",
                rule_id="decision.satisficing",
                json_path=("beta",),
            ),
        ),
    )
    arm_b = ArmDesign(
        "B",
        (
            Factor(
                "beta",
                (0, 0.5, 1, 2, 4),
                arm="B",
                apply_kind="rule_param",
                rule_id="decision.satisficing",
                json_path=("beta",),
            ),
            Factor(
                "novelty",
                (0.0, 0.4, 0.8),
                arm="B",
                apply_kind="rule_param",
                rule_id="shock.scheduled",
                json_path=("shocks", "*", "novelty"),
            ),
            Factor(
                "shock_magnitude",
                # manual Sec 30.4: "4 levels", magnitudes unspecified there --
                # ADR-00NN (this instruction's Part B ADR) does not pin these
                # either (out of scope: seed semantics, not shock design);
                # left as an explicit placeholder, flagged, not invented as
                # if settled.
                ("level_1", "level_2", "level_3", "level_4"),
                arm="B",
                apply_kind="rule_param",
                rule_id="shock.scheduled",
                json_path=("shocks", "*", "magnitude"),
            ),
            # NOTE: Arm B's "Shaping lag" factor (3 levels) is deliberately
            # ABSENT -- ADR-0051's own exclusion. Do not add it back without
            # a new ADR (ADR-0051 Point 3's standing anti-tuning constraint
            # covers exactly this kind of silent re-inclusion).
        ),
    )
    arm_c = ArmDesign(
        "C",
        (
            Factor(
                "beta",
                (0, 0.5, 1, 2, 4),
                arm="C",
                apply_kind="rule_param",
                rule_id="decision.satisficing",
                json_path=("beta",),
            ),
            Factor(
                "decision_plugin",
                (
                    {
                        "id": "decision.satisficing",
                        "version": "^1",
                        "params": {},
                    },
                    {
                        "id": "decision.random",
                        "version": "^1",
                        "params": {},
                    },
                ),
                arm="C",
                apply_kind="rule_swap",
                rule_category="decision.",
            ),
        ),
    )
    assert arm_a.cell_count == 125, arm_a.cell_count
    assert arm_b.cell_count == 60, arm_b.cell_count
    assert arm_c.cell_count == 10, arm_c.cell_count

    hypotheses = (
        Hypothesis(
            "H1a",
            "Holding h constant, increasing shortfall varsigma increases "
            "search width and repertoire entropy.",
            "positive",
            "A",
            "search_width",
        ),
        Hypothesis(
            "H1b",
            "Holding shortfall varsigma constant, decreasing h decreases them.",
            "positive",
            "A",
            "search_width",
        ),
        Hypothesis(
            "H1c",
            "Where h and varsigma covary endogenously, the relationship is "
            "non-monotonic (inverted-U vs. discontinuity).",
            "quadratic",
            "B",
            "repertoire_entropy",
        ),
        Hypothesis(
            "H2",
            "At matched h, threat novelty produces greater narrowing than "
            "threat magnitude.",
            "novelty>magnitude",
            "B",
            "repertoire_entropy",
        ),
        Hypothesis(
            "H4",
            "Narrowing improves survival under low-novelty threats and "
            "impairs it under high-novelty threats.",
            "interaction",
            "B",
            "survival_time",
        ),
        # H3 is deliberately absent -- ADR-0051: excluded from this
        # filing's registered design, not descoped as a hypothesis
        # (ADR-0050's disposition is unmodified; H3 stays open, filed
        # separately per ADR-0051 Point 6's re-filing path).
    )

    return ExperimentSpec(
        id="e1-narrowed",
        schema_version="1.0.0",
        engine_requirement=">=0.1.0, <0.2.0",
        question_id="dissociating-shortfall-from-viability-proximity",
        hypotheses=hypotheses,
        arms=(arm_a, arm_b, arm_c),
        model_config_template=model_config_template,
        num_replicates=num_replicates,
        interventions=(),
        observation=(
            "repertoire_entropy",
            "search_width",
            "survival_time",
            "shaping_abandonment",
            "dependence",
        ),
        analysis_plan={
            "H1a_H1b": "delta_H_rep ~ varsigma + h + varsigma:h + beta + (1|seed)",
            "H1c": "linear vs. quadratic fit; quadratic term CI excludes zero "
            "and preferred by cross-validated error",
            "H2": "contrast novelty and magnitude coefficients at matched h",
            "H4": "Cox proportional hazards",
            "inference": "bootstrap 95% CI (10000 resamples) primary; "
            "p-values secondary",
            "multiplicity": "primary outcome uncorrected; secondary "
            "outcomes Benjamini-Hochberg FDR q=0.05",
        },
        stopping_rules="All jobs execute to completion or firm extinction. "
        "No interim analysis, no optional stopping (manual Sec 30.6).",
        alternative_explanation="For Arm A/B, narrowing could reflect "
        "admissibility shrinking (fewer affordable actions) rather than "
        "decision narrowing -- discriminated by reporting |A_used| against "
        "|A_adm| (manual Sec 11.4, Sec 28.2/Sec 28.3).",
    )


def build_synthetic_spec(
    model_config_template: Mapping[str, Any] | None = None,
    *,
    num_replicates: int = 3,
) -> ExperimentSpec:
    """A small, deliberately synthetic spec for pipeline testing -- 2
    factors x 2 levels in one arm x ``num_replicates`` replicates = 4 x
    num_replicates jobs. Exercises ``top_level`` and ``rule_param`` apply
    kinds (``direct_intervention`` and ``rule_swap`` are each exercised by
    their own dedicated tests instead, against ADR-0054's real pin keys and
    the real narrowed-E1 spec respectively). Not the real E1 design; never
    confuse this spec's ``id`` with ``build_narrowed_e1_spec``'s.
    """
    # Uses `decision.satisficing` (a real `model_registry()` plugin), not a
    # `testkit.*` rule: `firma run --model` (the flag the real narrowed-E1
    # pipeline uses, per C1/C2) resolves against `model_registry()`, which
    # does not include the testkit plugins at all (confirmed directly,
    # `crates/firma-cli/src/registry_setup.rs`) -- a `testkit.*`-based
    # synthetic config would fail to parse under `--model`, testing nothing.
    template = model_config_template or {
        "experiment": "synthetic-pipeline-test",
        "schema_version": "1.0.0",
        "engine": ">=0.1.0, <0.2.0",
        "world": {
            "ticks": 5,
            "resources": ["capital", "input"],
            "conflict_resolver": {"id": "conflict.additive", "version": "^1"},
            "global_reals": {"theta_limit": 0.9, "theta_cap": 0.2},
            "global_ints": {"theta_q": 100, "input_price": 2, "output_price": 3},
        },
        "agents": [
            {
                "id": 0,
                "stocks": {"capital": 500, "input": 500},
                "reals": {"capability": 0.5, "legitimacy": 1.0},
            }
        ],
        "environment": {"stocks": {"capital": 100000000, "input": 100000000}},
        "rules": [
            {"id": "decision.satisficing", "version": "^1", "params": {"l_w": 8}}
        ],
    }
    arm_t = ArmDesign(
        "T",
        (
            Factor(
                "ticks",
                (5, 8),
                arm="T",
                apply_kind="top_level",
                json_path=("world", "ticks"),
            ),
            Factor(
                "l_w",
                (4, 8),
                arm="T",
                apply_kind="rule_param",
                rule_id="decision.satisficing",
                json_path=("l_w",),
            ),
        ),
    )
    return ExperimentSpec(
        id="synthetic-pipeline-test",
        schema_version="1.0.0",
        engine_requirement=">=0.1.0, <0.2.0",
        question_id="pipeline-self-test",
        hypotheses=(
            Hypothesis(
                "HT",
                "Synthetic: no real prediction.",
                "n/a",
                "T",
                "n/a",
            ),
        ),
        arms=(arm_t,),
        model_config_template=template,
        num_replicates=num_replicates,
        interventions=(),
        observation=("n/a",),
        analysis_plan={"note": "pipeline self-test only"},
        stopping_rules="All jobs execute to completion.",
        alternative_explanation="n/a -- synthetic pipeline test.",
    )
