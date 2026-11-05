"""firma_lab.spec -- seed-derivation and ExperimentSpec tests (this
instruction's Part B / Part C1/C3).

Pure-Python tests: no ``firma`` binary invocation here (that's
``test_runner.py``'s job) -- these check the index arithmetic and
validation discipline in isolation.
"""

from __future__ import annotations

import copy

import pytest

from firma_lab.spec import (
    ArmDesign,
    ExperimentSpec,
    Factor,
    Hypothesis,
    StreamSeeds,
    build_narrowed_e1_spec,
    build_synthetic_spec,
    derive_seeds,
)

# ---------------------------------------------------------------------------
# Part B -- derive_seeds
# ---------------------------------------------------------------------------


def test_derive_seeds_deterministic() -> None:
    assert derive_seeds(1) == derive_seeds(1)
    assert derive_seeds(200) == derive_seeds(200)


def test_derive_seeds_matched_across_all_four_fields() -> None:
    """The adopted design (this module's header comment): every field gets
    the *same* value as the replicate index, not just environment/shock."""
    for n in (1, 7, 55, 200):
        s = derive_seeds(n)
        assert s == StreamSeeds(mechanism=n, environment=n, shock=n, init=n)
        assert s.mechanism == s.environment == s.shock == s.init == n


def test_derive_seeds_injective_no_collisions_1_to_200() -> None:
    seen: dict[StreamSeeds, int] = {}
    for n in range(1, 201):
        s = derive_seeds(n)
        assert s not in seen, f"replicate {n} collides with replicate {seen[s]}"
        seen[s] = n
    assert len(seen) == 200


def test_derive_seeds_rejects_zero_and_negative() -> None:
    with pytest.raises(ValueError):
        derive_seeds(0)
    with pytest.raises(ValueError):
        derive_seeds(-1)


def test_derive_seeds_not_bounded_to_200() -> None:
    """A later, separate H3/E3 filing (ADR-0051 Point 6) needs a
    non-overlapping replicate range from the *same* function -- confirm it
    accepts values beyond 200 rather than being artificially capped."""
    s = derive_seeds(201)
    assert s == StreamSeeds(201, 201, 201, 201)


def test_stream_seeds_to_dict_shape() -> None:
    s = StreamSeeds(1, 2, 3, 4)
    assert s.to_dict() == {"mechanism": 1, "environment": 2, "shock": 3, "init": 4}


# ---------------------------------------------------------------------------
# Factor / ArmDesign validation
# ---------------------------------------------------------------------------


def test_factor_rejects_empty_levels() -> None:
    with pytest.raises(ValueError):
        Factor("x", (), arm="A")


def test_factor_rejects_bad_apply_kind() -> None:
    with pytest.raises(ValueError):
        Factor("x", (1,), arm="A", apply_kind="not_a_real_kind")


def test_factor_top_level_requires_json_path() -> None:
    with pytest.raises(ValueError):
        Factor("x", (1,), arm="A", apply_kind="top_level", json_path=None)


def test_factor_rule_param_requires_rule_id_and_path() -> None:
    with pytest.raises(ValueError):
        Factor("x", (1,), arm="A", apply_kind="rule_param", json_path=("y",))
    with pytest.raises(ValueError):
        Factor("x", (1,), arm="A", apply_kind="rule_param", rule_id="decision.satisficing")


def test_factor_rule_swap_requires_rule_category() -> None:
    with pytest.raises(ValueError):
        Factor("x", ({"id": "a"},), arm="A", apply_kind="rule_swap")


def test_arm_design_rejects_factor_arm_mismatch() -> None:
    bad = Factor("x", (1,), arm="B", apply_kind="top_level", json_path=("y",))
    with pytest.raises(ValueError):
        ArmDesign("A", (bad,))


def test_arm_design_rejects_duplicate_factor_names() -> None:
    f1 = Factor("x", (1,), arm="A", apply_kind="top_level", json_path=("y",))
    f2 = Factor("x", (2,), arm="A", apply_kind="top_level", json_path=("z",))
    with pytest.raises(ValueError):
        ArmDesign("A", (f1, f2))


