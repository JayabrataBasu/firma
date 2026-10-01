"""firma_lab.prereg -- check manual Sec 30.9's filing conditions mechanically
and emit a pre-registration document from an ``ExperimentSpec`` (Sec 23.1).

Each Sec 30.9 bullet becomes a ``ChecklistItem`` with one of four statuses
and the raw evidence it rests on:

- ``PASS`` / ``FAIL`` -- decided mechanically from the evidence shown;
- ``NEEDS_OWNER`` -- the evidence is gathered, but the bullet needs a
  judgement this module must not make (e.g. whether SC-1...SC-6 hold "for
  every hypothesis and Arm actually registered", ADR-0044/ADR-0051);
- ``NOT_EVALUATED`` -- the evidence was not supplied.

Nothing here runs the registered design. The only runs it inspects are ones
already on disk, and the test suites it parses are passed in as captured
``cargo test`` output (``run_cargo_test`` captures it).

**Sec 30.9's seed-range bullet (ADR-0053).** The manual's literal text is
"Seed range 1-200 confirmed unused by any exploratory Phase 2 run".
ADR-0053 (Accepted) reasons, per Sec 30.6, that a run touches the
registered grid iff its **resolved config, seeds included, equals a
registered job's config**, and makes that conclusion conditional on a check
built here: ``check_seed_range`` compares every exploratory run config found
on disk against every registered job config with the same seeds, after
parsing both with the engine's own ``RunConfig`` parser
(``firma_lab._native.config_identity_hash`` -- Python never re-implements
serde defaults). Runs whose seeds fall in the registered range but whose
config differs, and seed literals in source files, are listed as evidence.
Plugin ``params`` are compared as written (see the native function's
docstring), so a config that spells out a plugin default is not recognised
as equal to one that omits it; this limitation is stated in the evidence.
"""

from __future__ import annotations

import hashlib
import json
import re
import subprocess
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterable, Iterator, Mapping, Sequence

from firma_lab.runner import build_job_config
from firma_lab.spec import ExperimentSpec, derive_seeds

PASS, FAIL, NEEDS_OWNER, NOT_EVALUATED = "PASS", "FAIL", "NEEDS_OWNER", "NOT_EVALUATED"

#: The Sec 30.9 bullets, verbatim from manual v1.1.0.
SECTION_30_9 = {
    "validation": "All §25.4 validation tests pass, **including VT-8**, with evidence attached.",
    "sanity": (
        "Sanity conditions SC-1…SC-6 satisfied for every hypothesis and Arm actually "
        "registered in this filing (§16.2). A hypothesis or design factor explicitly "
        "excluded from a given filing's registered design (see e.g. ADR-0051) is not "
        "required to clear a sanity condition it would otherwise need."
    ),
    "artefacts": "AT-1…AT-5 run and reported.",
    "determinism": "All DT tests green.",
    "seed_range": "Seed range 1–200 confirmed unused by any exploratory Phase 2 run.",
    "hashes": "All attached documents content-hashed and hashes recorded.",
}


@dataclass(frozen=True)
class ChecklistItem:
    key: str
    text: str
    status: str
    evidence: tuple[str, ...] = field(default_factory=tuple)


# ---------------------------------------------------------------------------
# cargo test output
# ---------------------------------------------------------------------------

_TEST_LINE = re.compile(r"test (\S+) \.\.\. (ok|FAILED|ignored)")


def parse_cargo_test_output(text: str) -> dict[str, str]:
    """``{test_name: "ok" | "FAILED" | "ignored"}`` from ``cargo test``
    output. Matches anywhere in a line, because ``--nocapture`` output from
    parallel tests can share a line with a result."""
    return {m.group(1): m.group(2) for m in _TEST_LINE.finditer(text)}


