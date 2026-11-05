//! `firma_lab._native` — the PyO3 extension backing `firma_lab.metrics`
//! (manual §23.1, Stage 7 Part C). **Offline metric computation only** — no
//! simulation logic lives here, and this crate never depends on
//! `firma-kernel` or a plugin (checked the same way as `firma-tui`'s purity,
//! `cargo tree -p firma-py -e normal`).
//!
//! Every formula this module returns (`h`, the four `g_j`, `u`, repertoire
//! entropy, the SC-1…6 thresholds) is computed by `firma-analysis`
//! (ADR 0045/0046) — the same code `firma-conformance`'s Rust tests and
//! `firma-tui`'s live panels call. `firma_lab.metrics` (the Python side,
//! `python/firma_lab/metrics.py`) is a thin wrapper around the two functions
//! here; it does not re-derive any of these formulas in Python.
//!
//! `firma_lab.load` (`python/firma_lab/load.py`) reads event logs and
//! manifests into pandas `DataFrame`s in **pure Python** — that is
//! generic NDJSON/JSON parsing with no domain formula in it, so there is
//! nothing here for it to call through to (ADR 0046's Note).

#![forbid(unsafe_code)]

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use firma_analysis::sanity::{
    reconstruct_checked, try_sanity_from_run as rust_try_sanity_from_run,
};
use firma_config::RunConfig;

fn load_config(config_json: &str) -> PyResult<RunConfig> {
    RunConfig::from_json(config_json)
        .map_err(|e| PyValueError::new_err(format!("invalid RunConfig JSON: {e}")))
}

fn read_events(run_dir: &str) -> PyResult<Vec<firma_core::Event>> {
    let path = std::path::Path::new(run_dir).join("events.ndjson");
    firma_io::read_events::<firma_core::Event>(&path)
        .map_err(|e| PyValueError::new_err(format!("cannot read {}: {e}", path.display())))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| PyValueError::new_err(format!("malformed event record: {e}")))
}

/// The SC-1…SC-6 sanity report (manual §16.2) for one run, as a Python
/// `dict`. Calls straight through to
/// [`firma_analysis::sanity::sanity_from_run`] — the exact function
/// `tests/tests/sanity.rs::sc16_gate` locks as a regression, so a number
/// this returns for a given log is, by construction, the number the Rust
/// gate test would compute for the same log.
///
/// # Errors
/// A `ValueError` if `config_json` does not parse, `run_dir/events.ndjson`
/// cannot be read, the config's `scales` parameter is malformed, or the log
/// contains an unparsable persisted record (the message names each one).
#[pyfunction]
fn sanity_report(py: Python<'_>, config_json: &str, run_dir: &str) -> PyResult<Py<PyDict>> {
    let cfg = load_config(config_json)?;
    let report = rust_try_sanity_from_run(&cfg, std::path::Path::new(run_dir))
        .map_err(PyValueError::new_err)?;

    let d = PyDict::new(py);
    d.set_item("initial_firms", report.initial_firms)?;
    d.set_item("survivors", report.survivors)?;
    d.set_item("sc1_survival", report.sc1_survival)?;
    d.set_item("sc1_pass", report.sc1_pass())?;
    d.set_item("sc2_survival_attention", report.sc2_survival_attention)?;
    d.set_item(
        "sc2_reconstructed_h_below_crit",
        report.sc2_reconstructed_h_below_crit,
    )?;
    d.set_item("sc2_pass", report.sc2_pass())?;
    d.set_item("sc3_binds", report.sc3_binds.to_vec())?;
    d.set_item("sc3_first_bind_tick", report.sc3_first_bind_tick.to_vec())?;
    d.set_item("sc3_pass", report.sc3_pass())?;
    d.set_item("sc4_shaping_fraction", report.sc4_shaping_fraction)?;
    d.set_item("sc4_pass", report.sc4_pass())?;
    d.set_item("sc5_shaping_success", report.sc5_shaping_success)?;
    d.set_item("sc5_pass", report.sc5_pass())?;
    d.set_item("sc6_entropy_variance", report.sc6_entropy_variance)?;
    d.set_item("sc6_entropy_mean", report.sc6_entropy_mean)?;
    d.set_item("sc6_pass", report.sc6_pass())?;
    d.set_item("decisions", report.decisions)?;
    d.set_item("shaping_commits", report.shaping_commits)?;
    d.set_item("all_pass", report.all_pass())?;
    Ok(d.into())
}