def test_arm_design_cell_count_and_cells() -> None:
    f1 = Factor("x", (1, 2), arm="A", apply_kind="top_level", json_path=("p",))
    f2 = Factor("y", (10, 20, 30), arm="A", apply_kind="top_level", json_path=("q",))
    arm = ArmDesign("A", (f1, f2))
    assert arm.cell_count == 6
    cells = arm.cells()
    assert len(cells) == 6
    assert {"x": 1, "y": 10} in cells
    assert {"x": 2, "y": 30} in cells


# ---------------------------------------------------------------------------
# ExperimentSpec validation ("no silent defaults")
# ---------------------------------------------------------------------------


def _minimal_kwargs() -> dict:
    f = Factor("x", (1, 2), arm="A", apply_kind="top_level", json_path=("p",))
    return dict(
        id="s",
        schema_version="1.0.0",
        engine_requirement=">=0.1.0",
        question_id="q",
        hypotheses=(Hypothesis("H", "text", "positive", "A", "dv"),),
        arms=(ArmDesign("A", (f,)),),
        model_config_template={"a": 1},
        num_replicates=2,
        interventions=(),
        observation=("dv",),
        analysis_plan={"note": "x"},
        stopping_rules="x",
        alternative_explanation="x",
    )


def test_experiment_spec_accepts_valid_minimal_kwargs() -> None:
    ExperimentSpec(**_minimal_kwargs())


@pytest.mark.parametrize(
    "field_name,bad_value",
    [
        ("id", ""),
        ("schema_version", "   "),
        ("hypotheses", ()),
        ("arms", ()),
        ("model_config_template", {}),
        ("num_replicates", 0),
        ("num_replicates", -5),
        ("observation", ()),
        ("analysis_plan", {}),
        ("stopping_rules", ""),
        ("alternative_explanation", ""),
    ],
)
def test_experiment_spec_rejects_malformed_fields_at_construction(
    field_name: str, bad_value: object
) -> None:
    kwargs = _minimal_kwargs()
    kwargs[field_name] = bad_value
    with pytest.raises(ValueError):
        ExperimentSpec(**kwargs)


def test_experiment_spec_rejects_duplicate_arm_names() -> None:
    f = Factor("x", (1,), arm="A", apply_kind="top_level", json_path=("p",))
    kwargs = _minimal_kwargs()
    kwargs["arms"] = (ArmDesign("A", (f,)), ArmDesign("A", (f,)))
    with pytest.raises(ValueError):
        ExperimentSpec(**kwargs)


def test_experiment_spec_rejects_hypothesis_referencing_unknown_arm() -> None:
    kwargs = _minimal_kwargs()
    kwargs["hypotheses"] = (Hypothesis("H", "text", "positive", "NOPE", "dv"),)
    with pytest.raises(ValueError):
        ExperimentSpec(**kwargs)


# ---------------------------------------------------------------------------
# content_hash
# ---------------------------------------------------------------------------


def test_content_hash_deterministic() -> None:
    spec = ExperimentSpec(**_minimal_kwargs())
    assert spec.content_hash() == spec.content_hash()


def test_content_hash_order_independent_dict_construction() -> None:
    kwargs1 = _minimal_kwargs()
    kwargs1["model_config_template"] = {"a": 1, "b": 2}
    kwargs2 = _minimal_kwargs()
    kwargs2["model_config_template"] = {"b": 2, "a": 1}  # different insertion order
    assert ExperimentSpec(**kwargs1).content_hash() == ExperimentSpec(**kwargs2).content_hash()


def test_content_hash_changes_when_content_changes() -> None:
    kwargs1 = _minimal_kwargs()
    kwargs2 = _minimal_kwargs()
    kwargs2["model_config_template"] = {"a": 2}  # different from kwargs1's {"a": 1}
    h1 = ExperimentSpec(**kwargs1).content_hash()
    h2 = ExperimentSpec(**kwargs2).content_hash()
    assert h1 != h2


def test_content_hash_is_64_hex_chars() -> None:
    h = ExperimentSpec(**_minimal_kwargs()).content_hash()
    assert len(h) == 64
    int(h, 16)  # raises if not valid hex


# ---------------------------------------------------------------------------
# jobs() -- the matched/varying property, enforced not just documented
# ---------------------------------------------------------------------------


