# ADR 0037 — The `Locality` interface: a query service, distinct from `G_t`, no MVP consumer

**Status:** Accepted (2026-09-05)
**Phase:** 2 (Model), Stage 5
**Relates to:** manual §7.6 / §34.7 (non-spatial by default; locality is a
plugin behind `neighbours(i, t) -> Set<AgentId>`), §8.3 (`G_t` — the realised
relation graph), §13.1 (`supply` edges carry realised inflows), §20.3
(`Locality` category), §26.6 (Phase 4: multi-firm rivalry, H5 externalities),
§27.2 (Locality is an MVP dimension); ADR-0019 (Locality/Resource are Phase 2
scope)

## Context

§7.6 fixes the interface — `neighbours(agent, t) -> Set<AgentId>` — with MVP
implementations `wellmixed` and `network`. Two questions:

1. Is `Locality` a `Rule` (emits deltas, runs in a phase) or a query service?
2. Is `locality.network` the same structure as `G_t` (the §8.3 relation
   graph), which already exists in `firma-domain`?

## Decision

### 1. `Locality` is a query service, not a `Rule`

```rust
pub trait Locality: Send + Sync {
    fn id(&self) -> &'static str;
    fn neighbours(&self, agent: AgentId, live: &[AgentId], tick: u64) -> Vec<AgentId>;
    fn assumption(&self) -> &str;
}
```

`neighbours` MUST return an ascending, deduplicated list and MUST NOT include
`agent`. `Locality` does **not** implement `firma_core::Rule`: it emits no
`Delta` and does not belong to a phase — it answers a question. It is
therefore **not in `model_registry`** (a `Rule` registry); `firma-cli` reaches
it through `locality_catalogue()`.

### 2. `locality.network` is a different structure from `G_t`

`G_t` (`firma_domain::RelationGraph`) is the **realised** economic graph:
`supply` / `alliance` / `rivalry` edges the model *creates and severs* as
dynamics unfold (a `contract` fixes a supply edge; a death removes all incident
edges — ADR-0017). `locality.network` is the **fixed map of who can
potentially interact at all** (§7.6). Conflating them would make "sever a
supply tie" also mean "these firms can no longer interact", which is not the
model. So `locality.network` takes its own undirected edge list in config and
does not read `keys::RELATION_EDGES`.

### 3. Nothing in the MVP consumes `neighbours()`

Checked honestly: no constraint, action, decision, shock, or resource rule
filters or routes by locality. The mechanics that need it — multi-firm
rivalry, distance-dependent supply, H5 externalities — are Phase 4 (§26.6,
§26.4's platform phase). The interface and both implementations are built and
unit-tested now because §27.2 lists Locality as an MVP dimension (ADR-0019);
**wiring a consumer waits for one to exist.** `wellmixed` vs `network` is
already the intended clean two-level ablation (§34.7) the moment a consumer
lands.

## Alternatives

- **Make `Locality` a `Rule` that writes a `neighbours` list to the store.**
  It has nothing to write that any rule reads, and running it in a phase every
  tick to produce a value nobody consumes is waste. When a consumer exists it
  can call `neighbours()` directly (it is a pure function of `(live, edges,
  tick)`), or a thin adapter rule can materialise it then.
- **Reuse `RelationGraph` for `locality.network`.** Conflates potential and
  realised interaction (see Decision 2).
- **Skip `locality` this Stage since nothing uses it.** §27.2 / ADR-0019 make
  it an MVP deliverable; and building the interface now means the Phase-4
  consumer is written against a fixed contract, not the other way round.

## Consequences

- **Positive.** The `neighbours` contract is fixed and tested; a Phase-4
  rivalry rule is written against it, not vice versa. `wellmixed` (the null)
  and `network` are both ready for the "does interaction structure matter?"
  ablation.
- **Negative, accepted.** A built, tested plugin category with **no runtime
  integration** until Phase 4. Its `assumption()` strings will not appear in
  any current run's manifest (no `Rule` ⇒ no manifest entry) — noted so the
  §27.3-criterion-6 check (every *registered rule* has an assumption) is not
  expected to cover it.
- **Neutral.** No kernel change, no new DeltaKind, no dependency (the crate is
  `core` + `domain` only). No shipped numerical output changes.

## Compliance

- `firma_plugin_locality::Locality` — the trait; `WellMixed`, `Network`;
  `catalogue()` → `(id, hash, ctor)`.
- `firma_cli::locality_catalogue()` re-exports it; not in `model_registry`.
- Tests: `firma-plugin-locality` (`wellmixed` = everyone else, `network` =
  configured adjacency ∩ live, self-loop ignored, non-member empty); conformance
  `locality_plugins_construct_and_answer_neighbours` (both construct, answer,
  and `network` genuinely restricts).

## Note

"Build the interface, defer the consumer" is the honest state here, and saying
so plainly is better than inventing a fake consumer to justify the category.
§34.7's whole point is that locality is a *plugin behind an interface* — the
interface is the deliverable; a spatial result becomes a measured comparison
later, not a design assumption now.
