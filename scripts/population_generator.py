#!/usr/bin/env python3
"""DIAGNOSTIC-ONLY - seeded population heterogeneity (review R20, ADR-0058 DRAFT).

Draws each firm's starting capital, capability, capital-growth aspiration and
(optionally) an initial obligation from the run's *init seed*, in the config
generator. Nothing in the kernel, the engine or ``firma_lab`` changes: the
output is an ordinary ``agents`` list for a ``firma run --model`` config.

Keyed derivation (no stateful generator)
----------------------------------------
For an init seed, a firm id and a purpose tag::

    u = int.from_bytes(sha256(f"{init_seed}|{firm_id}|{purpose_tag}".encode("ascii")).digest()[:8],
                       "big") / 2**64          # uniform on [0, 1)

``init_seed`` and ``firm_id`` are written as plain base-10 integers. The value
for ``(seed, firm, purpose)`` depends on nothing else: not on how many firms
or purposes exist, not on the order in which they are generated. Normal and
lognormal values are obtained from ``u`` by the inverse normal CDF
(``statistics.NormalDist().inv_cdf``). ``u == 0`` (probability 2**-64) has no
inverse-CDF value and raises rather than being replaced by another number.

Distributions (the numbers live in ``SPEC``; ``spec_bytes()`` is what step 0
writes to ``docs/review/evidence/R20/hetero_spec.json``)
-----------------------------------------------------------------------------
* capital: lognormal with median 100 and coefficient of variation ``cv_cap``
  (``sigma = sqrt(ln(1 + cv_cap**2))``), rounded half up to an integer;
* capability: ``clip(Normal(0.55, sd_cap), 0.05, 1.0)``;
* aspiration_capital_growth: uniform on ``[8.0 - a_spread, 8.0 + a_spread]``
  (``a_spread`` is an ABSOLUTE half-width, the literal reading of the R20
  prompt);
* obligation: with probability 1/6 the firm starts with ``q`` uniform on the
  integers 50..80; ``theta_q`` is 40 for every drawn population.

Every other per-firm field is copied from firm 0 of the base config, as in the
R16/R18 regime grids.

Usage: ``population_generator.py --write-spec <path>`` (step 0), or import.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
import statistics
from pathlib import Path
from typing import Any, Mapping, Sequence

BANNER = "DIAGNOSTIC-ONLY - SEEDED POPULATION HETEROGENEITY - NOT REGISTERED - NOT FOR TUNING"

#: Purpose tags. One draw per (init seed, firm id, tag).
TAG_CAPITAL = "capital"
TAG_CAPABILITY = "capability"
TAG_ASPIRATION = "aspiration_capital_growth"
TAG_OBLIGATION_FLAG = "obligation_flag"
TAG_OBLIGATION_Q = "obligation_q"

SPEC: dict[str, Any] = {
    "task": "R20 HETERO - DIAGNOSTIC-ONLY; nothing registered; not for tuning",
    "disclosure": "theta_limit = 0.90 was chosen by the reviewer AFTER seeing regime grid 2 "
    "(R18: SC-2 was only reachable at 0.90). The distributions and spread levels are "
    "reviewer-proposed (R20) and were fixed here before any run.",
    "derivation": {
        "uniform": "u = int.from_bytes(sha256(f'{init_seed}|{firm_id}|{purpose_tag}'"
        ".encode('ascii')).digest()[:8], 'big') / 2**64",
        "byte_order": "big-endian (builder's choice; the prompt says 'first 8 bytes')",
        "normal": "statistics.NormalDist().inv_cdf(u); u == 0 raises",
        "purpose_tags": [TAG_CAPITAL, TAG_CAPABILITY, TAG_ASPIRATION, TAG_OBLIGATION_FLAG,
                         TAG_OBLIGATION_Q],
    },
    "n_firms": 12,
    "distributions": {
        "capital": {"family": "lognormal", "median": 100, "cv": "cv_cap",
                    "sigma": "sqrt(ln(1 + cv_cap^2))", "value": "100 * exp(sigma * z)",
                    "rounding": "half up to an integer; no clipping"},
        "capability": {"family": "normal", "mean": 0.55, "sd": "sd_cap",
                       "clip": [0.05, 1.0], "rounding": "none"},
        "aspiration_capital_growth": {
            "family": "uniform", "centre": 8.0, "half_width": "a_spread",
            "value": "8.0 - a_spread + 2 * a_spread * u",
            "reading": "a_spread is an ABSOLUTE half-width (literal reading of 'uniform within "
            "+/- a_spread of 8.0'); the relative reading 8.0 * (1 +/- a_spread) was NOT used"},
        "obligation": {"probability": "u(obligation_flag) < 1/6",
                       "q": "50 + floor(31 * u(obligation_q)), i.e. uniform on integers 50..80",
                       "theta_q": 40, "q_max": 100},
        "every_other_per_firm_field": "copied from firm 0 of the base config (input stock 40, "
        "legitimacy 1.0); firm ids are 0 .. n-1",
    },
    "spread_levels": {
        "low": {"cv_cap": 0.25, "sd_cap": 0.05, "a_spread": 0.10},
        "medium": {"cv_cap": 0.50, "sd_cap": 0.10, "a_spread": 0.25},
        "high": {"cv_cap": 1.00, "sd_cap": 0.20, "a_spread": 0.50},
    },
    "fixed": {
        "base_config": "configs/experiments/phase2-stage5-smoke.json (rule set and all "
        "parameters not listed here)",
        "output_price": 3, "input_price": 2, "theta_limit": 0.90, "theta_cap": 0.40,
        "theta_q": 40,
        "decision": "decision.satisficing (l_w = 4; plugin defaults otherwise)",
        "observation": "observation.full",
        "shock": "shock.stochastic as in R12: regulatory/theta_limit, onset uniform 10..80, "
        "instant, permanent, targets all; magnitude_mean 0.20, magnitude_sd 0.20 / 3",
        "horizon_ticks": 120,
        "mechanism_seed": 910001, "environment_seed": 910002,
    },
    "seed_sets": {"A": "init seeds 910001..910050", "B": "init seeds 911001..911050"},
    "Q2_Q3_Q4_runs": {
        "per_spread_level_and_init_seed": "one NO-SHOCK baseline (shock rule removed, shock "
        "seed = init seed) and one shocked run (shock seed = init seed). The baseline IS the "
        "shocked run's no-shock fork (same config and seeds, shock rule removed).",
        "per_run_metrics": {
            "survivors": "12 minus the number of agent_died records",
            "collapse": "survivors <= 20% of n, i.e. <= 2 of 12",
            "distinct_compliance_death_ticks": "number of distinct ticks among deaths whose "
            "cause is 'compliance' (0 if none)",
            "mass_death_tick": "at least 5 firms die in one tick (any cause)",
            "matched_delta_h": "R14 definition (scripts/exploratory_e1_slice.py::matched_summary)",
        },
    },
    "Q3_difference_rule": "a metric differs between set A and set B if |A - B| > 0.10, or if "
    "the larger is more than 2x the smaller (when the smaller is > 0; a zero against a "
    "non-zero counts as a difference)",
    "Q4_success_criteria": {
        "status": "REVIEWER-PROPOSED, fixed at step 0",
        "T1": "per init seed, on the no-shock baseline: SC-1 sc1_survival in [0.60, 0.90]; SC-2 "
        "sc2_reconstructed_h_below_crit in [0.05, 0.25]; SC-3 sc3_pass and SC-6 sc6_pass from "
        "firma_analysis (same T1 as R16/R18, via scripts/regime_search.py::evaluate_targets)",
        "criteria": ["at least 50% of init seeds pass T1", "collapse rate <= 10% of shocked runs",
                     "at least 50% of shocked runs have non-zero matched DeltaH"],
        "pooling": "the verdict per spread level is computed over sets A and B pooled (100 init "
        "seeds); the same three numbers are also shown per set",
    },
    "Q5": {
        "cells": {"pinned_margin": [0.02, 0.40], "pinned_shortfall_capital_growth": [0.0, 1.0],
                  "companions": "pinned_shortfall_capability = 0.0 and "
                  "pinned_shortfall_obligation_clearance = 0.0 (ADR-0054; firma_lab/spec.py "
                  "Arm A 'varsigma' factor)",
                  "beta": 1.0},
        "pin_mechanism": "config 'interventions' at tick 0, written by "
        "firma_lab.runner._add_pin_intervention (imported, not edited)",
        "drawn_arm": "n = 1: firm id 0 drawn by this generator at the MEDIUM spread level "
        "(all five purposes, including the obligation draw), init seeds 910001..910050",
        "negative_control": "n = 1: firm 0 of the base config unchanged (capital 600, input 40, "
        "capability 0.6, aspiration 8.0, no obligation), same 50 init seeds; expect 1 sequence",
        "config": "the fixed items above with the shock rule removed (no shock in Q5); shock "
        "seed 910001; decision.satisficing params l_w = 4, beta = 1.0",
        "outcome": "H_rep per the R12 definition: mean over live firm-ticks of the entropy of "
        "the trailing 8 selected actions; and the sha256 of the selected-action sequence",
    },
    "controls": {
        "byte_equal": "low-spread init seed 910001: config built twice (byte-equal JSON) and "
        "the shocked run executed twice (byte-equal events.ndjson)",
        "cliff_detection": "the R12 4-firm smoke slice config and R18 grid-2 cell 79 at shock "
        "mean 0.20, each over shock seeds 910001..910020 (init seed 910004), evaluated with "
        "the Q2/Q4 code, must reproduce the numbers recorded in evidence/R16 and evidence/R18",
    },
    "choices_made_by_the_builder_before_any_run": [
        "big-endian byte order for the 8 hash bytes",
        "a_spread read as an absolute half-width",
        "capital rounded half up, capability not rounded",
        "q = 50 + floor(31 u), integers 50..80 inclusive",
        "theta_q = 40 in every drawn population, whether or not any firm is seeded",
        "collapse = at most 2 survivors of 12",
        "Q4 verdict pooled over sets A and B",
        "Q5: no shock; the drawn single firm keeps its obligation draw; the negative control "
        "is base-config firm 0",
        "the Q3 relative-difference rule when one value is zero",
    ],
}

SPREAD_LEVELS = tuple(SPEC["spread_levels"])  # declared order: low, medium, high


def spec_bytes() -> bytes:
    return (json.dumps(SPEC, indent=2, sort_keys=True) + "\n").encode("utf-8")


def spec_sha256() -> str:
    return hashlib.sha256(spec_bytes()).hexdigest()


# ---------------------------------------------------------------------------
# Keyed derivation
# ---------------------------------------------------------------------------


def uniform(init_seed: int, firm_id: int, purpose_tag: str) -> float:
    """The keyed uniform on [0, 1) (see the module docstring)."""
    key = f"{int(init_seed)}|{int(firm_id)}|{purpose_tag}".encode("ascii")
    return int.from_bytes(hashlib.sha256(key).digest()[:8], "big") / 2**64


def standard_normal(init_seed: int, firm_id: int, purpose_tag: str) -> float:
    """Inverse-CDF standard normal from the keyed uniform."""
    u = uniform(init_seed, firm_id, purpose_tag)
    if u == 0.0:
        raise ValueError(
            f"keyed uniform is exactly 0 for ({init_seed}, {firm_id}, {purpose_tag!r}); "
            "the inverse normal CDF is undefined there"
        )
    return statistics.NormalDist().inv_cdf(u)


def _round_half_up(x: float) -> int:
    return int(math.floor(x + 0.5))


def draw_values(init_seed: int, firm_id: int, level: str) -> dict[str, Any]:
    """The drawn values for one firm at one spread level. Depends only on
    ``(init_seed, firm_id, level)``."""
    s = SPEC["spread_levels"][level]
    d = SPEC["distributions"]
    sigma = math.sqrt(math.log(1.0 + s["cv_cap"] ** 2))
    capital = _round_half_up(
        d["capital"]["median"] * math.exp(sigma * standard_normal(init_seed, firm_id, TAG_CAPITAL))
    )
    lo, hi = d["capability"]["clip"]
    capability = min(hi, max(lo, d["capability"]["mean"]
                             + s["sd_cap"] * standard_normal(init_seed, firm_id, TAG_CAPABILITY)))
    centre = d["aspiration_capital_growth"]["centre"]
    aspiration = (centre - s["a_spread"]
                  + 2.0 * s["a_spread"] * uniform(init_seed, firm_id, TAG_ASPIRATION))
    obligation = None
    if uniform(init_seed, firm_id, TAG_OBLIGATION_FLAG) < 1.0 / 6.0:
        obligation = 50 + int(math.floor(31 * uniform(init_seed, firm_id, TAG_OBLIGATION_Q)))
    return {"capital": capital, "capability": capability,
            "aspiration_capital_growth": aspiration, "obligation": obligation}


def draw_firm(init_seed: int, firm_id: int, level: str,
              template_firm: Mapping[str, Any]) -> dict[str, Any]:
    """One config ``agents`` entry: ``template_firm`` with the drawn values."""
    v = draw_values(init_seed, firm_id, level)
    firm = copy.deepcopy(dict(template_firm))
    firm["id"] = firm_id
    firm["stocks"] = dict(firm["stocks"])
    firm["stocks"]["capital"] = v["capital"]
    firm["reals"] = dict(firm["reals"])
    firm["reals"]["capability"] = v["capability"]
    firm["reals"]["aspiration_capital_growth"] = v["aspiration_capital_growth"]
    firm.pop("ints", None)
    if v["obligation"] is not None:
        firm["ints"] = {"obligation": v["obligation"]}
    return firm


def population(init_seed: int, n: int, level: str, template_firm: Mapping[str, Any],
               generation_order: Sequence[int] | None = None) -> list[dict[str, Any]]:
    """``n`` drawn firms, returned in id order. ``generation_order`` (a
    permutation of ``range(n)``) only changes the order in which the firms
    are generated; it exists so a test can show the order does not matter."""
    order = list(range(n)) if generation_order is None else list(generation_order)
    if sorted(order) != list(range(n)):
        raise ValueError("generation_order must be a permutation of range(n)")
    firms = {i: draw_firm(init_seed, i, level, template_firm) for i in order}
    return [firms[i] for i in range(n)]


def main(argv: Sequence[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=BANNER)
    ap.add_argument("--write-spec", help="STEP 0: write hetero_spec.json to this path and exit")
    args = ap.parse_args(argv)
    print(BANNER)
    if not args.write_spec:
        print(f"declared spec sha256 {spec_sha256()}; use --write-spec <path> for step 0")
        return 0
    path = Path(args.write_spec)
    if path.exists():
        print(f"REFUSED: {path} already exists; the spec is declared once")
        return 1
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(spec_bytes())
    print(f"step 0: wrote {path}; sha256 {spec_sha256()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