/// Every firm's final reconstructed state — `h`, the four `g_j` (§9.1,
/// canonical `[solvency, compliance, scope, obligation]` order), `u`, and
/// repertoire entropy over its trailing window — as a list of Python
/// `dict`s, one per firm, ascending id order. Folds the *entire* event log
/// through [`firma_analysis::Reconstruction`], the same incremental
/// reconstruction `firma-tui`'s live panels use.
///
/// # Errors
/// A `ValueError` if `config_json` does not parse, `run_dir/events.ndjson`
/// cannot be read, the config's `scales` parameter is malformed, or the log
/// contains an unparsable persisted record (the message names each one).
#[pyfunction]
fn final_margins(py: Python<'_>, config_json: &str, run_dir: &str) -> PyResult<Vec<Py<PyDict>>> {
    let cfg = load_config(config_json)?;
    let events = read_events(run_dir)?;
    final_firms(&cfg, &events)
        .map_err(PyValueError::new_err)?
        .into_iter()
        .map(|f| {
            let d = PyDict::new(py);
            d.set_item("id", f.id)?;
            d.set_item("alive", f.alive)?;
            d.set_item("liquid_capital", f.liquid_capital)?;
            d.set_item("input_stock", f.input_stock)?;
            d.set_item("capability", f.capability)?;
            d.set_item("obligation", f.obligation)?;
            d.set_item("legitimacy", f.legitimacy)?;
            d.set_item("regulated_intensity", f.regulated_intensity)?;
            d.set_item("h", f.h)?;
            d.set_item("g", f.g.to_vec())?;
            d.set_item(
                "repertoire_entropy",
                firma_analysis::repertoire_entropy(&f.window),
            )?;
            Ok(d.into())
        })
        .collect()
}

/// Every firm's final snapshot, for `final_margins` — or an error if the
/// config's `scales` parameter is malformed or the log holds an unparsable
/// persisted record ([`reconstruct_checked`]). Margins are never returned
/// from a reconstruction that had to leave records out.
fn final_firms(
    cfg: &RunConfig,
    events: &[firma_core::Event],
) -> Result<Vec<firma_analysis::FirmSnapshot>, String> {
    Ok(reconstruct_checked(cfg, events)?.firms())
}

/// Every live firm-tick of a run ([`firma_analysis::firm_ticks`]) as a list
/// of Python `dict`s in `(tick, agent)` order: `tick`, `agent`, `action`,
/// `h` (reconstructed margin at the end of the tick), `shortfalls`
/// (`[ς_1, ς_2, ς_3]` at the firm's decision in that tick) and
/// `shortfall_max` (`max_j ς_j`). The arithmetic is entirely Rust-side; see
/// the module docs of `firma_analysis::firm_ticks` for what each value is
/// sampled at and what it does not cover.
///
/// # Errors
/// A `ValueError` under the same conditions as `final_margins`.
#[pyfunction]
fn firm_ticks(py: Python<'_>, config_json: &str, run_dir: &str) -> PyResult<Vec<Py<PyDict>>> {
    let cfg = load_config(config_json)?;
    let events = read_events(run_dir)?;
    firma_analysis::firm_ticks::firm_ticks(&cfg, &events)
        .map_err(PyValueError::new_err)?
        .into_iter()
        .map(|f| {
            let d = PyDict::new(py);
            d.set_item("tick", f.tick)?;
            d.set_item("agent", f.agent)?;
            d.set_item("action", f.action)?;
            d.set_item("h", f.h)?;
            d.set_item("shortfalls", f.shortfalls.to_vec())?;
            d.set_item("shortfall_max", f.shortfall_max)?;
            Ok(d.into())
        })
        .collect()
}

