# ADR 0013 — Enforce `PluginId::numeric()` uniqueness at rule registration

**Status:** Accepted (2026-09-03)
**Phase:** 1 (Kernel) — hardening fix found during review
**Relates to:** manual §17 A3 (determinism is structural, not disciplinary),
§18.2 (`firma-registry` rejection classes), §21.2 property 4 (stream
isolation), §25.2 DT-6, §15.5 (Example E), ADR 0003, ADR 0008

## Context

`PluginId::numeric()` (`crates/firma-core/src/ids.rs`) is a **64-bit FNV-1a
hash of the plugin's id string**. The kernel's RNG key builder
(`crates/firma-kernel/src/lib.rs`, `fn rng_key`) feeds `rule.id().numeric()`
straight into `RngKey.plugin_id`, which `firma-rng::digest()` then folds into
the SHA-256 key material:

```text
key = H(run_seed ‖ stream_id ‖ plugin_id ‖ phase_id ‖ tick ‖ agent_id ‖ purpose_tag)
```

**Nothing in the system checks `numeric()` for uniqueness.** `firma-registry`
keys its rule table on the id *string* and asserts only that the string is not
already registered; `numeric()` is never computed, stored, or compared
anywhere.

If two distinct, legitimately-registered rule plugins hash to the same
`numeric()` value, their `RngKey`s become **byte-identical** for every
otherwise-matched `(run_seed, stream, phase, tick, agent_id, purpose_tag)`
tuple. They would then draw from the *same* Philox stream and receive
correlated draws while every part of the system — and every ablation built on
CRN forks — believes them independent.