def run_cargo_test(repo_root: Path, test_target: str) -> str:
    """Capture ``cargo test -p firma-conformance --test <target>`` output
    (stdout + stderr) for the checks below. Not called by this module's own
    unit tests."""
    proc = subprocess.run(
        ["cargo", "test", "-p", "firma-conformance", "--test", test_target, "--", "--nocapture"],
        cwd=repo_root, capture_output=True, text=True,
    )
    return proc.stdout + proc.stderr


def _sha(text: str | bytes) -> str:
    data = text.encode("utf-8") if isinstance(text, str) else text
    return hashlib.sha256(data).hexdigest()


def _numbered_suite(
    key: str, output: str | None, prefix: str, numbers: Sequence[int], label: str
) -> ChecklistItem:
    if output is None:
        return ChecklistItem(key, SECTION_30_9[key], NOT_EVALUATED, ("no test output supplied",))
    results = parse_cargo_test_output(output)
    evidence = [f"captured output sha256 {_sha(output)}"]
    evidence += [f"{name}: {res}" for name, res in sorted(results.items())]
    missing = [
        f"{label}-{n}" for n in numbers
        if not any(name.startswith(f"{prefix}{n}_") for name in results)
    ]
    failing = [n for n, r in results.items() if r == "FAILED"]
    not_ok = [
        name for name, r in results.items()
        if r != "ok" and any(name.startswith(f"{prefix}{n}_") for n in numbers)
    ]
    if missing:
        evidence.append(f"no test found for: {missing}")
    if failing:
        evidence.append(f"FAILED: {failing}")
    status = PASS if not (missing or failing or not_ok) else FAIL
    return ChecklistItem(key, SECTION_30_9[key], status, tuple(evidence))


def check_validation_tests(output: str | None) -> ChecklistItem:
    """PASS iff a ``vt<k>_*`` test exists and is ``ok`` for every k in 1..8
    (Sec 25.4) and nothing in the output FAILED."""
    return _numbered_suite("validation", output, "vt", range(1, 9), "VT")


def check_determinism_tests(output: str | None) -> ChecklistItem:
    """PASS iff a ``dt<k>_*`` test exists and is ``ok`` for every k in 1..6
    (Sec 25.2) and nothing in the output FAILED."""
    return _numbered_suite("determinism", output, "dt", range(1, 7), "DT")


def check_sanity(output: str | None) -> ChecklistItem:
    """Always ``NEEDS_OWNER``: whether SC-1...SC-6 hold for each registered
    hypothesis and arm is a scoping judgement (ADR-0043, ADR-0044, ADR-0051),
    not a test result. The evidence lists every sanity test's result and
    calls out any failure first."""
    if output is None:
        return ChecklistItem("sanity", SECTION_30_9["sanity"], NOT_EVALUATED, ("no test output supplied",))
    results = parse_cargo_test_output(output)
    failing = sorted(n for n, r in results.items() if r == "FAILED")
    evidence = [f"captured output sha256 {_sha(output)}"]
    if failing:
        evidence.insert(0, f"FAILING sanity test(s): {failing}")
    evidence += [f"{n}: {r}" for n, r in sorted(results.items())]
    evidence.append("scoping per ADR-0043 / ADR-0044 / ADR-0051 is the owner's call")
    return ChecklistItem("sanity", SECTION_30_9["sanity"], NEEDS_OWNER, tuple(evidence))


_AT_FN = re.compile(r"fn\s+(at[1-5])_\w*")


def check_artefact_suite(source_roots: Sequence[Path]) -> ChecklistItem:
    """FAIL if any of AT-1...AT-5 (Sec 25.5) has no ``at<k>_*`` test function
    under ``source_roots``; otherwise NEEDS_OWNER, since "reported" is not
    something a file scan can confirm."""
    found: dict[str, list[str]] = {}
    for root in source_roots:
        for path in sorted(Path(root).rglob("*.rs")):
            for m in _AT_FN.finditer(path.read_text(encoding="utf-8", errors="replace")):
                found.setdefault(m.group(1), []).append(str(path))
    missing = [f"AT-{k}" for k in range(1, 6) if f"at{k}" not in found]
    evidence = [f"{k.upper()}: {v}" for k, v in sorted(found.items())]
    if missing:
        evidence.append(f"no test function found for: {missing}")
        return ChecklistItem("artefacts", SECTION_30_9["artefacts"], FAIL, tuple(evidence))
    return ChecklistItem("artefacts", SECTION_30_9["artefacts"], NEEDS_OWNER, tuple(evidence))