def test_jobs_matched_across_cells_at_the_same_replicate() -> None:
    """For a fixed replicate n, every job (any arm, any cell) shares the
    identical StreamSeeds tuple -- the CRN property, checked directly on
    real job output, not just asserted from derive_seeds in isolation."""
    spec = build_synthetic_spec(num_replicates=5)
    jobs_by_replicate: dict[int, list[StreamSeeds]] = {}
    for job in spec.jobs():
        jobs_by_replicate.setdefault(job.replicate, []).append(job.seeds)
    for n, seeds_list in jobs_by_replicate.items():
        assert all(s == seeds_list[0] for s in seeds_list), (
            f"replicate {n}: seeds differ across cells: {set(seeds_list)}"
        )
        assert seeds_list[0] == StreamSeeds(n, n, n, n)


def test_jobs_vary_genuinely_across_replicates() -> None:
    spec = build_synthetic_spec(num_replicates=5)
    seeds_per_replicate = {job.replicate: job.seeds for job in spec.jobs()}
    assert len(set(seeds_per_replicate.values())) == 5


def test_jobs_count_matches_total_jobs() -> None:
    spec = build_synthetic_spec(num_replicates=3)
    assert len(list(spec.jobs())) == spec.total_jobs()


# ---------------------------------------------------------------------------
# build_narrowed_e1_spec -- ADR-0051's 195-cell design
# ---------------------------------------------------------------------------


def _real_template() -> dict:
    return {
        "experiment": "e1",
        "schema_version": "1.0.0",
        "engine": ">=0.1.0, <0.2.0",
        "world": {},
        "agents": [],
        "environment": {},
        "rules": [
            {"id": "decision.satisficing", "version": "^1", "params": {}},
            {
                "id": "shock.scheduled",
                "version": "^1",
                "params": {"shocks": [{"novelty": 0.1, "magnitude": 1}]},
            },
        ],
    }


def test_narrowed_e1_spec_cell_counts_match_adr_0051() -> None:
    spec = build_narrowed_e1_spec(_real_template())
    assert spec.arm("A").cell_count == 125
    assert spec.arm("B").cell_count == 60
    assert spec.arm("C").cell_count == 10
    assert spec.total_cells() == 195


def test_narrowed_e1_spec_total_jobs_at_200_replicates() -> None:
    spec = build_narrowed_e1_spec(_real_template())
    assert spec.num_replicates == 200
    assert spec.total_jobs() == 195 * 200 == 39000


def test_narrowed_e1_spec_arm_b_has_no_shaping_lag_factor() -> None:
    """ADR-0051's own exclusion, enforced: Arm B has exactly 3 factors
    (beta, novelty, shock_magnitude), never a fourth 'shaping_lag'."""
    spec = build_narrowed_e1_spec(_real_template())
    names = {f.name for f in spec.arm("B").factors}
    assert names == {"beta", "novelty", "shock_magnitude"}
    assert "shaping_lag" not in names


def test_narrowed_e1_spec_h3_absent_from_hypotheses() -> None:
    ids = {h.id for h in build_narrowed_e1_spec(_real_template()).hypotheses}
    assert "H3" not in ids
    assert ids == {"H1a", "H1b", "H1c", "H2", "H4"}


def test_narrowed_e1_spec_constructs_and_hashes() -> None:
    spec = build_narrowed_e1_spec(_real_template())
    h = spec.content_hash()
    assert len(h) == 64
    # constructing twice from equal input hashes equal
    spec2 = build_narrowed_e1_spec(_real_template())
    assert spec.content_hash() == spec2.content_hash()


def test_narrowed_e1_spec_requires_explicit_model_config_template() -> None:
    """No silent default: omitting the template is a TypeError (a required
    positional/keyword argument), not a quietly-invented base population."""
    with pytest.raises(TypeError):
        build_narrowed_e1_spec()  # type: ignore[call-arg]


def test_narrowed_e1_spec_no_full_job_id_collision_across_39000_jobs() -> None:
    """The full real job space: every (arm, cell, replicate) triple must
    produce a unique identity. Checked against the *cell/replicate* tuple
    directly (cheaper than materialising 39000 dicts to compare)."""
    spec = build_narrowed_e1_spec(_real_template())
    seen: set[tuple[str, int, int]] = set()
    count = 0
    for job in spec.jobs():
        key = (job.arm, job.cell_index, job.replicate)
        assert key not in seen, f"duplicate job key: {key}"
        seen.add(key)
        count += 1
    assert count == 39000
    assert len(seen) == 39000