/// The identity hash of a run config: parse it exactly as `firma run` does
/// ([`RunConfig::from_json`]), apply the same [`RunConfig::normalise`] the
/// orchestrator applies before hashing, and return
/// [`RunConfig::content_hash`]. Two configs get the same hash iff they are
/// the same `RunConfig` after Rust-side parsing, so a Python-built job
/// config and a run manifest's `resolved_config` (which carries serde-filled
/// defaults such as `"params": null`, `"interventions": []`) compare equal
/// when they describe the same run. Used by `firma_lab.prereg`'s
/// ADR-0053 template-identity check; Python never re-implements the serde
/// defaults itself.
///
/// Plugin `params` objects are compared as written: a config that spells
/// out a plugin default and one that omits it hash differently (the
/// plugin, not `RunConfig`, applies those defaults).
///
/// # Errors
/// A `ValueError` if `config_json` does not parse or cannot be hashed.
#[pyfunction]
fn config_identity_hash(config_json: &str) -> PyResult<String> {
    let mut cfg = load_config(config_json)?;
    cfg.normalise();
    cfg.content_hash()
        .map_err(|e| PyValueError::new_err(format!("cannot hash RunConfig: {e}")))
}

/// The `firma_lab._native` extension module.
///
/// # Errors
/// Never, in practice — `PyResult` is PyO3's required signature for module
/// init; failures here would only come from the Python interpreter itself
/// rejecting a function registration.
#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(sanity_report, m)?)?;
    m.add_function(wrap_pyfunction!(final_margins, m)?)?;
    m.add_function(wrap_pyfunction!(firm_ticks, m)?)?;
    m.add_function(wrap_pyfunction!(config_identity_hash, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use firma_core::{AgentId, DeltaKind, DeltaTarget, Event, PluginId};

    fn cfg() -> RunConfig {
        RunConfig::from_json(
            r#"{
              "experiment": "unit", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
              "seeds": {"mechanism": 910001, "environment": 910002, "shock": 910003, "init": 910004},
              "world": {"ticks": 4, "resources": ["capital", "input"],
                "conflict_resolver": {"id": "conflict.additive", "version": "^1"}},
              "agents": [{"id": 0, "stocks": {"capital": 100}}],
              "rules": [{"id": "constraint.action_window", "version": "^1", "params": {"l_w": 4}}]
            }"#,
        )
        .unwrap()
    }

    fn window_event(records: Vec<String>) -> Event {
        Event::DeltaApplied {
            tick: 2,
            phase: 7,
            origin: PluginId::new("constraint.action_window"),
            target: DeltaTarget::Agent(AgentId(0)),
            kind: DeltaKind::ReplaceAgentList {
                list: "action_window".to_owned(),
                records_json: records,
            },
        }
    }

    #[test]
    fn final_firms_returns_firms_for_a_clean_log() {
        let ok = r#"{"tick":0,"action":1}"#.to_owned();
        assert_eq!(
            final_firms(&cfg(), &[window_event(vec![ok])])
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn final_firms_refuses_a_log_with_one_corrupt_record() {
        let ok = r#"{"tick":0,"action":1}"#.to_owned();
        let bad = r#"{"tick":"CORRUPT-RECORD"}"#.to_owned();
        let Err(err) = final_firms(&cfg(), &[window_event(vec![ok, bad])]) else {
            panic!("a corrupt record must not yield firm margins");
        };
        for p in ["agent 0", "action_window", "CORRUPT-RECORD"] {
            assert!(err.contains(p), "error does not name {p:?}: {err}");
        }
    }
}