def check_attachments(paths: Sequence[Path], spec: ExperimentSpec) -> ChecklistItem:
    """SHA-256 of every attached document, plus the spec's own content hash.
    FAIL if nothing is attached or a path does not exist."""
    evidence = [f"ExperimentSpec {spec.id!r} content hash {spec.content_hash()}"]
    missing = [str(p) for p in paths if not Path(p).is_file()]
    for p in paths:
        if Path(p).is_file():
            evidence.append(f"{p}: sha256 {_sha(Path(p).read_bytes())}")
    if missing:
        evidence.append(f"missing: {missing}")
    status = PASS if paths and not missing else FAIL
    if not paths:
        evidence.append("no documents attached")
    return ChecklistItem("hashes", SECTION_30_9["hashes"], status, tuple(evidence))


# ---------------------------------------------------------------------------
# Seed range / template identity (ADR-0053)
# ---------------------------------------------------------------------------


def scan_run_configs(roots: Sequence[Path]) -> Iterator[tuple[str, dict[str, Any]]]:
    """Every run config found under ``roots``: a run manifest's
    ``identity.resolved_config``, a ``firma_lab`` job record's
    ``resolved_config``, or a bare config JSON (has ``seeds``, ``world`` and
    ``rules``). Yields ``(path, config)`` in sorted path order."""
    for root in roots:
        for path in sorted(Path(root).rglob("*.json")):
            try:
                d = json.loads(path.read_text(encoding="utf-8"))
            except (OSError, ValueError):
                continue
            if not isinstance(d, dict):
                continue
            if isinstance(d.get("identity"), dict) and "resolved_config" in d["identity"]:
                yield str(path), d["identity"]["resolved_config"]
            elif isinstance(d.get("resolved_config"), dict):
                yield str(path), d["resolved_config"]
            elif {"seeds", "world", "rules"} <= set(d):
                yield str(path), d


_SEED_PATTERNS = [
    re.compile(
        r'"mechanism"\s*:\s*(\d+)\s*,\s*"environment"\s*:\s*(\d+)\s*,\s*'
        r'"shock"\s*:\s*(\d+)\s*,\s*"init"\s*:\s*(\d+)'
    ),
    re.compile(
        r"StreamSeeds\s*\{\s*mechanism\s*:\s*(\d+)\s*,\s*environment\s*:\s*(\d+)\s*,\s*"
        r"shock\s*:\s*(\d+)\s*,\s*init\s*:\s*(\d+)"
    ),
    re.compile(r"StreamSeeds\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)"),
]
_DERIVE = re.compile(r"derive_seeds\(\s*(\d+)\s*\)")


def scan_seed_literals(
    roots: Sequence[Path], suffixes: Sequence[str] = (".rs", ".py", ".json")
) -> list[tuple[str, int, tuple[int, int, int, int]]]:
    """Every literal seed tuple in source files under ``roots``, as
    ``(path, line, (mechanism, environment, shock, init))``. Recognised
    forms: JSON/format-string ``"mechanism": a, "environment": b, "shock": c,
    "init": d``; Rust ``StreamSeeds { mechanism: a, ... }``; Python
    ``StreamSeeds(a, b, c, d)``; and ``derive_seeds(n)`` (= ``(n,n,n,n)``,
    ADR-0053). A heuristic scan: other spellings are not seen."""
    out = []
    for root in roots:
        for path in sorted(Path(root).rglob("*")):
            if path.suffix not in suffixes or not path.is_file():
                continue
            text = path.read_text(encoding="utf-8", errors="replace")
            for pat in _SEED_PATTERNS:
                for m in pat.finditer(text):
                    line = text.count("\n", 0, m.start()) + 1
                    out.append((str(path), line, tuple(int(g) for g in m.groups())))
            for m in _DERIVE.finditer(text):
                n = int(m.group(1))
                out.append((str(path), text.count("\n", 0, m.start()) + 1, (n, n, n, n)))
    return sorted(set(out))


