# ADR 0012 — Phase 1 event-log and snapshot format: deterministic NDJSON

**Status:** Accepted (2026-09-03)
**Phase:** 1 (Kernel)
**Relates to:** manual §22.1 (three streams; "Arrow/Parquet for the event log;
compact binary for snapshots"), §22.3 (manifest), §25.2 (DT-1), §25.7 (golden
traces), §21.4 (bit-identity)

## Context

Manual §22.1 says, as a strong default: "Arrow/Parquet for the event log;
compact binary for snapshots." This is a SHOULD-class choice (§0.2), so deviating
requires an ADR (§34.0).

The Phase-1 gate (§26.3) needs three things from the persistence layer: DT-1
(two runs from one manifest produce a **byte-identical** event log), VT-6
(conservation reconstructible from the log over 10,000 ticks), and a golden
trace (§25.7, a committed hash of a reference run's full event log). None of
these needs columnar analytics; all of them need *exact* byte reproducibility,
which is easiest to guarantee with the simplest possible encoder.

## Decision

**Phase 1 uses newline-delimited JSON (NDJSON) for the event log and pretty JSON
for snapshots and the manifest.** One `serde_json` value per line for events, in
emission order, written through a single buffered writer with `\n` (never
`\r\n`) separators and a trailing newline.

Determinism of the encoding rests on:

- `serde_json` serialises struct fields in **declaration order** and `BTreeMap`
  in **sorted key order** — both deterministic. Every map in a logged type is a
  `BTreeMap` (also required by §19.2 / D1).
- Phase-1 logged state is **integer-only** (`i64` stocks, `u64` ticks/ids), so
  there is no float-formatting question yet. When Phase 2 adds `f64` fields
  (`margin`, `capability`), those go through a fixed `{:?}`/`ryu` shortest-repr
  path and DT-1 is verified per-platform in CI (§21.4) — the format decision here
  does not change that obligation.
- No timestamps, durations, or absolute paths in the event stream (those live in
  the manifest, which is identity, not trajectory).

The **manifest** (§22.3) is a single pretty-printed JSON object; its SHA-256 is
the run id. The **snapshot store** writes one pretty JSON file per snapshot tick
plus an index.

**Parquet/Arrow migration is a Phase-2 deliverable**, tracked in `PROGRESS.md`
(OQ-3). The event *schema* (the Rust `Event` enum) is designed now so the later
migration is a re-encoding, not a redesign: every event is a flat, typed record.

## Alternatives

- **Parquet now** (`arrow` + `parquet` crates). Rejected for Phase 1: a large
  transitive dependency tree and a non-trivial audit surface, added to the phase
  whose gate is a working deterministic kernel, to serve analytics that do not
  exist until Phase 3. Parquet writers also have buffering/dictionary/compression
  settings that must all be pinned before byte-identity holds — *more* surface
  for a DT-1 regression, not less.
- **`bincode` / a compact binary encoding.** Smaller and faster, and a reasonable
  Phase-1 choice. Rejected because a human (and a reviewer) cannot read it, and
  Phase-1 runs are tiny; the debugging value of a greppable log outweighs the
  size cost until sweeps start.
- **CBOR.** Deterministic-encoding profile exists, but it is binary (loses
  readability) and adds a dependency for no Phase-1 gain over NDJSON.

## Consequences

**Positive.**
- The simplest possible path to DT-1: byte-identity is "did we write the same
  bytes", trivially auditable with `diff` or `sha256sum`.
- Event logs are human-readable and greppable — valuable while the kernel is
  being brought up and for the `firma-cli verify` path.
- Golden traces (§25.7) are `sha256sum run/events.ndjson`, committed as a hex
  string; a diff on failure is readable.

**Negative, accepted.**
- **NDJSON is 3–10× larger than Parquet** and has no columnar predicate
  push-down. Irrelevant at Phase-1 scale (thousands of ticks, one plugin);
  becomes relevant at Phase-3 sweep scale (§30.4: 63,000 runs), which is exactly
  when the Phase-2 migration lands.
- Two encoders will briefly coexist during the Phase-2 migration; the `Event`
  enum being the single source of truth keeps that window short.
- `serde_json`'s determinism guarantees are strong but *by construction*, not
  spec-guaranteed forever; the golden-trace test (§25.7) is the tripwire if a
  `serde_json` upgrade ever changes output.

## Compliance

- DT-1 in `tests/tests/determinism.rs`: run twice from one manifest, assert
  `events.ndjson` bytes equal.
- Golden trace in `tests/tests/golden.rs`: committed SHA-256 of the reference
  run's event log; any change fails CI and demands a MAJOR bump + stated reason
  (§25.7, §24.6).
- `firma-io` `//!` doc records "NDJSON per ADR 0012; Parquet deferred to Phase 2".

## Note

The manual's §22.1 wording is a recommendation about the *eventual* platform, and
§22.2 is explicit that the binding requirement is *log sufficiency*, not log
format: "the event log MUST be sufficient to reconstruct any quantity a metric
might need." Phase 1 keeps that property (every applied delta, birth, death,
shock, intervention, and violation is logged) while choosing the encoder that
makes the Phase-1 gate cheapest to prove.
