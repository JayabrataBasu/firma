"""Stage 7 Part D -- the cross-language agreement proof.

Runs a real FIRMA simulation once (the ``firma`` CLI binary, built from this
same checkout), then computes the SC-1...SC-6 sanity report and every firm's
final margin two ways against the *same* run directory:

* Rust: ``firma-analysis``'s own ``examples/sanity_report_json`` binary
  (``firma_analysis::sanity::sanity_from_run`` + ``Reconstruction::firms()``
  directly -- the exact function ``tests/tests/sanity.rs::sc16_gate`` locks).
* Python: ``firma_lab.metrics.sanity_report`` / ``final_margins`` -- the
  PyO3 binding over the *same* Rust function (ADR 0045/0046).

If these ever disagree, `firma_lab` has stopped calling through to the Rust
formula and started reimplementing it -- exactly the duplication this
project's standing rule (ADR 0021/0026/0040/0045/0046) forbids. This test
is the tripwire.
"""

from __future__ import annotations

import json
import shutil
import subprocess
from pathlib import Path

import pytest

from firma_lab import metrics

REPO_ROOT = Path(__file__).resolve().parents[2]
FIRMA_BIN = REPO_ROOT / "target" / "debug" / "firma"
CONFIG = REPO_ROOT / "configs" / "experiments" / "phase2-stage5-smoke.json"


@pytest.fixture(scope="module")
def run_dir(tmp_path_factory) -> Path:
    if not FIRMA_BIN.exists():
        pytest.skip(f"{FIRMA_BIN} not built -- run `cargo build` first")
    out = tmp_path_factory.mktemp("part-d-run")
    subprocess.run(
        [str(FIRMA_BIN), "run", "--config", str(CONFIG), "--out", str(out), "--model"],
        check=True,
        capture_output=True,
        text=True,
    )
    return out


@pytest.fixture(scope="module")
def rust_side(run_dir: Path) -> dict:
    """The same numbers, computed by `firma-analysis`'s own example binary
    on the Rust side -- built via `cargo build -p firma-analysis --example
    sanity_report_json` (Stage 7 verification block builds it before pytest
    runs)."""
    example_bin = REPO_ROOT / "target" / "debug" / "examples" / "sanity_report_json"
    if not example_bin.exists():
        pytest.skip(
            f"{example_bin} not built -- run "
            "`cargo build -p firma-analysis --example sanity_report_json` first"
        )
    result = subprocess.run(
        [str(example_bin), str(CONFIG), str(run_dir)],
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


def test_sanity_report_matches_rust_exactly(run_dir: Path, rust_side: dict):
    py = metrics.sanity_report(run_dir)
    rust = rust_side["sanity"]

    # scalar / list fields: exact equality (both sides compute the identical
    # f64 arithmetic -- no tolerance needed, this is the same binary
    # representation on both sides of the FFI boundary).
    for key in [
        "initial_firms",
        "survivors",
        "sc1_survival",
        "sc2_survival_attention",
        "sc2_reconstructed_h_below_crit",
        "sc3_binds",
        "sc3_first_bind_tick",
        "sc4_shaping_fraction",
        "sc5_shaping_success",
        "sc6_entropy_variance",
        "sc6_entropy_mean",
        "decisions",
        "shaping_commits",
    ]:
        assert py[key] == rust[key], f"{key}: python={py[key]!r} rust={rust[key]!r}"


def test_final_margins_h_and_entropy_match_rust_exactly(run_dir: Path, rust_side: dict):
    py = metrics.final_margins(run_dir)
    rust_firms = {f["id"]: f for f in rust_side["firms"]}

    assert set(py["id"]) == set(rust_firms.keys())
    for _, row in py.iterrows():
        r = rust_firms[row["id"]]
        assert row["h"] == r["h"], f"firm {row['id']}: h python={row['h']} rust={r['h']}"
        assert row["g_solvency"] == r["g"][0]
        assert row["g_compliance"] == r["g"][1]
        assert row["g_scope"] == r["g"][2]
        assert row["g_obligation"] == r["g"][3]
        assert row["alive"] == r["alive"]


def test_at_least_one_sc_metric_is_actually_nontrivial(run_dir: Path):
    # Not a tautology check: confirm this run exercises real dynamics (some
    # firm below h_crit, some constraint bound) rather than accidentally
    # validating agreement on an all-zero degenerate case.
    report = metrics.sanity_report(run_dir)
    assert report["decisions"] > 0
    assert any(report["sc3_binds"])
    assert report["sc6_entropy_mean"] >= 0.0