This is exactly the failure mode manual §21.2 property 4 ("Stream isolation —
adding or removing a rule perturbs no other rule's draws") and §25.2 DT-6
exist to catch, **except triggered by plugin identity rather than by an added
rule**. DT-6 as currently written cannot catch it: it distinguishes two rules
by `purpose_tag`, never by `plugin_id`, so a `plugin_id` collision would pass
DT-6 green while silently correlating draws. Per §17 A3, a failure this severe
must not rest on the astronomically-low-but-unchecked probability of an FNV-1a
collision — that is a disciplinary guarantee where a structural one is
required, and A3 is one of CLAUDE.md's seven non-negotiables.

**Where this sits relative to existing guarantees.** `firma-registry` already
enforces three rejection classes at plugin resolution — unknown plugin,
version mismatch, content-hash mismatch (manual §18.2). This is the same class
of guarantee with one case missing.

**The same hazard is already reasoned about elsewhere for the same type.**
`crates/firma-core/src/delta.rs`'s `Ord` impl for `Delta` comments explicitly
that it compares `origin` *by its string, not the FNV projection*, "so the
order is total — no hash collision can make two distinct plugins compare
equal." That is the identical concern, handled, in a *less* consequential path
(sort stability rather than RNG correlation). This ADR applies the same
reasoning to the path where it actually matters.

**Corroborating evidence from the manual.** §15.5 Example E shows `plugin_id`
for `decision.satisficing` as `0x1A` — a single byte, consistent with a small
assigned integer, not an 8-byte hash. This does not by itself dictate the fix
(see Rejected alternatives), but it confirms the manual's illustrative
encoding assumed something closer to a collision-free identifier than what was
built.

## Decision

**`firma-registry::Registry` gains a second table — `numeric()` → owning id
string — and `register_rule` rejects a collision before insertion, with the
same severity and at the same point as the existing duplicate-string-id
assertion: a start-up wiring bug, a `panic!`, not a runtime `Result`.**

```rust
pub struct Registry {
    rules: BTreeMap<String, RegisteredRule>,
    resolvers: BTreeMap<String, RegisteredResolver>,
    rule_numeric_ids: BTreeMap<u64, String>, // PluginId::numeric() -> owning string id
}
```

`register_rule` computes `r.id.numeric()`; if that value is already recorded
against a *different* string id, it panics with a diagnostic naming both ids,
the colliding value, and the governing manual sections. Registering the same
string id twice still hits the pre-existing duplicate-id panic, not this one.

This converts a `numeric()` collision from a silent statistical event into a
loud, deterministic failure at process start-up — a structural guarantee
(§17 A3) that the *registered set* is injective under `numeric()`, which is
what makes the current hash scheme safe to keep using in the RNG key path.

**Scope: rule registrations only.** The RNG key formula, the §21.2 field
widths and encoding (open question OQ-1), `PluginId::numeric()` itself, and
the resolver table are all untouched.

## Rejected alternatives

- **Switch `PluginId::numeric()` from a hash to a registry-assigned integer**
  (a registration index, matching §15.5's `0x1A`). This is the honest
  long-term fix, but it is **out of scope for a hardening change**: it would
  alter the RNG key derivation for every existing registration; it would
  ripple into the `firma-rng` Example E unit tests, which call `numeric()`
  standalone with no registry in scope; and it would reopen the §21.2
  width/encoding question that OQ-1 deliberately leaves open. A hardening fix
  must not move numerical output. This belongs in a future manual revision
  (§15.5 already leans that way) and its own ADR, not here.

- **Extend the check to resolver registrations.** `rng_key()` only ever calls
  `.numeric()` on a `Rule`'s id; a `ConflictResolver` takes `&[Delta] ->
  Vec<Delta>` and never receives an `RngKey`, so a resolver `numeric()`
  collision cannot currently affect any draw. Guarding a property that cannot
  be violated given the call graph adds surface for no benefit. Revisit if a
  later phase gives resolvers RNG access.

- **Make it a resolve-time `RegistryError` rather than a start-up panic.** The
  collision is a property of the *registered set*, fully known at wiring time,
  not of any particular `PluginRef` passed to `resolve_rule`. It should fail
  before any run starts — like the existing duplicate-id assertion — so a
  mis-wired binary cannot get partway into a run. A `Result` on the resolve
  path would be the wrong shape and the wrong time.

## Consequences

**Positive.**
- A `numeric()` collision becomes impossible to introduce unnoticed: it aborts
  registry construction (hence `standard_registry()`, hence any `firma run`)
  at process start with a message naming both colliding plugins.
- The RNG key path gains the same structural collision guarantee that the
  `Delta` ordering path already has for `PluginId`.
- Closes the gap between "§18.2 enumerates three plugin-identity rejection
  classes" and the actual set of ways plugin identity can corrupt a run.

**Negative, accepted.**
- The check is an `assert!`/`panic!`, so it is active in release builds too
  (Rust does not compile `assert!` out). This is intentional and matches the
  existing duplicate-id assertion; the cost is one `BTreeMap<u64, String>`
  lookup + insert per rule registration — negligible for the handful of rules
  a run loads.
- This makes the hash *safe to use*; it does not settle whether `plugin_id`
  should be a hash at all. That deeper question (first rejected alternative)
  remains open for a future manual revision.

**Neutral.**
- **No numerical output changes for any currently-passing config.** No
  registered *rule* collides under `numeric()` — `testkit.transfer` and
  `testkit.keyed_nudge` have distinct values, and the new guard would abort
  registration if they did not. (`conflict.additive` is a resolver, outside
  this guard's scope by design — see the second rejected alternative — so it
  is neither guarded nor checked; that is intentional, not an oversight.) This
  is hardening, not a behaviour change: **no MAJOR bump under §20.5, no
  golden-trace update required.**

## Compliance

- `crates/firma-registry/src/lib.rs`: `register_rule` panics on a `numeric()`
  collision with a distinguishing message.
- `crates/firma-registry/src/lib.rs` tests, three of them:
  - `numeric_collision_rejected` — seeds the private `rule_numeric_ids` table
    to simulate a *different* id already owning a slot (no real FNV-1a
    collision is searched for), then registers a rule that hashes to it, and
    asserts the new collision panic fires.
  - `duplicate_string_id_still_rejected_by_the_original_check` — registering
    the same id string twice still trips the *original* duplicate-id panic,
    not the new collision panic, proving the new check does not fire on the
    case it is not meant to catch.
  - `distinct_ids_register_without_collision` — two genuinely distinct ids
    register cleanly, guarding against a false positive on the normal path.
- DT-6 is unchanged. The review established that DT-6 structurally cannot
  exercise the `plugin_id` collision path; this ADR's start-up guard is the
  structural replacement, not a new probabilistic test.

## Note

The right long-term answer is an assigned, collision-free `plugin_id`,
matching §15.5's `0x1A` — but that touches the §21.2 key tuple and OQ-1 and is
a manual-revision decision with its own ADR. Until someone takes that on, this
ADR guarantees the registered set is injective under `numeric()`, and that is
precisely the property that lets the current hash scheme stay in the RNG key
path without a silent-correlation hazard.