def _seed_tuple(cfg: Mapping[str, Any]) -> tuple[int, ...] | None:
    s = cfg.get("seeds")
    if not isinstance(s, Mapping):
        return None
    try:
        return tuple(int(s[k]) for k in ("mechanism", "environment", "shock", "init"))
    except (KeyError, TypeError, ValueError):
        return None


def _identity_hash(cfg: Mapping[str, Any]) -> str:
    from firma_lab import _native

    return _native.config_identity_hash(json.dumps(cfg))


def check_seed_range(
    spec: ExperimentSpec,
    exploratory_configs: Iterable[tuple[str, Mapping[str, Any]]],
    seed_literals: Sequence[tuple[str, int, tuple[int, int, int, int]]],
    *,
    acknowledged: Iterable[tuple[str, tuple[int, int, int, int]]] = (),
) -> ChecklistItem:
    """The ADR-0053 template-identity check (see module docstring).

    - ``FAIL`` if any exploratory config equals a registered job config, or
      a registered job config cannot even be built (the check would be
      incomplete);
    - ``NEEDS_OWNER`` if an exploratory config cannot be parsed, or a source
      seed literal equals a registered seed tuple and is not in
      ``acknowledged`` (``(path, tuple)`` pairs already reasoned about, e.g.
      in ADR-0053 Part 4);
    - ``PASS`` otherwise -- under ADR-0053's criterion. Any seed overlap
      that is not a config match is still listed, since the manual's literal
      "seed range unused" is then not met.
    """
    order = ("mechanism", "environment", "shock", "init")
    rep_of = {
        tuple(derive_seeds(n).to_dict()[k] for k in order): n
        for n in range(1, spec.num_replicates + 1)
    }
    registered_tuples = set(rep_of)
    jobs_by_rep: dict[int, list] = {}
    for job in spec.jobs():
        jobs_by_rep.setdefault(job.replicate, []).append(job)

    reg_hashes: dict[int, dict[str, str]] = {}
    build_errors: list[str] = []

    def hashes_for(rep: int) -> dict[str, str]:
        if rep not in reg_hashes:
            hs = {}
            for job in jobs_by_rep.get(rep, []):
                label = f"{job.arm}/cell{job.cell_index}/rep{rep}"
                try:
                    hs[_identity_hash(build_job_config(spec, job))] = label
                except Exception as e:  # noqa: BLE001 -- recorded as a FAIL below
                    build_errors.append(f"{label}: {type(e).__name__}: {e}")
            reg_hashes[rep] = hs
        return reg_hashes[rep]

    collisions, overlaps, unparsable = [], [], []
    n_scanned = 0
    for source, cfg in exploratory_configs:
        n_scanned += 1
        t = _seed_tuple(cfg)
        if t is None or t not in registered_tuples:
            continue
        try:
            h = _identity_hash(cfg)
        except Exception as e:  # noqa: BLE001
            unparsable.append(f"{source}: {e}")
            continue
        match = hashes_for(rep_of[t]).get(h)
        if match:
            collisions.append(f"{source} == registered job {match}")
        else:
            overlaps.append(f"{source}: seeds {t} in registered range, config differs")

    ack = {(p, tuple(t)) for p, t in acknowledged}
    literal_hits = [(p, ln, t) for p, ln, t in seed_literals if t in registered_tuples]
    unacknowledged = [(p, ln, t) for p, ln, t in literal_hits if (p, t) not in ack]

    evidence = [
        f"registered seeds: derive_seeds(n) for n in 1..{spec.num_replicates} (ADR-0053)",
        f"exploratory run configs scanned: {n_scanned}",
        "equality = same engine RunConfig after parsing; plugin params compared as written",
    ]
    evidence += [f"IDENTITY COLLISION: {c}" for c in collisions]
    evidence += [f"cannot build registered job config: {e}" for e in build_errors]
    evidence += [f"unparsable exploratory config: {u}" for u in unparsable]
    evidence += [f"seed overlap only: {o}" for o in overlaps]
    evidence += [f"source seed literal (acknowledged): {p}:{ln} {t}" for p, ln, t in literal_hits if (p, t) in ack]
    evidence += [f"source seed literal (NOT acknowledged): {p}:{ln} {t}" for p, ln, t in unacknowledged]

    if collisions or build_errors:
        status = FAIL
    elif unparsable or unacknowledged:
        status = NEEDS_OWNER
    else:
        status = PASS
    return ChecklistItem("seed_range", SECTION_30_9["seed_range"], status, tuple(evidence))


