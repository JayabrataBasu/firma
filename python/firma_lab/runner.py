"""firma_lab.runner -- expand an ``ExperimentSpec`` into concrete jobs,
execute each via the ``firma`` CLI binary as a subprocess, and persist a
real per-job manifest (manual Sec 23.1, Sec 22.3, Sec 30.6/Sec 30.7).

**Executes via subprocess, never re-implements orchestration.** This
module drives ``firma run --model`` as an external process for every job
-- it does not link `firma-kernel`/`firma-registry` (ADR 0045's Note is
still correct: an unused Rust dependency would have been designing for a
hypothetical), does not reconstruct phase scheduling, and does not
duplicate any domain formula. This mirrors ADR 0046's load/metrics split
one level up the stack: Python orchestrates and organises which jobs run
and in what order and records what happened; the Rust binary is the only
thing that ever computes a trajectory.

Every job's ``StreamSeeds`` comes from ``firma_lab.spec.derive_seeds``,
called exactly once by ``ExperimentSpec.jobs()`` -- this module never
re-derives or duplicates that mapping (see ``firma_lab.spec``'s own
docstring for why, and for the matched/varying-field design it
implements).
"""

from __future__ import annotations

import copy
import json
import subprocess
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterable, Sequence, Union

from firma_lab.spec import ExperimentSpec, Factor, Job

PathLike = Union[str, Path]

# ---------------------------------------------------------------------------
# Job-config synthesis
# ---------------------------------------------------------------------------


def _find_rule(cfg: dict[str, Any], rule_id: str) -> dict[str, Any]:
    for rule in cfg.get("rules", []):
        if rule.get("id") == rule_id:
            return rule  # type: ignore[no-any-return]
    raise KeyError(f"no rule with id={rule_id!r} in this job's config template")


def _apply_path(container: dict[str, Any], path: Sequence[str], value: Any) -> None:
    """Set ``container[path[0]][path[1]]...[path[-1]] = value``, creating
    intermediate dicts as needed. One ``"*"`` segment is allowed, meaning
    "every element of the list found at this point" (e.g. every configured
    shock) -- broadcasting ``value`` to the remaining sub-path on each list
    element. At most one wildcard is supported; a path needing more is not
    representable by this function (a real limitation, not silently
    worked around)."""
    if "*" in path:
        idx = path.index("*")
        prefix, suffix = list(path[:idx]), list(path[idx + 1 :])
        if not suffix:
            raise ValueError(f"path {path}: '*' cannot be the final segment")
        node: Any = container
        for key in prefix:
            node = node[key]
        if not isinstance(node, list) or len(node) == 0:
            raise TypeError(
                f"path {path}: expected a non-empty list at {prefix}, got {node!r}"
            )
        for item in node:
            _apply_path(item, suffix, value)
        return
    node = container
    for key in path[:-1]:
        node = node.setdefault(key, {})
    node[path[-1]] = value


def _add_pin_intervention(cfg: dict[str, Any], field: str, value: float) -> None:
    """ADR-0054: append one ``{"at": 0, "op": "set_agent_real", ...}``
    ``TimedIntervention`` per agent in ``cfg["agents"]``, targeting
    ``field``/``value`` -- the orchestration-level choice this module makes
    (not specified by ADR-0054 itself, which designed the kernel/domain
    mechanism, not "which agent(s) a pin applies to"): pin *every* agent
    the job's config defines, since Arm A's own template is expected to
    define exactly the firm(s) under test and nothing else. Ticks fire
    "before that tick's phases run" (`firma-cli::orchestrator::
    apply_interventions_at`, read directly) and this key is read-side
    persistent (ADR-0054 Q2) -- one entry at ``at: 0`` is sufficient for
    the whole run, not one per tick.
    """
    agents = cfg.get("agents", [])
    if not agents:
        raise ValueError(
            "direct_intervention factor requires at least one agent in "
            "the config template's \"agents\" list to target"
        )
    interventions = cfg.setdefault("interventions", [])
    for a in agents:
        interventions.append(
            {"at": 0, "op": "set_agent_real", "agent": a["id"], "field": field, "value": value}
        )


