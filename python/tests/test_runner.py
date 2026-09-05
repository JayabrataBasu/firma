"""firma_lab.runner -- end-to-end pipeline tests against a small synthetic
spec, run through the real ``firma`` CLI binary as a subprocess (this
instruction's Part C3).

**Does not construct or execute the real, full 195-cell/39,000-run E1
design** (this instruction's explicit hard stop) -- every test here uses
``firma_lab.spec.build_synthetic_spec`` (a handful of jobs) or a
deliberately tiny one-off spec built inline for the failure-handling test.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from firma_lab.spec import ArmDesign, ExperimentSpec, Factor, Hypothesis, build_synthetic_spec
from firma_lab.runner import JobRecord, build_job_config, job_id, run_experiment

REPO_ROOT = Path(__file__).resolve().parents[2]
FIRMA_BIN = REPO_ROOT / "target" / "debug" / "firma"

pytestmark = pytest.mark.skipif(
    not FIRMA_BIN.exists(), reason=f"{FIRMA_BIN} not built -- run `cargo build` first"
)


def _failing_spec() -> ExperimentSpec:
    """One job that will succeed (l_w=8) and one that will fail
    (l_w=0 -- SatisficingParams.validate() rejects "l_w must be >= 1",
    firma-plugin-decision/src/lib.rs), for testing failure recording."""
    template = {
        "experiment": "synthetic-failure-test",
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
        "rules": [{"id": "decision.satisficing", "version": "^1", "params": {"l_w": 8}}],
    }
    arm = ArmDesign(
        "T",
        (
            Factor(
                "l_w",
                (8, 0),  # 8 succeeds, 0 is invalid
                arm="T",
                apply_kind="rule_param",
                rule_id="decision.satisficing",
                json_path=("l_w",),
            ),
        ),
    )
    return ExperimentSpec(
        id="synthetic-failure-test",
        schema_version="1.0.0",
        engine_requirement=">=0.1.0, <0.2.0",
        question_id="pipeline-self-test",
        hypotheses=(Hypothesis("HT", "synthetic", "n/a", "T", "n/a"),),
        arms=(arm,),
        model_config_template=template,
        num_replicates=1,
        interventions=(),
        observation=("n/a",),
        analysis_plan={"note": "pipeline failure-handling self-test"},
        stopping_rules="All jobs execute to completion.",
        alternative_explanation="n/a -- synthetic pipeline test.",
    )


# ---------------------------------------------------------------------------
# build_job_config
# ---------------------------------------------------------------------------


def test_build_job_config_patches_seeds_and_factors() -> None:
    spec = build_synthetic_spec()
    job = next(iter(spec.jobs()))
    cfg = build_job_config(spec, job)
    assert cfg["seeds"] == job.seeds.to_dict()
    assert cfg["world"]["ticks"] == job.factor_levels["ticks"]
    rule = next(r for r in cfg["rules"] if r["id"] == "decision.satisficing")
    assert rule["params"]["l_w"] == job.factor_levels["l_w"]


def test_build_job_config_does_not_mutate_template() -> None:
    spec = build_synthetic_spec()
    template_before = json.dumps(spec.model_config_template, sort_keys=True)
    for job in spec.jobs():
        build_job_config(spec, job)
    assert json.dumps(spec.model_config_template, sort_keys=True) == template_before


def test_job_id_deterministic_and_distinct_across_jobs() -> None:
    spec = build_synthetic_spec(num_replicates=2)
    jobs = list(spec.jobs())
    ids = [job_id(spec, j) for j in jobs]
    assert len(ids) == len(set(ids))
    assert job_id(spec, jobs[0]) == job_id(spec, jobs[0])


# ---------------------------------------------------------------------------
# End-to-end pipeline
# ---------------------------------------------------------------------------


def test_pipeline_end_to_end_synthetic_spec(tmp_path: Path) -> None:
    spec = build_synthetic_spec(num_replicates=3)
    result = run_experiment(spec, tmp_path, FIRMA_BIN, max_workers=4)

    assert result.total_jobs == spec.total_jobs() == 12
    assert len(result.completed) == 12
    assert len(result.failed) == 0
    assert result.skipped_already_done == 0

    # Every completed job has a real run_id/event_log_sha256 and a manifest
    # on disk (this instruction's "persist a real run manifest per job").
    for record in result.completed:
        assert record.run_id and len(record.run_id) > 0
        assert record.event_log_sha256 and len(record.event_log_sha256) == 64
        assert record.conservation_ok is True
        job_dir = tmp_path / "jobs" / record.job_id
        assert (job_dir / "firma_lab_job.json").exists()
        assert (job_dir / "config.json").exists()
        assert (job_dir / "run" / "manifest.json").exists()
        assert (job_dir / "run" / "events.ndjson").exists()

    # Summary file written.
    summary_path = tmp_path / "experiment_summary.json"
    assert summary_path.exists()
    summary = json.loads(summary_path.read_text())
    assert summary["completed"] == 12
    assert summary["failed"] == 0


def test_pipeline_job_records_are_independently_readable(tmp_path: Path) -> None:
    """JobRecord.read reconstructs the same record a fresh process would see
    -- the resumability mechanism's own read path, tested directly."""
    spec = build_synthetic_spec(num_replicates=1)
    result = run_experiment(spec, tmp_path, FIRMA_BIN, max_workers=2)
    for record in result.completed:
        reread = JobRecord.read(tmp_path / "jobs" / record.job_id)
        assert reread is not None
        assert reread.status == "completed"
        assert reread.run_id == record.run_id
        assert reread.event_log_sha256 == record.event_log_sha256


