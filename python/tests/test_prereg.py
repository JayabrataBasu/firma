"""Tests for firma_lab.prereg. Only synthetic specs and configs; nothing is
executed. Seed literals needed by the scan tests are built at runtime, so
this file does not itself add an in-range seed literal to the repository."""

from __future__ import annotations

import copy
import dataclasses
import json

import pytest

from firma_lab import prereg
from firma_lab.runner import build_job_config
from firma_lab.spec import ArmDesign, Factor, build_synthetic_spec


def cargo_out(results: dict[str, str]) -> str:
    lines = ["running tests"]
    for name, r in results.items():
        lines.append(f"test {name} ... {r}")
    lines.append("test result: ok.")
    return "\n".join(lines)


VT_OK = {f"vt{k}_something": "ok" for k in range(1, 9)}
DT_OK = {f"dt{k}_something": "ok" for k in range(1, 7)}


def test_parser_handles_results_sharing_a_line_with_output():
    text = "test vt1_kernel ... ok    GOAL(3): order=[5, 3]\ntest vt8_x ... FAILED\n"
    assert prereg.parse_cargo_test_output(text) == {"vt1_kernel": "ok", "vt8_x": "FAILED"}


def test_validation_pass_requires_every_vt_including_vt8():
    assert prereg.check_validation_tests(cargo_out(VT_OK)).status == prereg.PASS
    no_vt8 = {k: v for k, v in VT_OK.items() if not k.startswith("vt8")}
    item = prereg.check_validation_tests(cargo_out(no_vt8))
    assert item.status == prereg.FAIL and any("VT-8" in e for e in item.evidence)


def test_any_failed_test_fails_the_suite_item():
    item = prereg.check_validation_tests(cargo_out({**VT_OK, "other_check": "FAILED"}))
    assert item.status == prereg.FAIL


def test_missing_output_is_not_evaluated():
    assert prereg.check_determinism_tests(None).status == prereg.NOT_EVALUATED


def test_determinism_suite():
    assert prereg.check_determinism_tests(cargo_out(DT_OK)).status == prereg.PASS
    assert prereg.check_determinism_tests(cargo_out({**DT_OK, "dt6_something": "FAILED"})).status == prereg.FAIL


def test_sanity_is_always_the_owners_call_and_lists_failures_first():
    item = prereg.check_sanity(cargo_out({"sc16_gate": "ok", "sc4_wmax_beta_probe": "FAILED"}))
    assert item.status == prereg.NEEDS_OWNER
    assert item.evidence[0].startswith("FAILING sanity test(s): ['sc4_wmax_beta_probe']")


def test_artefact_suite(tmp_path):
    (tmp_path / "a.rs").write_text("\n".join(f"fn at{k}_check() {{}}" for k in range(1, 5)))
    assert prereg.check_artefact_suite([tmp_path]).status == prereg.FAIL
    (tmp_path / "b.rs").write_text("fn at5_check() {}")
    assert prereg.check_artefact_suite([tmp_path]).status == prereg.NEEDS_OWNER


def test_attachments_are_hashed(tmp_path):
    spec = build_synthetic_spec(num_replicates=2)
    f = tmp_path / "vt8.txt"
    f.write_text("evidence")
    item = prereg.check_attachments([f], spec)
    assert item.status == prereg.PASS
    assert any(spec.content_hash() in e for e in item.evidence)
    assert prereg.check_attachments([], spec).status == prereg.FAIL
    assert prereg.check_attachments([tmp_path / "missing"], spec).status == prereg.FAIL


# -- seed range / template identity (ADR-0053) -------------------------------


def a_registered_config(spec, replicate=2):
    job = next(j for j in spec.jobs() if j.replicate == replicate)
    return build_job_config(spec, job)


def test_identical_config_is_a_collision():
    spec = build_synthetic_spec(num_replicates=3)
    item = prereg.check_seed_range(spec, [("scratch/run", a_registered_config(spec))], [])
    assert item.status == prereg.FAIL
    assert any(e.startswith("IDENTITY COLLISION") for e in item.evidence)