def _apply_factor(cfg: dict[str, Any], factor: Factor, level: Any) -> None:
    """Patch one factor's chosen level into a job's resolved config, per its
    ``apply_kind`` (see ``firma_lab.spec``'s ``Factor`` docstring for the
    four kinds)."""
    if factor.apply_kind == "top_level":
        assert factor.json_path is not None  # enforced by Factor.__post_init__
        _apply_path(cfg, factor.json_path, level)
    elif factor.apply_kind == "rule_param":
        assert factor.rule_id is not None and factor.json_path is not None
        rule = _find_rule(cfg, factor.rule_id)
        params = rule.setdefault("params", {})
        _apply_path(params, factor.json_path, level)
    elif factor.apply_kind == "rule_swap":
        assert factor.rule_category is not None
        rules = cfg.setdefault("rules", [])
        for i, rule in enumerate(rules):
            if str(rule.get("id", "")).startswith(factor.rule_category):
                rules[i] = json.loads(json.dumps(level))  # deep copy of the level
                return
        raise KeyError(
            f"factor {factor.name!r}: no existing rule with id starting "
            f"{factor.rule_category!r} to swap in the config template"
        )
    elif factor.apply_kind == "direct_intervention":
        assert factor.json_path is not None and len(factor.json_path) == 1
        field = factor.json_path[0]
        _add_pin_intervention(cfg, field, float(level))
        for companion_field, companion_value in factor.companions:
            _add_pin_intervention(cfg, companion_field, float(companion_value))
    else:  # pragma: no cover -- Factor.__post_init__ already restricts this
        raise AssertionError(f"unhandled apply_kind {factor.apply_kind!r}")


def build_job_config(spec: ExperimentSpec, job: Job) -> dict[str, Any]:
    """The concrete, resolved ``firma run --model`` config JSON for one
    ``Job``: a deep copy of ``spec.model_config_template`` with ``seeds``
    set from ``job.seeds`` and every one of the job's factor levels patched
    in per its ``Factor.apply_kind`` (ADR-0054's ``direct_intervention``
    included, as of that ADR's implementation)."""
    cfg = copy.deepcopy(dict(spec.model_config_template))
    cfg["seeds"] = job.seeds.to_dict()
    arm = spec.arm(job.arm)
    factors_by_name = {f.name: f for f in arm.factors}

    # Ordering matters and is deliberate, not incidental: `rule_swap`
    # factors (which rule id is even present) are applied *before*
    # `rule_param`/`top_level` factors (which patch a value *into* a
    # specific rule). Applying a rule_param first and letting a later
    # rule_swap silently overwrite that rule would drop the patch with no
    # error -- found exactly this bug while building this function against
    # the real narrowed-E1 spec (Arm C's `beta` factor targets
    # `decision.satisficing`, which a same-cell `decision_plugin=random`
    # swap removes; `decision.random`'s own `RandomParams` has no `beta`
    # field at all, `#[serde(deny_unknown_fields)]`-enforced --
    # crates/firma-plugins/firma-plugin-decision/src/lib.rs, read
    # directly). Swap-first makes that case fail loudly (`_find_rule`
    # raises `KeyError`) instead of silently dropping the patch -- see
    # this instruction's report for why Arm C's own factor-crossing is
    # itself ambiguous here, not just this function's bug.
    ordered = sorted(
        job.factor_levels.items(),
        key=lambda kv: 0 if factors_by_name[kv[0]].apply_kind == "rule_swap" else 1,
    )
    for factor_name, level in ordered:
        _apply_factor(cfg, factors_by_name[factor_name], level)
    return cfg


def job_id(spec: ExperimentSpec, job: Job) -> str:
    """A stable, filesystem-safe identifier for one job, used as its output
    directory name and as the resumability key. Deterministic in
    ``(spec.content_hash(), job.arm, job.cell_index, job.replicate)`` --
    the same job, re-expanded from the same spec, always maps to the same
    id, which is what makes resuming a partial run correct."""
    return f"{spec.content_hash()[:12]}-{job.arm}-c{job.cell_index:04d}-r{job.replicate:04d}"


# ---------------------------------------------------------------------------
# Execution
# ---------------------------------------------------------------------------