# ---------------------------------------------------------------------------
# Document
# ---------------------------------------------------------------------------


def render_prereg(spec: ExperimentSpec, checklist: Sequence[ChecklistItem]) -> str:
    """A Sec 30-shaped Markdown document for ``spec``: identity and hash,
    hypotheses with predicted signs, the design (factor levels and cell
    counts computed from the spec, not typed in), seeds, analysis plan,
    stopping rules, alternative explanation, and the Sec 30.9 checklist with
    evidence. Filing is the owner's act; this only drafts the document."""
    lines = [
        f"# Pre-registration: {spec.id}",
        "",
        f"- ExperimentSpec content hash: `{spec.content_hash()}`",
        f"- Question: `{spec.question_id}` · engine `{spec.engine_requirement}` · schema `{spec.schema_version}`",
        "",
        "## Hypotheses",
        "",
        "| ID | Hypothesis | Predicted sign | Arm | Primary DV |",
        "|---|---|---|---|---|",
    ]
    lines += [f"| {h.id} | {h.text} | {h.predicted_sign} | {h.arm} | `{h.primary_dv}` |" for h in spec.hypotheses]
    lines += ["", "## Design", "", "| Arm | Factor | Levels | Cells |", "|---|---|---|---|"]
    for a in spec.arms:
        for i, f in enumerate(a.factors):
            levels = ", ".join(
                str(v["id"]) if isinstance(v, Mapping) and "id" in v else str(v) for v in f.levels
            )
            lines.append(f"| {a.name if i == 0 else ''} | {f.name} | {levels} | {a.cell_count if i == 0 else ''} |")
    lines += [
        "",
        f"Total: **{spec.total_cells()} cells × {spec.num_replicates} replicates = {spec.total_jobs()} runs.**",
        "",
        "## Seeds",
        "",
        f"Replicate *n* runs with all four streams seeded *n* (ADR-0053), *n* = 1…{spec.num_replicates}; "
        "the same seeds at every cell (common random numbers).",
        "",
        "## Analysis plan",
        "",
    ]
    lines += [f"- **{k}:** {v}" for k, v in spec.analysis_plan.items()]
    lines += [
        "",
        "## Stopping rules",
        "",
        spec.stopping_rules,
        "",
        "## Alternative explanation",
        "",
        spec.alternative_explanation,
        "",
        "## Conditions for filing (§30.9)",
        "",
    ]
    for item in checklist:
        box = "x" if item.status == PASS else " "
        lines.append(f"- [{box}] **{item.status}** — {item.text}")
        lines += [f"    - {e}" for e in item.evidence]
    return "\n".join(lines) + "\n"