# ---------------------------------------------------------------------------
# Resumability
# ---------------------------------------------------------------------------


def test_resume_skips_already_completed_jobs(tmp_path: Path) -> None:
    spec = build_synthetic_spec(num_replicates=2)
    first = run_experiment(spec, tmp_path, FIRMA_BIN, max_workers=4)
    assert len(first.completed) == spec.total_jobs()
    assert first.skipped_already_done == 0

    first_hashes = {r.job_id: r.event_log_sha256 for r in first.completed}
    first_finished_at = {r.job_id: r.finished_at for r in first.completed}

    second = run_experiment(spec, tmp_path, FIRMA_BIN, max_workers=4)
    assert second.skipped_already_done == spec.total_jobs()
    assert len(second.failed) == 0
    # Skipped jobs are not re-run: same event_log_sha256, same finished_at
    # timestamp (a re-run would produce a new, later timestamp).
    second_hashes = {r.job_id: r.event_log_sha256 for r in second.completed}
    second_finished_at = {r.job_id: r.finished_at for r in second.completed}
    assert second_hashes == first_hashes
    assert second_finished_at == first_finished_at


def test_resume_after_partial_run_only_runs_remaining_jobs(tmp_path: Path) -> None:
    spec = build_synthetic_spec(num_replicates=2)
    all_jobs = list(spec.jobs())
    half = all_jobs[: len(all_jobs) // 2]

    first = run_experiment(spec, tmp_path, FIRMA_BIN, max_workers=2, jobs=half)
    assert len(first.completed) == len(half)
    assert first.total_jobs == len(half)

    # Now request the full spec's jobs; only the other half should actually
    # execute (skipped_already_done == the first half's count).
    second = run_experiment(spec, tmp_path, FIRMA_BIN, max_workers=2)
    assert second.skipped_already_done == len(half)
    assert len(second.completed) == len(all_jobs)


# ---------------------------------------------------------------------------
# Failure handling
# ---------------------------------------------------------------------------


def test_failure_is_recorded_not_raised_and_does_not_stop_other_jobs(
    tmp_path: Path,
) -> None:
    spec = _failing_spec()
    result = run_experiment(spec, tmp_path, FIRMA_BIN, max_workers=2)

    assert result.total_jobs == 2
    assert len(result.completed) == 1
    assert len(result.failed) == 1

    failed = result.failed[0]
    assert failed.status == "failed"
    assert failed.failure_reason is not None
    assert "l_w must be >= 1" in failed.failure_reason
    assert failed.run_id is None
    assert failed.event_log_sha256 is None

    # The failure is persisted, independently readable, and the summary
    # reports it with a reason (manual Sec 30.7's "excluded with counts and
    # reasons reported").
    reread = JobRecord.read(tmp_path / "jobs" / failed.job_id)
    assert reread is not None
    assert reread.status == "failed"
    summary = json.loads((tmp_path / "experiment_summary.json").read_text())
    assert summary["failed"] == 1
    assert summary["failures"][0]["job_id"] == failed.job_id
    assert "l_w must be >= 1" in summary["failures"][0]["reason"]


def test_failed_job_is_retried_on_a_fresh_call(tmp_path: Path) -> None:
    """A previously-failed job is retried on the next call (not left
    permanently failed) -- distinct from a *completed* job, which is
    skipped."""
    spec = _failing_spec()
    first = run_experiment(spec, tmp_path, FIRMA_BIN, max_workers=2)
    assert len(first.failed) == 1

    second = run_experiment(spec, tmp_path, FIRMA_BIN, max_workers=2)
    # The failing job is attempted again (not skipped) and fails again,
    # deterministically -- not silently skipped as "already done".
    assert second.skipped_already_done == 1  # only the succeeding job
    assert len(second.failed) == 1
    assert len(second.completed) == 1


# ---------------------------------------------------------------------------
# Determinism under concurrency
# ---------------------------------------------------------------------------


def test_concurrent_execution_does_not_change_any_jobs_event_log_hash(
    tmp_path: Path,
) -> None:
    """This instruction's explicit requirement: confirm, not merely assume,
    that concurrent execution does not affect any individual job's own
    event_log_sha256. Runs the same spec twice, into two separate output
    directories, once sequentially (max_workers=1) and once with real
    concurrency (max_workers=8), and compares every job's hash."""
    spec = build_synthetic_spec(num_replicates=3)
    seq_dir = tmp_path / "seq"
    par_dir = tmp_path / "par"

    seq = run_experiment(spec, seq_dir, FIRMA_BIN, max_workers=1)
    par = run_experiment(spec, par_dir, FIRMA_BIN, max_workers=8)

    seq_hashes = {r.job_id: r.event_log_sha256 for r in seq.completed}
    par_hashes = {r.job_id: r.event_log_sha256 for r in par.completed}
    assert len(seq_hashes) == len(par_hashes) == spec.total_jobs()
    assert seq_hashes == par_hashes


# ---------------------------------------------------------------------------
# Missing binary
# ---------------------------------------------------------------------------


def test_run_experiment_raises_clearly_if_binary_missing(tmp_path: Path) -> None:
    spec = build_synthetic_spec(num_replicates=1)
    with pytest.raises(FileNotFoundError):
        run_experiment(spec, tmp_path, tmp_path / "no-such-binary", max_workers=1)