@dataclass
class JobRecord:
    """The real per-job manifest this instruction asked for: everything
    needed to independently verify or re-audit one job later, without
    rerunning the experiment. Written to
    ``<job_dir>/firma_lab_job.json`` after every attempt (success or
    failure) -- this is the resumability ledger, read back by
    ``run_experiment`` to skip already-completed jobs."""

    job_id: str
    spec_id: str
    spec_content_hash: str
    arm: str
    cell_index: int
    factor_levels: dict[str, Any]
    replicate: int
    seeds: dict[str, int]
    resolved_config: dict[str, Any]
    status: str  # "pending" | "completed" | "failed"
    run_id: str | None = None
    event_log_sha256: str | None = None
    conservation_ok: bool | None = None
    failure_reason: str | None = None
    started_at: float | None = None
    finished_at: float | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "job_id": self.job_id,
            "spec_id": self.spec_id,
            "spec_content_hash": self.spec_content_hash,
            "arm": self.arm,
            "cell_index": self.cell_index,
            "factor_levels": self.factor_levels,
            "replicate": self.replicate,
            "seeds": self.seeds,
            "resolved_config": self.resolved_config,
            "status": self.status,
            "run_id": self.run_id,
            "event_log_sha256": self.event_log_sha256,
            "conservation_ok": self.conservation_ok,
            "failure_reason": self.failure_reason,
            "started_at": self.started_at,
            "finished_at": self.finished_at,
        }

    def write(self, job_dir: Path) -> None:
        job_dir.mkdir(parents=True, exist_ok=True)
        (job_dir / "firma_lab_job.json").write_text(
            json.dumps(self.to_dict(), indent=2, sort_keys=True)
        )

    @staticmethod
    def read(job_dir: Path) -> "JobRecord | None":
        path = job_dir / "firma_lab_job.json"
        if not path.exists():
            return None
        data = json.loads(path.read_text())
        return JobRecord(**data)


def _run_one_job(
    spec: ExperimentSpec, job: Job, firma_bin: Path, jobs_root: Path
) -> JobRecord:
    """Execute exactly one job: build its config, invoke ``firma run
    --model`` as a subprocess, parse its output, and return the resulting
    ``JobRecord`` (never raises for an ordinary run failure -- a failed
    job is a *recorded outcome*, per manual Sec 30.7's "runs failing an
    engine invariant are excluded with counts and reasons reported", not
    an exception that would abort the rest of the experiment).

    **Determinism under concurrency, stated explicitly, not merely
    assumed:** each call to this function is a fully independent OS
    process invocation of ``firma_bin`` against a config file unique to
    this job, writing to a directory unique to this job. No two jobs share
    any mutable state (no shared RNG, no shared file beyond each job's own
    directory), so running many jobs concurrently (``run_experiment``'s
    ``max_workers``) cannot change any individual job's own
    ``event_log_sha256`` relative to running it alone -- concurrency only
    changes wall-clock scheduling, never a job's own inputs or outputs.
    """
    jid = job_id(spec, job)
    job_dir = jobs_root / jid
    out_dir = job_dir / "run"
    cfg = build_job_config(spec, job)
    started = time.time()
    record = JobRecord(
        job_id=jid,
        spec_id=spec.id,
        spec_content_hash=spec.content_hash(),
        arm=job.arm,
        cell_index=job.cell_index,
        factor_levels=dict(job.factor_levels),
        replicate=job.replicate,
        seeds=job.seeds.to_dict(),
        resolved_config=cfg,
        status="pending",
        started_at=started,
    )
    job_dir.mkdir(parents=True, exist_ok=True)
    config_path = job_dir / "config.json"
    config_path.write_text(json.dumps(cfg, indent=2))

    try:
        proc = subprocess.run(
            [
                str(firma_bin),
                "run",
                "--model",
                "--config",
                str(config_path),
                "--out",
                str(out_dir),
            ],
            capture_output=True,
            text=True,
            timeout=600,
        )
    except subprocess.TimeoutExpired as e:
        record.status = "failed"
        record.failure_reason = f"timed out after {e.timeout}s"
        record.finished_at = time.time()
        record.write(job_dir)
        return record
    except OSError as e:
        record.status = "failed"
        record.failure_reason = f"could not launch subprocess: {e}"
        record.finished_at = time.time()
        record.write(job_dir)
        return record

    record.finished_at = time.time()
    if proc.returncode != 0:
        record.status = "failed"
        record.failure_reason = (
            f"firma run exited {proc.returncode}: "
            f"{proc.stderr.strip()[-2000:] or proc.stdout.strip()[-2000:]}"
        )
        record.write(job_dir)
        return record

    parsed: dict[str, str] = {}
    for line in proc.stdout.splitlines():
        parts = line.split(None, 1)
        if len(parts) == 2:
            parsed[parts[0]] = parts[1].strip()

    record.run_id = parsed.get("run_id")
    record.event_log_sha256 = parsed.get("event_log_sha256")
    conservation_raw = parsed.get("conservation_ok")
    record.conservation_ok = conservation_raw == "true" if conservation_raw else None

    if record.conservation_ok is False:
        record.status = "failed"
        record.failure_reason = "conservation_ok=false (an engine invariant failed)"
    elif not record.run_id or not record.event_log_sha256:
        record.status = "failed"
        record.failure_reason = (
            f"firma run exited 0 but stdout was unparsable: {proc.stdout[:2000]!r}"
        )
    else:
        record.status = "completed"

    record.write(job_dir)
    return record