def test_collision_is_found_through_engine_filled_defaults():
    # A run manifest's resolved_config carries serde defaults the input
    # config omitted; the check must still see them as the same run.
    spec = build_synthetic_spec(num_replicates=3)
    cfg = copy.deepcopy(a_registered_config(spec))
    cfg["interventions"] = []
    cfg["world"]["conflict_resolver"]["params"] = None
    manifest = {"identity": {"resolved_config": cfg}}
    item = prereg.check_seed_range(spec, [("m.json", manifest["identity"]["resolved_config"])], [])
    assert item.status == prereg.FAIL


def test_seed_overlap_with_a_different_config_passes_but_is_listed():
    spec = build_synthetic_spec(num_replicates=3)
    cfg = a_registered_config(spec)
    cfg["world"]["ticks"] = 99
    item = prereg.check_seed_range(spec, [("scratch/other", cfg)], [])
    assert item.status == prereg.PASS
    assert any(e.startswith("seed overlap only") for e in item.evidence)


def test_seeds_outside_the_registered_range_are_ignored():
    spec = build_synthetic_spec(num_replicates=3)
    cfg = a_registered_config(spec)
    n = spec.num_replicates + 1
    cfg["seeds"] = {"mechanism": n, "environment": n, "shock": n, "init": n}
    item = prereg.check_seed_range(spec, [("x", cfg)], [])
    assert item.status == prereg.PASS
    assert not any("overlap" in e for e in item.evidence)


def test_source_seed_literals_need_acknowledgement(tmp_path):
    n = 2
    rs = tmp_path / "fixture.rs"
    rs.write_text(f"let s = StreamSeeds {{ mechanism: {n}, environment: {n}, shock: {n}, init: {n} }};\n")
    far = 5000
    (tmp_path / "cfg.json").write_text(json.dumps(
        {"seeds": {"mechanism": far, "environment": far, "shock": far, "init": far}}
    ))
    literals = prereg.scan_seed_literals([tmp_path])
    assert (str(rs), 1, (n, n, n, n)) in literals
    spec = build_synthetic_spec(num_replicates=3)
    assert prereg.check_seed_range(spec, [], literals).status == prereg.NEEDS_OWNER
    ok = prereg.check_seed_range(spec, [], literals, acknowledged=[(str(rs), (n, n, n, n))])
    assert ok.status == prereg.PASS


def test_unbuildable_registered_config_fails_the_check():
    spec = build_synthetic_spec(num_replicates=3)
    bad_arm = ArmDesign("T", (Factor("x", (1, 2), arm="T", apply_kind="rule_param",
                                     rule_id="decision.random", json_path=("x",)),))
    bad = dataclasses.replace(spec, arms=(bad_arm,))
    cfg = a_registered_config(spec)
    item = prereg.check_seed_range(bad, [("x", cfg)], [])
    assert item.status == prereg.FAIL
    assert any("cannot build registered job config" in e for e in item.evidence)


def test_scan_run_configs_reads_manifests_job_records_and_bare_configs(tmp_path):
    spec = build_synthetic_spec(num_replicates=2)
    cfg = a_registered_config(spec, replicate=1)
    (tmp_path / "a").mkdir()
    (tmp_path / "a" / "manifest.json").write_text(json.dumps({"identity": {"resolved_config": cfg}}))
    (tmp_path / "b.json").write_text(json.dumps({"resolved_config": cfg}))
    (tmp_path / "c.json").write_text(json.dumps(cfg))
    (tmp_path / "d.json").write_text(json.dumps({"unrelated": 1}))
    found = list(prereg.scan_run_configs([tmp_path]))
    assert len(found) == 3 and all(c == cfg for _, c in found)


def test_render_is_computed_from_the_spec():
    spec = build_synthetic_spec(num_replicates=3)
    items = [prereg.check_determinism_tests(cargo_out(DT_OK)), prereg.check_determinism_tests(None)]
    doc = prereg.render_prereg(spec, items)
    assert spec.content_hash() in doc
    assert f"{spec.total_cells()} cells × 3 replicates = {spec.total_jobs()} runs" in doc
    assert "- [x] **PASS**" in doc and "- [ ] **NOT_EVALUATED**" in doc
    for h in spec.hypotheses:
        assert f"| {h.id} |" in doc
