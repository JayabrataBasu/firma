"""Run-safety guards (audit 2026-10-01, F06 / F18; manual Sec 30.5, Sec 30.6,
Sec 30.9; ADR-0053; PROGRESS.md OQ-14).

Nothing in this file executes the ``firma`` binary: ``run_experiment``'s
per-job executor is replaced by a stub, and the fixture scan only reads
source files. That is deliberate -- these tests are about refusing to run.
"""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path

import pytest

from firma_lab import prereg, runner
from firma_lab.runner import JobRecord, ReservedSeedError, build_job_config, job_id, run_experiment
from firma_lab.spec import (
    RESERVED_SEED_RANGE,
    build_narrowed_e1_spec,
    build_synthetic_spec,
    seeds_in_reserved_range,
)

REPO_ROOT = Path(__file__).resolve().parents[2]


@pytest.fixture
def stub(monkeypatch, tmp_path):
    """Replace the per-job executor with one that records the call and
    returns a completed record. Returns ``(calls, fake_bin)``."""
    calls: list = []

    def fake_run_one_job(spec, job, firma_bin, jobs_root):
        calls.append(job)
        return JobRecord(
            job_id=job_id(spec, job), spec_id=spec.id, spec_content_hash=spec.content_hash(),
            arm=job.arm, cell_index=job.cell_index, factor_levels=dict(job.factor_levels),
            replicate=job.replicate, seeds=job.seeds.to_dict(), resolved_config={},
            status="completed",
        )

    monkeypatch.setattr(runner, "_run_one_job", fake_run_one_job)
    fake_bin = tmp_path / "not-a-real-firma"
    fake_bin.write_text("stub; never executed")
    return calls, fake_bin


# -- F06 item 1: the seed guard ---------------------------------------------


def test_run_experiment_refuses_reserved_seeds_before_any_job_executes(stub, tmp_path):
    calls, fake_bin = stub
    spec = build_synthetic_spec(num_replicates=3, first_replicate=1)  # seeds (1..3, x4)
    with pytest.raises(ReservedSeedError) as e:
        run_experiment(spec, tmp_path / "out", fake_bin)
    assert calls == [], "a job was executed before the guard fired"
    assert "allow_reserved_seed_range" in str(e.value)
    assert not (tmp_path / "out").exists(), "the guard must fire before anything is written"


def test_guard_fires_when_only_the_last_replicate_is_reserved(stub, tmp_path):
    calls, fake_bin = stub
    spec = build_synthetic_spec(num_replicates=3, first_replicate=198)  # 198, 199, 200
    with pytest.raises(ReservedSeedError):
        run_experiment(spec, tmp_path / "out", fake_bin)
    assert calls == []


def test_guard_also_covers_a_caller_supplied_job_subset(stub, tmp_path):
    calls, fake_bin = stub
    spec = build_synthetic_spec(num_replicates=2, first_replicate=1)
    with pytest.raises(ReservedSeedError):
        run_experiment(spec, tmp_path / "out", fake_bin, jobs=list(spec.jobs())[:1])
    assert calls == []


def test_override_is_explicit_and_then_every_job_runs(stub, tmp_path):
    calls, fake_bin = stub
    spec = build_synthetic_spec(num_replicates=2, first_replicate=1)
    result = run_experiment(spec, tmp_path / "out", fake_bin, allow_reserved_seed_range=True)
    assert len(calls) == spec.total_jobs() == len(result.completed)


def test_out_of_range_spec_runs_without_the_override(stub, tmp_path):
    calls, fake_bin = stub
    spec = build_synthetic_spec(num_replicates=2)  # default first_replicate is outside 1-200
    result = run_experiment(spec, tmp_path / "out", fake_bin)
    assert len(calls) == spec.total_jobs() == len(result.completed)


def test_the_real_narrowed_e1_spec_is_refused_by_default(stub, tmp_path):
    calls, fake_bin = stub
    spec = build_narrowed_e1_spec(dict(build_synthetic_spec().model_config_template), num_replicates=2)
    with pytest.raises(ReservedSeedError):
        run_experiment(spec, tmp_path / "out", fake_bin)
    assert calls == []


def test_reserved_range_is_the_manual_1_to_200():
    assert (RESERVED_SEED_RANGE.start, RESERVED_SEED_RANGE.stop) == (1, 201)
    spec = build_synthetic_spec(num_replicates=1, first_replicate=200)
    assert seeds_in_reserved_range(next(spec.jobs()).seeds)
    spec = build_synthetic_spec(num_replicates=1, first_replicate=201)
    assert seeds_in_reserved_range(next(spec.jobs()).seeds) == []


# -- F06 item 2: fixtures that execute runs stay outside the range -----------


def _reserved_tuple(t) -> bool:
    return len(set(t)) == 1 and t[0] in RESERVED_SEED_RANGE


def test_no_rust_or_shipped_config_fixture_uses_a_reserved_seed_tuple():
    """Every seed literal under tests/, crates/ and configs/ belongs to a
    fixture that is executed by ``cargo test`` or shipped as a runnable
    config. None may be an exact ``(n, n, n, n)`` with n in 1-200 -- the
    only tuple shape ``derive_seeds`` assigns to a registered run."""
    hits = [
        (str(Path(p).relative_to(REPO_ROOT)), line, t)
        for p, line, t in prereg.scan_seed_literals(
            [REPO_ROOT / "tests", REPO_ROOT / "crates", REPO_ROOT / "configs"],
            suffixes=(".rs", ".json"),
        )
        if "/target/" not in p and _reserved_tuple(t)
    ]
    assert hits == []


def _load_test_runner_module():
    path = Path(__file__).with_name("test_runner.py")
    spec = importlib.util.spec_from_file_location("_test_runner_for_seed_check", path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def test_python_pipeline_fixtures_derive_seeds_outside_the_reserved_range():
    """``test_runner.py`` executes real ``firma run --model`` subprocesses
    from these two specs; their derived seeds must be outside 1-200."""
    specs = {
        "build_synthetic_spec()": build_synthetic_spec(),
        "test_runner._failing_spec()": _load_test_runner_module()._failing_spec(),
    }
    bad = {
        name: sorted({j.replicate for j in s.jobs() if seeds_in_reserved_range(j.seeds)})
        for name, s in specs.items()
    }
    assert {k: v for k, v in bad.items() if v} == {}


# -- F18 / OQ-14: one firm per directly-manipulated job ----------------------


def _two_agent_template() -> dict:
    t = json.loads(json.dumps(build_synthetic_spec().model_config_template))
    t["agents"].append({"id": 1, "stocks": {"capital": 500, "input": 500},
                        "reals": {"capability": 0.5, "legitimacy": 1.0}})
    return t


def test_build_job_config_rejects_a_multi_agent_arm_a_template():
    spec = build_narrowed_e1_spec(_two_agent_template(), num_replicates=1)
    job = next(j for j in spec.jobs() if j.arm == "A")
    with pytest.raises(ValueError, match="exactly one agent"):
        build_job_config(spec, job)


def test_build_job_config_still_accepts_a_single_agent_arm_a_template():
    spec = build_narrowed_e1_spec(dict(build_synthetic_spec().model_config_template), num_replicates=1)
    job = next(j for j in spec.jobs() if j.arm == "A")
    cfg = build_job_config(spec, job)
    assert {i["agent"] for i in cfg["interventions"]} == {0}


def test_multi_agent_template_is_fine_for_an_arm_without_direct_intervention():
    spec = build_synthetic_spec(_two_agent_template())
    cfg = build_job_config(spec, next(iter(spec.jobs())))
    assert len(cfg["agents"]) == 2