@dataclass
class ExperimentResult:
    spec_id: str
    spec_content_hash: str
    total_jobs: int
    completed: list[JobRecord] = field(default_factory=list)
    failed: list[JobRecord] = field(default_factory=list)
    skipped_already_done: int = 0

    def summary(self) -> dict[str, Any]:
        return {
            "spec_id": self.spec_id,
            "spec_content_hash": self.spec_content_hash,
            "total_jobs": self.total_jobs,
            "completed": len(self.completed),
            "failed": len(self.failed),
            "skipped_already_done": self.skipped_already_done,
            "failures": [
                {"job_id": r.job_id, "reason": r.failure_reason} for r in self.failed
            ],
        }

    def write_summary(self, out_dir: Path) -> None:
        out_dir.mkdir(parents=True, exist_ok=True)
        (out_dir / "experiment_summary.json").write_text(
            json.dumps(self.summary(), indent=2, sort_keys=True)
        )


def run_experiment(
    spec: ExperimentSpec,
    out_dir: PathLike,
    firma_bin: PathLike,
    *,
    max_workers: int = 4,
    jobs: Iterable[Job] | None = None,
) -> ExperimentResult:
    """Expand ``spec`` (or a caller-supplied subset via ``jobs``, used by
    the resumability test) into jobs and execute each via ``firma_bin``,
    up to ``max_workers`` concurrently.

    **Resumable:** before launching a job, its ``job_dir`` is checked for
    an existing ``firma_lab_job.json`` with ``status == "completed"`` --
    if found, the job is skipped (counted in
    ``ExperimentResult.skipped_already_done``), not re-run and not
    double-counted. A job previously recorded as ``"failed"`` **is**
    retried (failures are not retried *indefinitely* within one call --
    each call attempts every not-yet-completed job exactly once -- but a
    fresh call after a partial run will retry previously-failed jobs
    rather than leaving them permanently failed, since the underlying
    cause, e.g. a transient resource issue, may no longer apply).

    Every job's own directory and manifest is independent of every other
    job's, so a crash or interruption mid-run loses at most the
    in-flight jobs, never previously-completed ones' records.
    """
    out = Path(out_dir)
    jobs_root = out / "jobs"
    jobs_root.mkdir(parents=True, exist_ok=True)
    firma_bin = Path(firma_bin)
    if not firma_bin.exists():
        raise FileNotFoundError(f"firma binary not found at {firma_bin}")

    job_list = list(jobs) if jobs is not None else list(spec.jobs())
    result = ExperimentResult(
        spec_id=spec.id, spec_content_hash=spec.content_hash(), total_jobs=len(job_list)
    )

    to_run: list[Job] = []
    for job in job_list:
        jid = job_id(spec, job)
        existing = JobRecord.read(jobs_root / jid)
        if existing is not None and existing.status == "completed":
            result.completed.append(existing)
            result.skipped_already_done += 1
        else:
            to_run.append(job)

    with ThreadPoolExecutor(max_workers=max_workers) as pool:
        futures = {
            pool.submit(_run_one_job, spec, job, firma_bin, jobs_root): job
            for job in to_run
        }
        for fut in as_completed(futures):
            record = fut.result()
            if record.status == "completed":
                result.completed.append(record)
            else:
                result.failed.append(record)

    result.write_summary(out)
    return result
