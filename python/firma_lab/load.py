"""firma_lab.load -- read event logs and manifests into DataFrames (manual
Sec 23.1).

Pure Python (pandas), not a PyO3 binding: this is generic NDJSON/JSON
parsing with no FIRMA formula in it, so there is nothing for it to call
through to the Rust side for (contrast `firma_lab.metrics`, ADR 0046's
Note). Dependency choice: **pandas** -- every downstream Sec 23.1 module
(`stats`, `sensitivity`, `plot`; Phase 3) is specified against DataFrames,
and pandas is the ecosystem default for exactly this shape of long,
event-per-row scientific data.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Union

import pandas as pd

PathLike = Union[str, Path]


def read_events(run_dir: PathLike) -> pd.DataFrame:
    """Read ``run_dir/events.ndjson`` into a DataFrame, one row per event.

    Every event's own fields land as columns (``pandas.json_normalize``);
    a field a given event kind does not carry is ``NaN`` for that row, so
    ``df[df.event == "delta_applied"]`` is an ordinary filter, not a
    special case.
    """
    path = Path(run_dir) / "events.ndjson"
    rows = []
    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            rows.append(json.loads(line))
    return pd.json_normalize(rows)


def read_manifest(run_dir: PathLike) -> dict:
    """Read ``run_dir/manifest.json`` (manual Sec 22.3) as a plain dict.

    The resolved config, plugin list, and everything ``run_id()`` hashes is
    under ``manifest["identity"]``; provenance (duration, host, build) is
    under ``manifest["execution"]`` and is never hashed into the run id.
    """
    path = Path(run_dir) / "manifest.json"
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def read_config_json(run_dir: PathLike) -> str:
    """The resolved ``RunConfig`` JSON text for a run, from its manifest.

    What ``firma_lab.metrics`` passes through to the Rust reconstruction —
    it re-parses this config on the Rust side (the same
    ``RunConfig::from_json`` every other consumer uses); this function
    never re-derives FIRMA's own config-resolution logic in Python.
    """
    manifest = read_manifest(run_dir)
    return json.dumps(manifest["identity"]["resolved_config"])
