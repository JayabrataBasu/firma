---
title: "FIRMA — Project Manual"
author: ""
date: ""
---

# FIRMA — PROJECT MANUAL

### A Computational Laboratory for Firm Behaviour Under Constraint

**Document ID:** FIRMA-MANUAL
**Version:** 1.0.1
**Date:** 2 September 2026 (v1.0.1 PATCH: §28.2 experiment-table relabel, see ADR-0052)
**Status:** Authoritative and complete. Single source of truth.

**Supersedes and consolidates:** FIRMA-SPEC-001 v1.1.0 · FIRMA-MODEL-001 v1.0.0 · FIRMA-LIT-001 v1.1.0 · ADR 0001–0009 · TC-001–TC-005 · FIRMA-PREREG-E1 v1.0.0 · FIRMA-SPEC-001-AMD-001.
All prior documents are retired. Cite this manual by section number only.

**Not consolidated:** the ENDOGENON design analysis, which concerns a different, deferred project (multi-level organisational emergence). It remains a separate document and is referenced only in §31.5.

---

# MASTER INDEX

### Part 0 — Using this manual
| § | Title |
|---|---|
| 0.1 | Purpose and conformance test |
| 0.2 | Requirement language |
| 0.3 | Epistemic status tags |
| 0.4 | Reading paths |
| 0.5 | What this project is not |
| 0.6 | Change control |

### Part I — Purpose and science
| § | Title |
|---|---|
| 1 | Mission |
| 2 | Research question and hypotheses |
| 3 | Theoretical foundations |
| 4 | What FIRMA can and cannot establish |
| 5 | Terminology (binding) |

### Part II — Formal model
| § | Title |
|---|---|
| 6 | Mathematical core |
| 7 | Ontology |
| 8 | State |
| 9 | Constraints and viability |
| 10 | Dynamics and phases |
| 11 | Actions |
| 12 | Goals, observation, decision |
| 13 | Dependence and shocks |
| 14 | Measurement |
| 15 | Worked examples (normative) |
| 16 | Parameters and sanity conditions |

### Part III — Software architecture
| § | Title |
|---|---|
| 17 | Architecture principles |
| 18 | Module map |
| 19 | Kernel |
| 20 | Plugin system |
| 21 | Determinism and RNG |
| 22 | Persistence and provenance |
| 23 | Orchestration, analysis, interface |
| 24 | Code sustainability standards |
| 25 | Testing strategy |

### Part IV — Implementation
| § | Title |
|---|---|
| 26 | Phase plan |
| 27 | The MVP, defined |

### Part V — Research programme
| § | Title |
|---|---|
| 28 | Experiments |
| 29 | Statistical methodology |
| 30 | Pre-registration: Experiment E1 |
| 31 | Positioning, prior art, publication |
| 32 | Risks |
| 33 | Open questions and acknowledged gaps |

### Part VI — Decision records
| § | Title |
|---|---|
| 34.0 | ADR rules and index |
| 34.1–34.9 | ADR 0001–0009 |

### Part VII — Translation contracts
| § | Title |
|---|---|
| 35.0 | Contract rules |
| 35.1–35.5 | TC-001–TC-005 |

### Part VIII — Literature verification
| § | Title |
|---|---|
| 36 | Phase 0 literature verification report |

### Appendices
| § | Title |
|---|---|
| A (§37) | Notation |
| B (§38) | Directory layout |
| C (§39) | Checklists |
| D (§40) | Verified citations |

---

# PART 0 — USING THIS MANUAL

## 0.1 Purpose and conformance test

This manual specifies FIRMA completely enough that **a competent implementer could build and run it from this document alone**, without access to source code, prior documents, or its authors.

The test is not comprehensiveness but *unambiguity*: every quantity has a type, every transition an order, every tie a break rule, every parameter a default and a range.

Every worked example in §15 is computed by hand and is normative. If an implementation disagrees with one, the implementation is wrong — or this manual is, in which case it takes a version bump and an ADR.

## 0.2 Requirement language

| Word | Meaning |
|---|---|
| **MUST** | Mandatory. A build violating this is non-conforming. |
| **MUST NOT** | Prohibited without exception. |
| **SHOULD** | Strong default. Deviation requires a recorded ADR (§34). |
| **MAY** | Permitted at implementer discretion. |

## 0.3 Epistemic status tags

Claims about the world — as distinct from requirements on software — carry tags.

| Tag | Meaning |
|---|---|
| `[E]` | Established. Peer-reviewed, broadly accepted. |
| `[P]` | Plausible but contested. The literature disagrees; both sides noted. |
| `[S]` | Speculative. A hypothesis of this project. |
| `[D]` | Design decision. A choice, not a factual claim. |

**No FIRMA output may present an `[S]` claim as though it were `[E]`.**

## 0.4 Reading paths

| If you are… | Read |
|---|---|
| Deciding whether to fund or continue | §1, §2, §4, §31, §32, §36 |
| Implementing the kernel | §6, §17–§25, §34 |
| Implementing the model | §6–§16, §34.4, §34.6, §34.7 |
| Designing an experiment | §5, §14, §28–§30, §35 |
| Writing a paper | §4, §5.3, §31, §35, §39C, §40 |
| Returning after a long absence | §0.5, §1.3, §2, §34 |

**If you read only four sections:** §1 (mission), §2 (question), §5 (terminology), §27 (MVP).

## 0.5 What this project is not

Stated first, because it prevents the likeliest misreading.

- **Not an artificial-life simulator.** Biological mechanisms are optional plugins, never defaults (§3.4).
- **Not an economic forecasting tool.** It makes no predictions about actual firms or markets (§4).
- **Not a general-purpose ABM framework.** It is purpose-built for one class of question.
- **Not an optimiser.** It has no objective function to maximise. It asks what is *survivable*, not what is *optimal* (§3.1).

## 0.6 Change control

Semantic versioning.

| Change | Bump |
|---|---|
| Altering a MUST, a primitive, or the formal model | MAJOR |
| Adding a phase, plugin category, metric, ADR, or contract | MINOR |
| Clarification, typo, expanded rationale | PATCH |

A MAJOR bump requires an ADR. **The manual version in force MUST be recorded in every run manifest**, so any run can be interpreted against the rules that governed it.

---

# PART I — PURPOSE AND SCIENCE

# 1. Mission

## 1.1 Name

**FIRMA.** From Latin *firmus* — steady, enduring — the root of both "firm" and "firmness." Not an acronym; MUST NOT be expanded into one.

Rejected: *ENDOGENON* (belongs to the deferred emergence project); *VIABLE* (forced acronym); *KERNEL* (collides with operating-system usage in Part III).

## 1.2 Mission statement

FIRMA is a deterministic, reproducible computational laboratory for studying how firms behave when they cannot do whatever they want. It represents a firm as a bounded-rational agent whose available actions are limited by a formally specified constraint set — resources, law, scope, capability — and whose behaviour is driven by staying inside that set rather than by maximising an objective. Its distinguishing feature is that firms can act not only *within* their constraints but *upon* them: lobbying, contracting, allying, and co-opting are first-class actions that move the constraint boundary itself, at a cost and with uncertainty. Every construct it measures is defined interventionally — as a comparison between a factual trajectory and a counterfactual one run with matched randomness — so that claims rest on controlled manipulation rather than narrative interpretation of single runs.

## 1.3 Design priorities, in strict order

When two conflict, the higher-numbered yields. **This ordering is binding.**

1. **Reproducibility.** A result that cannot be exactly regenerated is not a result.
2. **Interpretability.** A mechanism whose effect cannot be traced is not a mechanism.
3. **Theoretical modularity.** Any theory of firm behaviour must be replaceable without touching the kernel.
4. **Extensibility.** New theories, constraints, and measures addable without rewriting.
5. **Performance.** Fast enough that sweeps are routine, not events.
6. **Ergonomics.** Pleasant to use.

Performance MUST NOT be purchased with nondeterminism. Ergonomics MUST NOT be purchased with hidden coupling.

## 1.4 Intended user

A researcher asking: *given these constraints and this threat, what does a firm do — and would it have done something different had one specific thing changed?* The platform exists to make the second half answerable rigorously.

---

# 2. Research question and hypotheses

> **Revision history.** This section was revised once, after the Phase 0 literature check fired twice in a single day (§36, ADR 0009). The contribution claim it supports has been narrowed accordingly and is stated precisely in §31.2.

## 2.1 Foundational question

> Do performance shortfall and viability proximity drive organisational **search breadth** in opposite directions; do they operate **simultaneously** rather than by attention-switching; and does the capacity to act upon one's own viability boundary change the relationship?

Two quantities that covary in all field data are distinguished:

- **Performance shortfall** $\varsigma$ — gap between realised outcome and an adaptive aspiration level (§12.1; contract §35.4).
- **Viability proximity** $h$ — normalised distance to a constraint whose violation terminates the firm (§9.2; contract §35.1).

## 2.2 Why this question

**It has an established theoretical parent.** March & Shapira (1992, *Psychological Review* 99(1):172–183) modelled risk-taking with two reference points — the aspiration level and the survival point — using random-walk conceptions of performance, and showed that which reference point attention falls on determines whether risk-taking rises or falls as fortune worsens `[E]`.

**It targets a live, named contradiction.** The behavioural theory of the firm predicts shortfall *increases* search; the threat-rigidity thesis (Staw, Sandelands & Dutton, 1981, *ASQ* 26(4):501–524) predicts threat *decreases* it. Both are extensively supported `[E]`. Billinger, Stieglitz & Schumacher (*Organization Science*, 2013) found experimentally that failure promotes *more* exploratory search — the BTOF direction `[E]`.

**The obstacle to resolving it is exactly what simulation removes.** A 2023 *JPART* treatment attributes the contradiction's persistence to the fact that decision-makers' search dynamics are not directly empirically observed, so the two mechanisms cannot be examined simultaneously `[E]`. In FIRMA, search width is a logged variable and both mechanisms are independently manipulable.

**The parent model's empirical support is itself mixed, which makes this a live question.** Miller & Chen (2004, *AMJ* 47(1):105–115) tested the March–Shapira model on organisational data and found the *sizes* but not the *signs* of effects differed across performance categories, with poorly performing organisations showing **increased** risk as they neared bankruptcy `[E]` — partially contrary to the survival-focus prediction. The two-reference-point structure is established; its behavioural consequences are not settled.

**It has a natural null.** Viability theory's inertia principle predicts minimal, targeted correction (§3.1). Threat-rigidity predicts disproportionate narrowing. Opposite predictions about one observable.

**It is falsifiable at the level of the whole project** (§2.5).

**What this question is not.** It is not a claim that the aspiration/survival dissociation is novel. It is not (§31.2, §36.8).

## 2.3 Sub-questions

| ID | Question |
|---|---|
| SQ1 | Does the modelled firm exhibit repertoire narrowing under threat, and under what threat parameters? |
| SQ2 | Does availability of constraint-shaping actions reduce, eliminate, or amplify rigidity? |
| SQ3 | How does concentration of resource dependence moderate threat response? |
| SQ4 | Under rivalry, does one firm's constraint-shaping impose externalities on rivals' viability kernels? |
| **SQ5** | **Do shortfall and viability proximity affect search breadth with opposite signs, and does the shape of the combined relationship distinguish simultaneous operation from attention-switching?** |

## 2.4 Hypotheses

All `[S]` — this project's conjectures.

| ID | Hypothesis | Primary DV |
|---|---|---|
| **H1a** | Holding margin $h$ constant, increasing shortfall $\varsigma$ **increases** search width $w_{\text{eff}}$ and repertoire entropy $H_{\text{rep}}$. *(BTOF direction)* | `search_width`, `repertoire_entropy` |
| **H1b** | Holding shortfall $\varsigma$ constant, decreasing margin $h$ **decreases** them. *(Threat-rigidity direction)* | `search_width`, `repertoire_entropy` |
| **H1c** | Where $h$ and $\varsigma$ covary endogenously, the relationship is **non-monotonic**; a smooth inverted-U indicates simultaneous operation, a discontinuity indicates attention-switching. | `repertoire_entropy` |
| H2 | At matched $h$, threat *novelty* produces greater narrowing than threat *magnitude*. | `repertoire_entropy` |
| H3 | Shaping availability reduces narrowing when shaping lag < time-to-boundary, and increases it when lag > time-to-boundary. | `repertoire_entropy`, `survival_time` |
| H4 | Narrowing improves survival under low-novelty threats and impairs it under high-novelty threats. | `survival_time` |
| H5 | Under rivalry, successful constraint-shaping by one firm contracts rivals' viability kernels. | `kernel_volume_delta` |

**H1c is primary.** It discriminates between formal structures rather than merely confirming a known direction.

## 2.5 Falsification and kill criterion

**H1a and H1b are jointly load-bearing.** If both do not hold across the pre-registered sweep at ≥200 seeds per cell, the dissociation does not operate in this model. Because it is an established result elsewhere `[E]`, joint failure indicates either a model defect or a genuine boundary condition on the established finding. **Either is substantive, but they MUST be distinguished before publication.**

**VT-8 is a precondition, not an outcome.** If $h$ and $\varsigma$ cannot be independently manipulated (§25.4), H1a and H1b are true by construction and E1 is void regardless of results.

**Kill criterion (binding).** If at the end of Phase 3 the platform cannot produce *either* support for H1a/H1b/H1c *or* an interpretable, publishable negative result, the project MUST stop, publish the negative result and the platform, and not proceed to Phase 4. This exists to make stopping an anticipated outcome rather than an admission of failure.

---

# 3. Theoretical foundations

Three pillars, chosen because they compose without contradiction and each supplies what the others lack. §3.4 states deliberate exclusions.

## 3.1 Pillar 1 — Viability theory (the constraint formalism)

**Source:** Aubin, *Viability Theory* (Birkhäuser, 1991); *Dynamic Economic Theory: A Viability Approach* (Springer, 1997); Aubin, Bayen & Saint-Pierre, *Viability Theory: New Directions* (Springer, 2nd ed., 2011). `[E]`

**Supplies:** a rigorous, computable definition of "cannot do whatever it wants."

Viability theory studies dynamical systems under constraints that must hold at every instant, asking which states admit at least one trajectory respecting them indefinitely. Three features make it correct here rather than optimal control:

1. **No objective function.** It seeks survivable trajectories, not optimal ones `[E]` — matching the behavioural view and avoiding a smuggled maximand.
2. **Set-valued.** The output is a *set* of viable states — the natural representation of room for manoeuvre.
3. **The inertia principle.** Controls change only when required to maintain viability `[E]`. A formal, testable model of "do nothing until the boundary approaches," and this project's null against threat-rigidity.

**Borrowed:** viability kernel, inertia principle, backward fixed-point computation.
**Not borrowed:** continuous-time differential-inclusion machinery (FIRMA is discrete-time, §6.2); the sustainability/resource-management application domain.

## 3.2 Pillar 2 — Resource dependence theory (content of threat and response)

**Source:** Pfeffer & Salancik, *The External Control of Organizations* (Harper & Row, 1978; Stanford Business Classics, 2003). `[E]`

**Supplies:** a non-anthropomorphic account of what a firm "wants" and what counts as threat. Organisations are not autonomous but depend on their environment for critical resources; control over a resource is the basis of power over the depender; much organisational behaviour is explicable as dependence reduction `[E]`. It supplies a documented action repertoire — merger, diversification, alliance, co-optation, political action — rather than requiring one be invented (§11.3).

**Borrowed:** dependence as a measurable quantity (§13.1); the dependence-reduction repertoire; the outside-in framing of threat.
**Not borrowed:** the symbolic/legitimacy account of executive succession; the managerial-prescription layer.

**Terminological warning:** RDT's "power" is a relational property of resource control, not a psychological or political construct. FIRMA uses it only in that narrow sense.

## 3.3 Pillar 3 — Behavioural theory of the firm (the decision mechanism)

**Source:** Cyert & March, *A Behavioral Theory of the Firm* (Prentice-Hall, 1963; Blackwell 2nd ed. 1992); Simon, *Administrative Behavior* (Macmillan, 1947). `[E]`

**Supplies:** how the firm decides, without assuming optimisation or foresight — **satisficing** against aspirations; **sequential attention** to goals; **adaptive aspirations**; **problemistic search** triggered by shortfall `[E]`.

**Borrowed:** the aspiration/satisficing rule (§12.3); goal multiplicity; shortfall-triggered search.
**Not borrowed (in MVP):** the coalitional account in which the firm's goal emerges from internal bargaining among subunits. **Deferred extension** (§26, Phase 5), not a rejection. See §8.4 and §33.1 for the acknowledged cost.

## 3.4 Deliberate exclusions

| Excluded | Reason | Status |
|---|---|---|
| **Organism metaphor** (homeostasis, firm-as-living-being) | Prejudges the conclusion; well-critiqued as sliding from description into prescription (Morgan, *Images of Organization*) `[E]` | Permanently excluded as framing. The *substance* — self-preservation, threat response — is retained via Pillars 1–2. |
| **Genetic/vertical inheritance of firm behaviour** | Organisational transmission is observably horizontal and Lamarckian. Whether an abstracted Darwinian account nonetheless applies is live and unresolved (Nelson & Winter 1982; Hodgson & Knudsen's generalised Darwinism; critics) `[P]` | Excluded as **default**. Available as optional plugin (§20.6). **FIRMA takes no position in that dispute.** |
| **Optimisation / equilibrium selection** | Contradicts Pillars 1 and 3 | Excluded |
| **Neural or learned controllers** | Destroys the ablation design supplying internal validity; reward specification becomes a hidden theory | Excluded from MVP. §33.2 |
| **Population-level emergence of firms from workers** | Different question, different level (Axtell's territory, §31.3) | Out of scope |

## 3.5 Academic framing

FIRMA is **computational organisation design theory**: agent-based theory building as positioned by Davis, Eisenhardt & Bingham (2007, *AMR* 32(2):480–499), who locate simulation between inductive theory-creating methods and statistical theory-testing, with internal validity and facility with longitudinal and nonlinear phenomena as characteristic strengths `[E]`.

It is **not** evolutionary organisation theory. It is **not** artificial life.

---

# 4. What FIRMA can and cannot establish

## 4.1 Claim tiers

Every statement produced using FIRMA MUST be classified into exactly one tier, identifiable from its wording.

| Tier | Form | Evidence | In publication |
|---|---|---|---|
| **1 — Model claim** | "Under assumptions *A*, configuration *R*, parameters *θ*, across *n* seeds, mechanism *ρ* produced outcome *Y* with effect size *δ*." | Run set, manifest, ablation | Yes, always |
| **2 — Conditional prediction** | "If a firm satisfies the conditions preserved in contract §35.x, expect *Y* in direction *d*." | Tier 1 + translation contract. **Direction only, never magnitude** | Yes, with contract cited |
| **3 — Empirical claim** | "Firms do *Y*." | Independent empirical data | **Never from FIRMA alone** |

## 4.2 Permanent limits

Not solvable by more compute.

1. **Necessity.** Showing a mechanism *can* produce an outcome never shows it *does*.
2. **External validity.** No degree of internal validity produces it. Zero.
3. **Effect magnitudes.** Model parameters have no empirical units.
4. **Freedom from encoding artefacts.** Discretisation, tie-breaking, update order generate spurious regularities. §25.5 reduces this risk without eliminating it.
5. **Anything requiring foresight or intent.** FIRMA's agents do not anticipate, deceive, or reinterpret. Real firms do.

## 4.3 The binding constraint

**Construct validity, not computation.** If a finding appears under one operationalisation of "rigidity" and vanishes under another reasonable one, there is no finding.

**Requirement:** every core construct MUST have ≥2 non-equivalent operationalisations, and every headline result MUST be reported under all of them (§14.5).

## 4.4 Translation contracts

Any Tier 2 claim requires a contract (§35) with six fields: *model mechanism*, *abstract mechanism*, *organisational construct*, *preserved*, *discarded*, *introduced*, *known non-transfer + empirical scope condition*.

Contracts are content-hashed and MUST be cited by ID in any publication using them.

---

# 5. Terminology (binding)

## 5.1 Core terms

| Term | Definition in FIRMA | Explicitly NOT |
|---|---|---|
| **Firm-agent** | Entity with state, constraint set, goal structure, action repertoire | Not a person, organism, or necessarily a company |
| **Constraint set** *K* | Subset of state space in which the agent may exist | Not a preference or soft penalty. Violation means death |
| **Viability kernel** *Viab(K)* | Subset of *K* from which some admissible trajectory remains in *K* indefinitely | Not the optimal or profitable set |
| **Viability margin** *h(x)* | Scalar distance to nearest constraint violation | Not fitness. Not utility |
| **Threat** | Parameterised exogenous perturbation to state, constraints, or environment | Not subjective perception. Agents have no fear |
| **Rigidity** | Reduction in diversity of actions used, and/or concentration of action selection, vs. matched counterfactual | Not stubbornness. Not a psychological state |
| **Constraint-shaping action** | Action whose effect is on the parameters defining *K* | Not cheating. A modelled, costly, uncertain capability |
| **Dependence** | Concentration of critical-resource inflows across sources | Not emotional or contractual dependency |
| **Aspiration** | Per-goal reference level for judging outcomes satisfactory | Not a target to maximise |
| **Intervention** | `do(·)` on state, rules, or parameters at a specified tick | Not a policy recommendation |
| **Run** | One execution of one configuration with one seed | — |
| **Experiment** | Set of runs defined by one `ExperimentSpec`, including its analysis plan | — |

## 5.2 Terms that MUST NOT be conflated

| | |
|---|---|
| Viability ≠ | Profitability, optimality, fitness |
| Rigidity ≠ | Stability, persistence, inertia |
| Inertia (viability-theoretic: change control only when needed) ≠ | Inertia (organisational-ecology: structural resistance to change) |
| Threat ≠ | Risk, uncertainty, volatility |
| Dependence ≠ | Interdependence, coupling |
| Constraint ≠ | Cost, penalty, preference |
| Adaptation ≠ | Learning, evolution, improvement |
| Randomness ≠ | Entropy, uncertainty, noise |
| Repertoire entropy ≠ | Thermodynamic entropy (there is none in this model) |
| Search breadth ≠ | Risk-taking (**critical — see §31.2**) |

**The word "entropy" in any FIRMA output MUST carry a qualifier naming which entropy.**

## 5.3 Forbidden framings

MUST NOT appear in code comments, documentation, or publications:

- "The firm wants / fears / decides to protect itself" — anthropomorphism. Say: "the agent's action selection under low viability margin."
- "The firm evolved to…" — implies a selection process the MVP does not model.
- "Emergent" without naming the operational sense and citing its test.
- "The simulation shows that firms…" — a Tier 3 claim from Tier 1 evidence.
- "Organism," "homeostasis," "immune response," "metabolism" describing firm behaviour.

---

# PART II — FORMAL MODEL

# 6. Mathematical core

## 6.1 Types

| Name | Type | Notes |
|---|---|---|
| `Tick` | `u64` | One fiscal quarter (§6.2) |
| `AgentId` | `u64` | Monotonic, never reused |
| Resource quantity | `i64` | Integer; fixed-point scale 10⁻³ where fractions needed (§34.4) |
| Real state | `f64` | Non-conserved quantities only |
| Probability | `f64` ∈ [0,1] | |

## 6.2 Time

Discrete-time with explicit phases. Base tick τ = one **fiscal quarter** by default, configurable. `[D]`

Rationale: constraint-shaping (lobbying, alliance formation, regulatory response) operates on quarters-to-years. A finer tick multiplies compute for no gain.

Continuous-time is **not supported** and MUST NOT be added without an ADR — it would break the deterministic phase scheduler (§19.3).

## 6.3 Global state

$$X_t = \big(\lbrace \mathbf{x}_{i,t}\rbrace _{i \in I_t}, \boldsymbol{\theta}_t, \mathbf{e}_t, G_t\big)$$

| Symbol | Meaning |
|---|---|
| $I_t$ | Live agent IDs |
| $\mathbf{x}_{i,t}$ | Agent state (§8.1) |
| $\boldsymbol{\theta}_t$ | Constraint parameters (§8.2) — what shaping moves |
| $\mathbf{e}_t$ | Environment: resources, regulatory regime, market conditions |
| $G_t$ | Typed, weighted, directed relation graph |

## 6.4 Dynamics

$$X_{t+1} = \Pi\Big(X_t, \textstyle\bigsqcup_{p \in P} \bigsqcup_{\rho \in R_p} \rho\big(\mathrm{view}_{\rho}(X_t^{(p)}), \kappa(\text{seed}, \rho, p, t, i)\big)\Big)$$

| Symbol | Meaning |
|---|---|
| $P$ | Ordered phase sequence (§10.1) |
| $R_p$ | Rule plugins registered to phase *p* |
| $\rho$ | A rule: pure function from read-only view to proposed deltas |
| $\kappa$ | RNG key derivation (§21.2) |
| $\Pi$ | Reconciliation: applies deltas under conflict resolution, enforces invariants |
| $X_t^{(p)}$ | State at start of phase *p* |

**Rules propose; they do not mutate.** This is the central architectural invariant (§19.4) and why ordering within a phase cannot affect results.

## 6.5 Intervention algebra

$$X^{do(a)}_{T} = a(X_T), \qquad \text{evolution thereafter under identical } \kappa$$

| Operator | Effect |
|---|---|
| `set_state(i, component, value)` | Direct state modification |
| `set_param(path, value)` | Parameter change |
| `remove_rule(id)` / `add_rule(spec)` | Mechanism ablation or addition |
| `freeze_rule(id)` | Rule loaded but emits no deltas — distinguishes "absent" from "present but inactive" |
| `apply_shock(spec)` | Inject a threat (§13.2) |
| `remove_agent(i)` / `add_agent(spec)` | Population manipulation |

## 6.6 Interventional construct definition — the measurement rule

**Every FIRMA construct MUST be defined as a functional of a matched trajectory pair:**

$$\Phi_c = F_c\Big(\lbrace X_t\rbrace _{t \ge T}, \lbrace X^{do(a)}_t\rbrace _{t \ge T}\Big)$$

Not as a statistic of a single trajectory. Three deliberate consequences:

1. Structurally prevents narrative interpretation of single runs.
2. Requires kernel-level forking with common random numbers (§21.2) — a hard architectural requirement, not a nicety.
3. Makes effect estimates paired, drastically reducing variance.

**Exception:** descriptive monitoring statistics (population count, tick rate) need not be interventional. They are labelled `descriptive` and MUST NOT appear as dependent variables in any hypothesis test.

---

# 7. Ontology

## 7.1 Design rule

> A concept is a **primitive** only if it cannot be derived from lower primitives without assuming the phenomenon under study.

## 7.2 The nine primitives

Adding a tenth requires an ADR.

| # | Primitive | Definition | Why primitive |
|---|---|---|---|
| 1 | **Agent** | Identified, persistent state-bearer with stable `AgentId`, birth tick, optional death tick | Something must bear state; nothing presumes "firm" |
| 2 | **State component** | Typed, named slice of agent state; agents are component bags | Enables heterogeneous agents without inheritance hierarchies |
| 3 | **Resource** | Conserved, transferable, non-negative integer with a ledger | Conservation is the strongest correctness invariant (§25.3) |
| 4 | **Constraint** | $g_j(\mathbf{x},\boldsymbol{\theta}) \le 0$ with scale factor and violation semantic | Core object of the theory |
| 5 | **Action** | Named, costed, admissible transformation proposal | Unit of firm behaviour |
| 6 | **Relation** | Typed directed edge with weight and age | Dependence and rivalry are relational |
| 7 | **Shock** | Parameterised exogenous perturbation | Threat must be manipulable, not emergent, to be an IV |
| 8 | **Rule** | Pure `(view, rng_key) -> Vec<Delta>` | Unit of theoretical assumption |
| 9 | **Intervention** | `do(·)` with a tick stamp | Required by §6.6 |

## 7.3 Derived, never primitive

Firm, industry, market, alliance, coalition, hierarchy, capability, legitimacy, power, threat-perception, rigidity, adaptation, strategy, routine, fitness.

**"Firm" is derived.** There is no `Firm` type in the kernel. A firm is an agent *configured with* firm-like components, constraints, and actions. This is what makes §7.5 possible.

**"Fitness" is forbidden.** No `fitness` field anywhere. Performance measures are computed post hoc from realised outcomes over an explicitly named horizon. Cross-agent or cross-time comparison at mismatched horizons is a known source of invalid inference.

## 7.4 Heterogeneity by composition

Agents are **component bags**. `{Ledger, Constraints}` is a passive resource holder; add `{Goals, ActionRepertoire}` for a decision-maker; add `{ShapingCapability}` to act on constraints; add `{Strategic}` to the regulator and the environment becomes an adversary.

Entity-component pattern, adopted for a scientific reason rather than a performance one.

## 7.5 The focal firm is not special

**The kernel MUST NOT contain any concept of "the firm under study."** Every firm-agent — focal and rival — is the same primitive on the same code path.

| Configuration | How achieved |
|---|---|
| Single firm vs. passive environment | `n_firms = 1`, regulator lacks `Strategic` |
| Multi-firm rivalry | `n_firms = k`, shared pools, rivalry relations |
| Single firm now, rivals later | Same binary, different config |
| Strategic regulator | Give regulator `{Goals, ActionRepertoire, Strategic}` |
| Nested units (future) | Agents holding `MemberOf` relations |

Analysis-time focus on one agent is a *reporting* concern (`analysis.focal_agent`), never a modelling one.

## 7.6 Space

**Non-spatial by default** (§34.7). Locality is supplied by a plugin behind one interface — `neighbours(i, t) -> Set<AgentId>` — with implementations `wellmixed`, `network`, `tag_affinity`, and (Phase 4) `metric_space`. Adding geography later is a plugin, not a rewrite.

---

# 8. State

## 8.1 Firm-agent state

$$\mathbf{x}_{i,t} = \big(r^{L}_{i,t}, r^{I}_{i,t}, c_{i,t}, q_{i,t}\big) \quad\text{(constraint-carrying, } d=4\text{)}$$

| Symbol | Name | Type | Domain | Meaning |
|---|---|---|---|---|
| $r^L$ | Liquid capital | `i64` | $[0, R^L_{\max}]$ | Spendable. Zero ⇒ insolvency |
| $r^I$ | Input stock | `i64` | $[0, R^I_{\max}]$ | Production input, storage-capped |
| $c$ | Capability | `f64` | $[0,1]$ | Determines in-scope activities |
| $q$ | Outstanding obligation | `i64` | $[0, Q_{\max}]$ | Undelivered contracted quantity |

**Auxiliary state** — not constraint-carrying, does not count toward $d \le 4$ (§9.3):

| Symbol | Name | Meaning |
|---|---|---|
| $\lambda_{i,t}$ | Legitimacy `f64` ∈ [0,1] | Modulates shaping success (§11.3) |
| $u_{i,t}$ | Regulated-activity intensity `f64` ≥ 0 | Trailing-window mean of regulated production |
| $\mathbf{A}_{i,t}$ | Aspirations `f64`³ | One per goal (§12.1) |
| $M_{i,t}$ | Memory, length $L_M$ | Past shocks and outcomes |
| $\Lambda_{i,t}$ | Lagged-effect queue | `(maturity_tick, Effect)` pairs |
| $W_{i,t}$ | Action window, length $L_W$ | Last $L_W$ actions. Source of R1, R2, R3 |

> **On $u$:** a *derived* trailing statistic of past actions, stored for efficiency. $g_2$ is a function of $(u, \theta^{\text{limit}})$, and $u$ is fully determined by $W_{i,t}$ — hence $d=4$, not 5. An implementation MAY recompute $u$ from $W$; results MUST be identical.

## 8.2 Constraint parameters

$$\boldsymbol{\theta}_t = \big(\theta^{\text{limit}}_t, \theta^{\text{cap}}_t, \theta^{Q}_t\big)$$

| Symbol | Meaning | Moved by |
|---|---|---|
| $\theta^{\text{limit}}$ | Permitted regulated-activity intensity | Regulatory shocks; `lobby` |
| $\theta^{\text{cap}}$ | Minimum capability for regulated production | Regulatory shocks |
| $\theta^{Q}$ | Maximum permitted outstanding obligation | `contract` |

**Global by default** (one regime for all firms). Per-firm variant is Phase 4. This is the object making the viability kernel endogenous (§9.4).

## 8.3 Environment and relations

$$\mathbf{e}_t = \big(\pi^{I}_t, \pi^{O}_t, \Sigma_t\big)$$

where $\pi^{I}$ is input price, $\pi^{O}$ output price, and $\Sigma_t$ the set of active shocks.

$G_t$: edges $(i,j,\ell,w,\text{age})$ with $\ell \in \lbrace \texttt{supply}, \texttt{alliance}, \texttt{rivalry}\rbrace$. `supply` edges carry realised inflows used for dependence (§13.1).

## 8.4 Components and the unitary-firm decision

| Component | Contents | MVP |
|---|---|---|
| `Ledger` | Integer stocks, transaction log | ✓ |
| `Capability` | Levels by domain, decay rate | ✓ |
| `Constraints` | Active constraint instances | ✓ |
| `Goals` | Aspirations, realised values, update rates | ✓ |
| `ActionRepertoire` | Admissible actions, costs, lags, success models | ✓ |
| `ShapingCapability` | Shaping actions available, track record | ✓ |
| `Relations` | Typed edges | ✓ |
| `Memory` | Bounded ring buffer | ✓ |
| `Attention` | Current focus, search state | ✓ |
| `Strategic` | Marker: runs a decision policy | ✓ |
| `MemberOf`, `Subunits` | Higher-order membership, internal coalition | ✗ Phase 5 |

**The MVP firm is unitary** — one decision process resolving multiple goals (§34.6).

**The acknowledged cost, which MUST be stated in any publication:** treating the firm as unitary discards precisely the mechanism Cyert & March identified as *generating* organisational goals. Threat-rigidity in real organisations may be substantially coalitional — centralisation *is* a redistribution of internal decision rights, which a unitary model cannot represent. **This is the largest known validity gap in the MVP** (§33.1).

---

# 9. Constraints and viability

## 9.1 The four constraints

| ID | $g_j$ | Violation semantic |
|---|---|---|
| `solvency` | $g_1 = -r^L$ | **Death.** Agent removed at end of `enforce` |
| `compliance` | $g_2 = u - \theta^{\text{limit}}$ | **Graduated.** First: penalty $P_c$, $\lambda \mathrel{-}= \delta_{\lambda}$. Second within $T_c$ ticks: death |
| `scope` | $g_3 = \theta^{\text{cap}} - c$ | **Admissibility.** Regulated production inadmissible while violated. Never lethal |
| `obligation` | $g_4 = q - \theta^{Q}$ | **Relational.** All `supply` edges severed; penalty $P_q$ |

**`[D]`** The four semantics are deliberately different. Making all lethal would collapse the model to a single survival constraint and destroy the distinction between kinds of pressure. These differences are theoretical assumptions, recorded as such.

## 9.2 Scale factors and the viability margin

$$h(\mathbf{x}, \boldsymbol{\theta}) = -\max_j \frac{g_j(\mathbf{x}, \boldsymbol{\theta})}{s_j}$$

| $j$ | $s_j$ | Default | Rationale |
|---|---|---|---|
| 1 | $s_L$ | 100 | Median start-of-run liquid capital |
| 2 | $s_u$ | 1.0 | Already a normalised intensity scale |
| 3 | $s_c$ | 0.5 | Half the capability range |
| 4 | $s_q$ | 50 | Half of $Q_{\max}$ |

$h > 0$ ⇒ all constraints satisfied.

**Mandatory caveat, repeated wherever $h$ is used:** $h$ measures distance to *current* infeasibility; the viability kernel measures possibility of *indefinite* survival. **A state can have large $h$ and lie outside the kernel.** VT-3 quantifies the divergence; §15.2 demonstrates it concretely.

## 9.3 The viability kernel and its dimensionality limit

$$K^{(0)} = K(\boldsymbol{\theta}), \qquad K^{(n+1)} = \big\lbrace \mathbf{x} \in K^{(n)} : \exists \mathbf{u} \in U(\mathbf{x}), f(\mathbf{x},\mathbf{u}) \in K^{(n)}\big\rbrace $$

$$\mathrm{Viab}(K) = \lim_{n\to\infty} K^{(n)} = \bigcap_{n \ge 0} K^{(n)}$$

Monotone decreasing on a finite grid; terminates when $K^{(n+1)} = K^{(n)}$ `[E]`.

**The dimensionality limit — read before designing any model.** Grid computation costs $O(N_{\text{grid}}^{d}\cdot|U|)$ per iteration. **Exponential in *d*, and the hardest constraint on model design.**

| *d* | Feasibility | Use |
|---|---|---|
| 2–4 | Comfortable | **MVP MUST live here.** Exact kernel is ground truth |
| 5–6 | Expensive | Phase 3+, coarse grids, resolution sensitivity reported |
| ≥7 | Infeasible exactly | Margin proxy only |

**The MVP state space MUST have $d \le 4$ for kernel-carrying components.** Additional state may exist but MUST NOT enter $g_j$.

**Implementation:** grid at unit steps for integers, 21 steps of 0.05 for $c$; resolution is configurable and AT-2 tests convergence. The transition $f$ used for kernel computation is the **deterministic core** of §11 with stochastic terms at expectation — an approximation that MUST be recorded as such. Store as a bitset; $\text{volume} = |K|/|\text{grid}|$.

## 9.4 The endogenous constraint set — the formal contribution

$$\boldsymbol{\theta}_{t+1} = \gamma\big(\boldsymbol{\theta}_t, \lbrace \mathbf{a}^{c}_{i,t}\rbrace _{i}, \boldsymbol{\zeta}_t\big), \qquad \mathcal{V}_t = \mathrm{Viab}\big(K(\boldsymbol{\theta}_t)\big)$$

**The endogenous viability kernel is what FIRMA studies that existing models do not.** Standard treatments hold $K$ fixed. Here, a firm near its boundary has two categorically different options: move within $K$, or move $K$.

**Three properties MUST hold in any implementation of $\gamma$:**

1. **Costly** — shaping consumes resources that could have funded operations.
2. **Lagged** — effects arrive after $\Delta_c \ge 1$ ticks, drawn from a declared distribution.
3. **Uncertain** — success is probabilistic, with $p_{\max} < 1$ strictly.

**Without all three, shaping is a free escape hatch and the model is worthless.**

---

# 10. Dynamics and phases

## 10.1 Phase order

| # | Phase | Content |
|---|---|---|
| 1 | `environment` | Prices update; scheduled shocks fire |
| 2 | `observe` | Agents build observations (§12.2) |
| 3 | `decide` | Decision procedure (§12.3) selects one action per firm |
| 4 | `act_market` | Market actions emit deltas |
| 5 | `act_shaping` | Shaping actions emit deltas, enqueue lagged effects |
| 6 | `resolve_lagged` | Effects with `maturity_tick == t` fire |
| 7 | `constrain` | $\boldsymbol{\theta}$ updates; $u$ and $h$ recomputed |
| 8 | `enforce` | Violations processed (§9.1); deaths recorded |
| 9 | `record` | Aspirations update; events flushed |

Within a phase, all rules read state as of phase start (Jacobi). Ordering within a phase cannot affect results.

---

# 11. Actions

Nine discrete actions. **The canonical order below is normative** — it defines scan order (§12.4) and RNG `purpose_tag` indices.

## 11.1 Market actions

| # | Action | Precondition | Effect (deterministic core) | Cost |
|---|---|---|---|---|
| 0 | `hold` | always | none | 0 |
| 1 | `produce_ordinary` | $r^I \ge 1$ | $r^I \mathrel{-}= 1$; $r^L \mathrel{+}= \pi^O y_O(c)$ | 0 |
| 2 | `produce_regulated` | $r^I \ge 1$, $c \ge \theta^{\text{cap}}$ | $r^I \mathrel{-}= 1$; $r^L \mathrel{+}= \pi^O y_R(c)$; contributes to $u$ | 0 |
| 3 | `acquire_input` | $r^L \ge \pi^I$, $r^I < R^I_{\max}$ | $r^L \mathrel{-}= \pi^I$; $r^I \mathrel{+}= 1$ | $\pi^I$ |
| 4 | `invest_capability` | $r^L \ge \kappa_c$ | lagged: $c \mathrel{+}= \delta_c$ after $\Delta_{\text{cap}}$ | $\kappa_c$ |
| 5 | `deliver` | $q \ge 1$, $r^I \ge 1$ | $q \mathrel{-}= 1$; $r^I \mathrel{-}= 1$ | 0 |

$$y_O(c) = \lfloor y_0(1+\eta c)\rfloor, \qquad y_R(c) = \lfloor y_0(1+\eta c)(1+\gamma_R)\rfloor$$

Regulated production yields more ($\gamma_R > 0$) but consumes compliance headroom. **This tradeoff is the model's core operating tension** and is what makes `compliance` bind rather than decorate.

## 11.2 Constraint-shaping actions

| # | Action | Precondition | Cost (immediate) | Lag | On success | On failure |
|---|---|---|---|---|---|---|
| 6 | `lobby` | $r^L \ge \kappa_{\ell}$ | $\kappa_{\ell}$ | $\Delta_{\ell}$ | $\theta^{\text{limit}} \mathrel{+}= \delta_{\theta}$ | nothing |
| 7 | `contract` | $r^L \ge \kappa_k$, has `supply` partner | $\kappa_k$ | $\Delta_k$ | $\theta^{Q} \mathrel{+}= \delta_Q$; $q \mathrel{+}= q_0$; edge weight fixed | nothing |
| 8 | `diversify` | $r^L \ge \kappa_d$ | $\kappa_d$ | $\Delta_d$ | new `supply` edge to a fresh source | nothing |

$$p_{\text{success}}(a) = \mathrm{clip}\left(
p^{a}_{0} + b_{\lambda} \lambda_i + b_{\kappa} \frac{\text{spend} - \kappa_a}{\kappa_a},
 0, p^{a}_{\max}
\right)$$

where $\mathrm{clip}(x, l, u) = \min(\max(x, l), u)$, $b_{\lambda}$ and $b_{\kappa}$ are non-negative coefficients on legitimacy and on overspend beyond the minimum commitment, and $p^{a}_{\max} < 1$ strictly.

> **Notation warning.** These coefficients are $b_{\lambda}, b_{\kappa}$, **not** $\beta_{\lambda}, \beta_{\kappa}$. The symbol $\beta$ is reserved throughout this manual for the **narrowing sharpness** parameter (§12.3), which is the project's primary independent variable.

Lag is **drawn**, not fixed: $\Delta_a \sim \text{Uniform}\lbrace \Delta^{\min}_a,\dots,\Delta^{\max}_a\rbrace$ from the `mechanism` stream, `purpose_tag = "shaping_lag"`.

`contract` is deliberately double-edged: it raises $\theta^Q$ while adding $q_0$ to obligation and fixing a supply edge (raising dependence). **It should not be uniformly beneficial; if implementation makes it so, the parameters are wrong.**

## 11.3 Mandatory properties (restating §9.4 as an implementation requirement)

1. **Cost paid at commitment, not at success.** Failed lobbying still consumes resources. Refund-on-failure implementations are non-conforming.
2. **Lag ≥ 1 tick**, drawn from a declared distribution.
3. **Success probabilistic**, $p_{\max} < 1$ strictly, model declared in plugin docs and exposed in config.

**Test VT-7 MUST verify all three for every registered shaping plugin.**

## 11.4 Admissibility

$U(\mathbf{x})$ is computed by the `Admissibility` service: an action is admissible if in repertoire, affordable, and its immediate effect does not violate a hard constraint. **Inadmissible actions are still counted** for repertoire-entropy purposes as unavailable — the distinction between "did not choose" and "could not choose" is the core of rigidity measurement.

---

# 12. Goals, observation, decision

## 12.1 Goals and aspirations

| $j$ | Goal | Realised $v_j$ | Shortfall $\varsigma_j$ |
|---|---|---|---|
| 1 | Capital growth | $r^L_{t}-r^L_{t-1}$ | $A_1 - v_1$ |
| 2 | Capability | $c_t$ | $A_2 - v_2$ |
| 3 | Obligation clearance | $-q_t$ | $A_3 - v_3$ |

$$A_{j,t+1} = A_{j,t} + \alpha (v_{j,t}-A_{j,t}), \qquad \alpha \in (0,1)$$

Standard behavioural formulation `[E]`. Aspirations fall after sustained failure, so "satisfactory" is history-dependent. **Intended.**

## 12.2 Observation

Own state observed exactly. Environment and $\boldsymbol{\theta}$ through the `Observation` plugin: `full`, `noisy(σ)`, or `delayed(k)`.

**`[D]` Modelling assumption, stated because it bounds every claim:** the firm observes its own $h$ **without error**. Real managers act on *perceived* proximity, which is biased. Recorded as a known non-transfer in §35.1.

## 12.3 The decision procedure

Executed in `decide` by every firm holding `Strategic`. **This is the mechanism under test.**

**Step 1 — Evaluate.** Compute $h_{i,t}$ (§9.2) and all $\varsigma_j$ (§12.1).

**Step 2 — Attend.**
```
if h < h_crit: focus := SURVIVAL
else if max_j ς_j > 0: focus := GOAL(argmax_j ς_j) # ties → lowest j
else: focus := NONE
```

**Step 3 — Narrowing (the rigidity mechanism).**

$$\psi(h) = 1 \qquad \text{when } h \ge h_{\text{crit}}$$

$$\psi(h) = \left( \frac{\max(h, 0)}{h_{\text{crit}}} \right)^{\beta} \qquad \text{when } 0 \le h < h_{\text{crit}}$$

$$w_{\text{eff}} = \max\big(1, \lceil w_{\max} \cdot \psi(h) \rceil\big)$$

**$\beta$ is the primary independent variable.** $\beta=0$ ⇒ $\psi\equiv1$ ⇒ no narrowing: **the null arm.**

> **Why this is not question-begging.** $\beta$ makes narrowing *possible*, not rigidity *inevitable*. Narrowing the search does not by itself narrow the *realised* repertoire — a firm scanning three actions may still use all three across ticks. Whether search narrowing (R2) translates into repertoire concentration (R1) and shaping abandonment (R3) is an outcome of the dynamics, not an assumption. **H1b is a claim about that translation and it can fail.**

**Step 4 — Scan order.** Focus-dependent priority. This is what produces R3.

| Focus | Priority order (action indices) |
|---|---|
| `SURVIVAL` | 1, 3, 5, 2, 0, 4, 6, 7, 8 |
| `GOAL(1)` capital | 2, 1, 3, 6, 8, 5, 4, 0, 7 |
| `GOAL(2)` capability | 4, 1, 3, 2, 6, 5, 0, 8, 7 |
| `GOAL(3)` obligation | 5, 3, 1, 7, 2, 8, 6, 0, 4 |
| `NONE` | repeat previous action (**inertia principle**) |

Shaping actions (6,7,8) sit late under `SURVIVAL`. With small $w_{\text{eff}}$ they are never reached — **R3 emerges from the interaction of Step 3 and this table rather than being imposed.**

**Step 5 — Satisficing selection.**
```
scanned := 0
for a in priority_order(focus):
 if scanned >= w_eff: break
 if not admissible(a, x, θ, e): continue # inadmissible: not counted
 scanned += 1
 if satisfices(a, focus): return a
return priority_order(focus)[first admissible] # fallback
```

**Two rules that MUST NOT be optimised away:**

1. **First satisficing, not argmax.** Replacing with maximisation converts the model from behavioural to rational-choice and invalidates Pillar 3. A `decision.optimizing` plugin exists for that comparison (E7).
2. **Inadmissible actions do not consume scan budget** (§11.4).

`satisfices(a, focus)`:

| Focus | Test |
|---|---|
| `SURVIVAL` | Expected $h_{t+1}$ under $a$ (deterministic core) $> h_t$ |
| `GOAL(j)` | Expected $\Delta v_j$ under $a$ $\ge \varsigma_j$ |

Expectations use the deterministic core with stochastic terms at expectation. **The firm does not simulate; it applies a one-step lookahead.** Bounded rationality in the literal sense.

---

# 13. Dependence and shocks

## 13.1 Dependence

Over a trailing window $L_D$, with $w^k_{ij}$ the realised inflow of resource $k$ to firm $i$ from source $j$:

$$D_i^k = \frac{\sum_j (w^k_{ij})^2}{\big(\sum_j w^k_{ij}\big)^2}
\qquad
D_i = \sum_k \omega_k D_i^k
\qquad
\omega_k = \frac{\big|\partial h / \partial r^k\big|}{\sum_{k'} \big|\partial h / \partial r^{k'}\big|}$$

$D_i^k \in [1/n, 1]$ for $n$ active sources: $1$ is total dependence on a single source, $1/n$ an even spread. The criticality weights satisfy $\sum_k \omega_k = 1$ by construction.

> **Notation warning.** $\omega_k$ (criticality weight) MUST NOT be written $\varsigma_k$. The symbol $\varsigma$ is reserved throughout this manual for **aspiration shortfall** (§12.1), which is a different quantity and one half of the central dissociation (§2.1). Reusing it here would collide with the project's primary independent variable.

Criticality $\omega_k$ is **derived by finite difference on the margin, not assigned.** Dependence is therefore a *measured consequence* of constraint structure, which is what makes it usable as a dependent variable. If $\sum_j w^k_{ij} = 0$ over the window, $D_i^k := 1$ and the case is logged.

## 13.2 Shock: the formal object

```
Shock {
 id, channel: Resource | Regulatory | Competitive | Reputational
 magnitude: f64
 onset: Tick
 ramp: Instant | Linear(duration) | Exponential(rate)
 persistence: Transient(duration) | Permanent | Recurring(period)
 observability: Full | Delayed(ticks) | Noisy(sigma) | Hidden
 novelty: f64 ∈ [0,1]
 targets: All | Set<AgentId> | Predicate
}
```

| Channel | Effect | Constraint stressed |
|---|---|---|
| `Resource` | $\pi^I \mathrel{+}= m$, or availability reduced | `solvency` |
| `Regulatory` | $\theta^{\text{limit}} \mathrel{-}= m$, or $\theta^{\text{cap}} \mathrel{+}= m$ | `compliance` |
| `Competitive` | $\pi^O \mathrel{-}= m$ | `solvency`, `obligation` |
| `Reputational` | $\lambda \mathrel{-}= m$; `supply` weights reduced | `obligation` |

**The regulatory channel is the important one:** it is the *same object* shaping actions manipulate (§9.4). Threat and response act on a shared surface, which makes the two-sided version (Phase 4) coherent.

## 13.3 Novelty — both operationalisations

**N1, memory-relative.** With $\mathbf{z}$ the shock's normalised parameter vector and $M_i$ the memory:
$$N1 = \min\left(1, \frac{\min_{\mathbf{z}'\in M_i}\lVert\mathbf{z}-\mathbf{z}'\rVert_2}{d_{\text{ref}}}\right)$$
Empty memory ⇒ $N1 = 1$.

**N2, repertoire-relative.**
$$N2 = 1 - \frac{|\lbrace a \in A_i : \text{effect}(a,\text{channel}) \ne 0\rbrace |}{|A_i|}$$

These can diverge — a shock may be memory-familiar yet repertoire-unaddressable. **Divergence is a finding, not a nuisance, and MUST be reported (§14.5).**

The 1981 source specifies that functional versus dysfunctional consequences turn on whether the threat is a known, anticipated, or trained condition `[E]` — so novelty is the moderator the thesis itself names, not an embellishment.

---

# 14. Measurement

## 14.1 Principle

Every construct is a functional of a matched trajectory pair (§6.6). Metrics are computed **offline from the event log** (§22.2), never inline. New metrics apply retroactively to all past runs.

## 14.2 Rigidity — three operationalisations

**All three MUST be reported for any rigidity claim.**

**R1 — Repertoire entropy.** Over window $W_{i,t}$, with $p_a$ the frequency of action $a$:
$$H_{\text{rep}} = -\sum_{a: p_a>0} p_a \log_2 p_a \quad\text{[bits]}, \qquad \tilde{H}_{\text{rep}} = \frac{H_{\text{rep}}}{\log_2 |A^{\text{adm}}_{i,t}|}$$

Both MUST be reported with $|A^{\text{used}}|$ and $|A^{\text{adm}}|$ separately. **Entropy alone conflates "used fewer" with "had fewer available."**

**R2 — Search narrowing.** $\overline{w_{\text{eff}}}/w_{\max}$ over the window. Closest to the thesis's information-restriction claim.

**R3 — Shaping abandonment.**
$$\text{SA} = 1 - \frac{\text{capital committed to actions } 6,7,8}{\text{capital committed to all actions}}$$
Zero denominator ⇒ undefined, logged, excluded from that window's aggregate.

## 14.3 Viability metrics

| Metric | Definition | Notes |
|---|---|---|
| `margin` | $h$ per §9.2 | Descriptive, cheap |
| `in_kernel` | Membership in $\mathrm{Viab}(K(\boldsymbol{\theta}_t))$ | Exact; $d\le4$ only |
| `kernel_volume` | Grid measure of $\mathcal{V}_t$ | For H5 |
| `time_to_boundary` | Ticks until $h \le 0$ under continued current action | Forward projection |
| `survival_time` | Ticks to lethal violation; right-censored | Survival methods (§29.3) |

## 14.4 Other constructs

| Construct | Operationalisations |
|---|---|
| Dependence | Herfindahl $D_i$ (§13.1); plus max single-source share |
| Adaptation | Change in `margin` attributable to own action vs. a `hold`-only counterfactual fork |
| Threat severity | Realised drop in `margin`; plus expected `time_to_boundary` reduction |
| Shaping efficacy | Realised $\Delta\boldsymbol{\theta}$ per resource committed vs. no-shaping fork |

## 14.5 The robustness requirement (binding)

**A result is reportable only if it holds under all declared operationalisations of its primary construct.** If it does not, the disagreement *is* the result and MUST be reported as such — never resolved by selecting the favourable measure.

## 14.6 Interventional form

For E1 the fork is the no-shock branch. The reported quantity is the CRN-matched paired difference $\Delta H_{\text{rep}} = H_{\text{rep}}^{\text{factual}} - H_{\text{rep}}^{\text{no-shock}}$, **not the raw level.**

---

# 15. Worked examples (normative)

Computed by hand. **Each becomes a test fixture.** An implementation disagreeing with any of these is wrong.

## 15.1 Example A — Margin computation

State: $r^L=40$, $r^I=12$, $c=0.55$, $q=30$, $u=0.72$.
Parameters: $\theta^{\text{limit}}=0.90$, $\theta^{\text{cap}}=0.40$, $\theta^{Q}=100$.
Scales: $s_L=100$, $s_u=1.0$, $s_c=0.5$, $s_q=50$.

| $j$ | $g_j$ | Value | $g_j/s_j$ |
|---|---|---|---|
| 1 solvency | $-r^L$ | $-40$ | $-0.400$ |
| 2 compliance | $u-\theta^{\text{limit}}$ | $-0.18$ | $-0.180$ |
| 3 scope | $\theta^{\text{cap}}-c$ | $-0.15$ | $-0.300$ |
| 4 obligation | $q-\theta^{Q}$ | $-70$ | $-1.400$ |

$$\max_j(g_j/s_j) = -0.180 \Rightarrow h = 0.180$$

**Binding constraint: `compliance`** — not the one with the largest *raw* slack. Scale factors matter, which is why §9.2 fixes them explicitly.

## 15.2 Example B — Viability kernel, computed by hand

Fixture for VT-1 and VT-2, and the clearest demonstration of why VT-3 exists.

**Setup.** State $(r^L, r^I)$, integers. $K = \lbrace 0 \le r^L \le 4, 0 \le r^I \le 4\rbrace$ — 25 grid points. Exceeding a bound is a *violation*, not clipping. Two actions, with per-tick upkeep of 1 capital folded in:

| Action | Transition | Admissible when |
|---|---|---|
| **P** (produce) | $(r^L+1, r^I-1)$ | $r^L \le 3$, $r^I \ge 1$ |
| **B** (buy) | $(r^L-2, r^I+2)$ | $r^L \ge 2$, $r^I \le 2$ |

**Iteration 1.** States with no action landing in $K^{(0)}$: $(0,0)$ and $(1,0)$ — P blocked by $r^I=0$, B by $r^L<2$. $(4,3)$ and $(4,4)$ — P blocked by $r^L=4$, B by $r^I>2$.
$$K^{(1)} = K^{(0)}\setminus\lbrace (0,0),(1,0),(4,3),(4,4)\rbrace , \quad |K^{(1)}| = 21$$

**Iteration 2.** $(0,1)$: P → $(1,0) \notin K^{(1)}$; B inadmissible. **Fails.** $(3,4)$: P → $(4,3) \notin K^{(1)}$; B inadmissible. **Fails.** All others succeed — e.g. $(2,0)$: B → $(0,2)$ ✓; $(0,2)$: P → $(1,1)$ ✓; $(4,0)$: B → $(2,2)$ ✓.
$$K^{(2)} = K^{(1)}\setminus\lbrace (0,1),(3,4)\rbrace , \quad |K^{(2)}| = 19$$

**Iteration 3.** Every state in $K^{(2)}$ retains an admissible action landing in $K^{(2)}$. Spot checks: $(0,2)$→$(1,1)$ ✓; $(1,1)$→$(2,0)$ ✓; $(3,0)$→$(1,2)$ ✓; $(4,2)$→$(2,4)$ ✓.
$$K^{(3)} = K^{(2)} \Rightarrow \textbf{fixed point}$$

$$\mathrm{Viab}(K) = K^{(2)}, \quad |\mathrm{Viab}(K)| = 19, \quad \text{volume} = 19/25 = 0.76$$

```
r^I ↑
 4 │ V V V ✗ ✗
 3 │ V V V V ✗
 2 │ V V V V V
 1 │ ✗ V V V V
 0 │ ✗ ✗ V V V
 └───────────────────→ r^L
 0 1 2 3 4 V = viable ✗ = excluded
```

**Note the asymmetry.** Low-capital/low-input corners die because they cannot produce; high-capital/high-input corners die because storage caps block the only admissible action. **Both extremes are non-viable** — a genuine property of the constraint structure, not an artefact.

**Every excluded state has $h > 0$.** They look comfortable and are doomed. This is precisely what the margin proxy misses and why VT-3 is mandatory.

## 15.3 Example C — Decision trace, three ticks

Parameters: $h_{\text{crit}}=0.15$, $\beta=2$, $w_{\max}=6$, $\alpha=0.1$. Aspirations: $A_1=5$, $A_2=0.6$, $A_3=-20$.

**Tick 1 — comfortable.** $h=0.42 \ge h_{\text{crit}}$ ⇒ $\psi=1$, $w_{\text{eff}}=6$.
$v_1=7 \Rightarrow \varsigma_1=-2$; $v_2=0.55 \Rightarrow \varsigma_2=0.05$; $v_3=-30 \Rightarrow \varsigma_3=-10$.
Only $\varsigma_2>0$ ⇒ `focus = GOAL(2)`. Scan 4, 1, 3, 2, 6, 5, 0, 8, 7. Action 4 admissible and satisfices ($\delta_c=0.05 \ge 0.05$). **Selected: 4.** Scanned: 1.

**Tick 2 — regulatory shock.** $\theta^{\text{limit}}: 0.90 \to 0.60$; with $u=0.55$, compliance margin falls to $0.05$; $h=0.05 < h_{\text{crit}}$.
$$\psi = \left(\frac{0.05}{0.15}\right)^{2} = (0.3333)^{2} = 0.1111
\qquad
w_{\text{eff}} = \max\big(1, \lceil 6 \times 0.1111 \rceil\big) = \max\big(1, \lceil 0.6667 \rceil\big) = 1$$
`focus = SURVIVAL`. Scan 1, 3, 5, 2, 0, 4, 6, 7, 8. Action 1 admissible, scanned (count 1), satisfices — ordinary production does not contribute to $u$, so $u$ decays out of the window and expected $h$ rises. Budget exhausted. **Selected: 1.**

**Shaping actions 6, 7, 8 were never reached.** R3 registers abandonment — not imposed, but following from Step 3 and the Step 4 table interacting.

**Tick 3 — partial recovery.** $u$ decays to $0.50$; $h = 0.10$.

$$\psi = \left(\frac{0.10}{0.15}\right)^{2} = (0.6667)^{2} = 0.4444
\qquad
w_{\text{eff}} = \max\big(1, \lceil 6 \times 0.4444 \rceil\big) = \lceil 2.6667 \rceil = 3$$
 Still `SURVIVAL`. Action 1 admissible but no longer satisfices (input depleted); action 3 scanned, satisfices. **Selected: 3.** Scanned: 2.

**Aspiration update after tick 3:**
$$A_{2} = 0.6 + 0.1(0.60-0.6) = 0.600, \qquad A_{1} = 5 + 0.1(2-5) = 4.7$$

Capital aspiration falls after underperformance — "satisfactory" is history-dependent (§12.1).

## 15.4 Example D — Repertoire entropy

Window $L_W=8$. Actions: `produce_ordinary` ×5, `deliver` ×2, `hold` ×1. $|A^{\text{adm}}|=9$.

$p = (0.625, 0.25, 0.125)$.

| $p_a$ | $\log_2 p_a$ | $p_a\log_2 p_a$ |
|---|---|---|
| 0.625 | $-0.678072$ | $-0.423795$ |
| 0.250 | $-2.000000$ | $-0.500000$ |
| 0.125 | $-3.000000$ | $-0.375000$ |

$$H_{\text{rep}} = 1.298795\text{ bits}, \qquad \tilde{H}_{\text{rep}} = \frac{1.298795}{\log_2 9} = \frac{1.298795}{3.169925} = 0.409724$$

Report with $|A^{\text{used}}|=3$, $|A^{\text{adm}}|=9$. Fixture tolerance $10^{-6}$.

## 15.5 Example E — RNG key derivation

Run seed `0x5EED0001`, stream `mechanism` (0), plugin `decision.satisficing` (0x1A), phase `decide` (3), tick 147, agent 12, purpose `"shaping_lag"` (0x9C).

```
key = H(0x5EED0001 ‖ 0x00 ‖ 0x1A ‖ 0x03 ‖ 0x0000_0093 ‖ 0x0000_000C ‖ 0x9C)
draw = philox4x32(key, counter = 0)
```

**Fixture requirements:**
1. Same tuple ⇒ same draw, in any run, in any order.
2. Changing **only** `agent_id` 12→13 changes the draw.
3. A forked branch reusing this tuple gets the **identical** draw (CRN).
4. Registering an additional randomness-consuming rule does **not** change this draw. **This is DT-6, the most important determinism test in the suite.**

## 15.6 Example F — Rationing with remainder

Pool 10 units. Same phase, same `conflict_class`: agent 3 claims 6, agent 7 claims 5, agent 11 claims 4. Total 15 > 10.

| Agent | Claim | $\lfloor 10\times\text{claim}/15\rfloor$ | Allocated |
|---|---|---|---|
| 3 | 6 | $\lfloor 4.000\rfloor = 4$ | 4 |
| 7 | 5 | $\lfloor 3.333\rfloor = 3$ | 3 |
| 11 | 4 | $\lfloor 2.667\rfloor = 2$ | 2 |
| | | **Subtotal** | **9** |

Remainder $1$, distributed one unit at a time in ascending `AgentId`: agent 3 receives it.

**Final: 5, 3, 2. Total 10. Conservation exact.**

Deterministic; advantages low-ID agents by at most one unit — a small theoretical assumption, recorded in §34.4 rather than buried.

---

# 16. Parameters and sanity conditions

## 16.1 Defaults

Starting points for Phase 2 exploration, **not calibrated values** (§33.3). All `[D]`.

| Symbol | Name | Default | Sweep range |
|---|---|---|---|
| $h_{\text{crit}}$ | Survival-attention threshold | 0.15 | {0.05, 0.15, 0.30} |
| $\beta$ | Narrowing sharpness | 1.0 | {0, 0.5, 1, 2, 4} — **$\beta=0$ is the null** |
| $w_{\max}$ | Max search width | 6 | {3, 6, 9} |
| $\alpha$ | Aspiration adaptation | 0.10 | {0.05, 0.1, 0.2} |
| $L_W$ | Action window | 8 | {4, 8, 16} |
| $L_M$ | Memory length | 16 | fixed |
| $L_D$ | Dependence window | 12 | fixed |
| $y_0$ | Base yield | 4 | fixed |
| $\eta$ | Capability yield elasticity | 0.8 | {0.4, 0.8, 1.2} |
| $\gamma_R$ | Regulated yield premium | 0.5 | {0.2, 0.5, 1.0} |
| $\kappa_{\ell}$ | Lobby cost | 25 | {10, 25, 50} |
| $\Delta^{\min}_{\ell},\Delta^{\max}_{\ell}$ | Lobby lag | 2, 6 | {(1,1),(2,6),(8,16)} |
| $p^{\ell}_0$ | Lobby base success | 0.25 | {0.1, 0.25, 0.5} |
| $p^{\ell}_{\max}$ | Lobby ceiling | 0.75 | fixed — **MUST be < 1** |
| $\delta_{\theta}$ | Lobby effect size | 0.10 | fixed |
| $\kappa_c$, $\delta_c$, $\Delta_{\text{cap}}$ | Capability investment | 20, 0.05, 3 | fixed |
| $P_c$, $T_c$, $\delta_{\lambda}$ | Compliance penalty, window, legitimacy loss | 30, 4, 0.15 | fixed |
| $R^L_{\max}$, $R^I_{\max}$, $Q_{\max}$ | Caps | 500, 40, 100 | fixed |
| $T$ | Run horizon | 400 | fixed (100 years quarterly) |
| $n$ | Firms | 12 | {1, 4, 12, 20} |

## 16.2 Sanity conditions

**MVP acceptance criterion 8 (§27.3) requires a regime where constraints genuinely bind.** Run in Phase 2 before any experiment.

| # | Condition | Target |
|---|---|---|
| SC-1 | Firms surviving to $T$ under no shock | 60–90% |
| SC-2 | Firm-ticks with $h < h_{\text{crit}}$ | 5–25% |
| SC-3 | Each of the four constraints binds at least once | > 0 |
| SC-4 | Shaping actions attempted | > 5% of actions |
| SC-5 | Shaping success rate | 0.1–0.6 |
| SC-6 | Repertoire-entropy variance across firm-ticks | non-degenerate |

**If SC-1…SC-6 cannot be jointly satisfied by any parameter setting, the model is mis-specified and MUST be revised before Phase 3.** A Phase 2 failure condition, not a reason to keep tuning.

## 16.3 Known ambiguities to resolve in Phase 1

Each requires an ADR when settled.

1. **$u$ trailing-window definition.** Simple mean over $L_W$ assumed; exponentially weighted is an alternative. Affects how fast compliance pressure decays and therefore recovery from §15.3 tick 2.
2. **$\boldsymbol{\theta}$ globality.** Global assumed. Per-firm regimes would change H5 substantially.
3. **Simultaneous lobbying.** Additive assumed; contested resolution is more realistic (Phase 4).
4. **Death and relations.** A dying firm's `supply` edges vanish, which can cascade. Whether cascades are finding or artefact needs AT-style testing.
5. **Capability decay.** Not modelled; $c$ is monotone non-decreasing, which is unrealistic and may make `invest_capability` dominant late in runs.

## 16.4 What the model deliberately cannot represent

So no implementer mistakes omission for oversight.

- **Internal firm structure** (§34.6). Centralisation-under-threat is measurable only as repertoire narrowing, not authority redistribution.
- **Anticipation.** No foresight beyond one step.
- **Learning.** Action selection changes only through aspiration adaptation and state change (§33.2).
- **Geography** (§34.7).
- **Perception error about own viability** (§12.2).
- **Entry.** Firms die; none are founded in the MVP.

---

# PART III — SOFTWARE ARCHITECTURE

# 17. Architecture principles

Seven principles. Every decision in Part III derives from one. A violation is a defect, not a style preference.

| # | Principle | Enforcement |
|---|---|---|
| **A1** | **The kernel contains no domain logic.** No firm, constraint, action, or theory in the kernel crate | CI: `firma-kernel` MUST NOT depend on any plugin crate (§25.6) |
| **A2** | **Rules propose deltas; only the reconciler mutates** | Type system: rules receive `&View`, return `Vec<Delta>` |
| **A3** | **Determinism is structural, not disciplinary.** Canonical ordering everywhere; RNG keyed, never stateful | CI: run twice, byte-diff (§25.2) |
| **A4** | **Theory lives in versioned, content-hashed plugins.** Swapping a theory is a config change | Plugin registry with semver + hashes (§20) |
| **A5** | **Measurement is offline.** Kernel writes events; everything else reconstructs | Kernel exposes no metric API |
| **A6** | **Every assumption is auditable.** Any theoretical choice appears in the run manifest | Provenance service (§22.3) |
| **A7** | **Boring code.** Minimal generics, minimal trait gymnastics, no clever abstractions | Review checklist (§24.6) |

**On A7:** this codebase will be read by people years hence who did not write it, possibly including its author. Cleverness saving twenty lines and costing an hour of comprehension is a net loss. Where Rust offers an elegant abstraction and a boring one, take the boring one and record why in a comment.

---

# 18. Module map

## 18.1 Dependency rules (binding, CI-enforced)

```
firma-core      ← nothing in the workspace
firma-rng       ← core
firma-config    ← core
firma-viability ← core
firma-kernel    ← core, rng            [MUST NOT depend on any plugin]
firma-registry  ← core, config
firma-io        ← core, config
plugins         ← core, rng, viability [MUST NOT depend on kernel or each other]
firma-cli       ← everything
firma-tui       ← core, io
firma-py        ← core, kernel, registry, io
```

**Two hard rules: the kernel never sees a plugin, and plugins never see each other.** Composition happens through the delta/reconciler mechanism, not direct calls. This is what makes theories independently replaceable.

## 18.2 Module contracts

Specified as *responsibility → interface → extension points → tests*.

**firma-core.** Shared types: `AgentId`, `Tick`, `ResourceKind`, `Delta`, `View`, `RngKey`, `PluginId`, error enums. No behaviour. Extension: new `Delta` variants (requires ADR — breaking). Tests: serialisation round-trip; ID uniqueness; `Delta` total ordering is a strict weak ordering.

**firma-kernel.** Advance state deterministically; nothing else.
```rust
fn step(&mut self, world: &mut World, schedule: &Schedule) -> TickReport;
fn snapshot(&self, world: &World) -> Snapshot;
fn restore(&self, snap: &Snapshot) -> World;
fn fork(&self, snap: &Snapshot, iv: &Intervention) -> World;
```
Extension: component, phase, and conflict-resolver registration. Tests: replay identity; permutation invariance over agent iteration; conservation; snapshot round-trip; fork-with-null-intervention equals continuation.

**firma-viability.** Kernels and margins.
```rust
fn margin(&self, x: &State, theta: &Params, cs: &[Constraint]) -> f64;
fn kernel(&self, grid: &Grid, dyn_: &Dynamics, cs: &[Constraint]) -> KernelSet;
fn in_kernel(&self, k: &KernelSet, x: &State) -> bool;
fn volume(&self, k: &KernelSet) -> f64;
```
Extension: alternative approximators (sampling-based, Phase 4). Tests: analytic kernels (VT-1, §15.2); monotone decrease (VT-2); margin/kernel agreement (VT-3).

**firma-registry.** Resolve config plugin references to compiled implementations; verify version and content hash; refuse on mismatch. `resolve(spec) -> Result<Box<dyn Rule>, RegistryError>`. Tests: unknown plugin rejected; version mismatch rejected; hash mismatch rejected; resolution deterministic.

**firma-io.** Durable, reconstructible record. `EventLog::append`, `SnapshotStore::put/get`, `Manifest::compute`. Tests: same manifest ⇒ byte-identical logs; log replay reconstructs state matching snapshots.

---

# 19. Kernel

## 19.1 Scope

A state store, a phase scheduler, a delta reconciler, an invariant enforcer, an RNG key derivator, an event writer. **That is the complete list.** Anything else belongs in a plugin.

## 19.2 State store

Component-oriented (structure-of-arrays), keyed by `AgentId`.

**All iteration over agents MUST occur in ascending `AgentId` order.** `AgentId` is a monotonically increasing `u64` assigned at creation.

**Hash maps MUST NOT be iterated where order can affect results.** Use `BTreeMap`, or collect and sort by canonical key. Rust's `std::collections::HashMap` uses a randomly seeded hasher and provides **no** iteration-order guarantee — documented behaviour, not a bug, and a live reproducibility hazard. Enforced by lint (§25.6).

## 19.3 Phases

The nine phases of §10.1, fixed and declared once. Within a phase all rules read state as of phase start (Jacobi), so ordering within a phase cannot affect results and rules MAY execute in parallel.

## 19.4 Deltas and reconciliation

```rust
pub struct Delta {
    pub target:         DeltaTarget,     // agent, environment, or global
    pub kind:           DeltaKind,
    pub conflict_class: ConflictClass,
    pub origin:         PluginId,
}
```

**Normative algorithm:**
```
1. Collect all deltas emitted in the phase.
2. Sort by (target_id, conflict_class, origin_plugin_id, kind_discriminant).
   Total and stable; ties impossible by construction because a plugin emits
   at most one delta per (target, kind) per phase. Violation panics in debug.
3. Group by conflict_class.
4. Apply the registered ConflictResolver per group.
5. Apply resolved deltas.
6. Check invariants. Violation aborts with a diagnostic — never silently corrected.
```

**Conflict resolvers are plugins, deliberately.** How contention over a scarce resource is settled — proportional rationing, priority, first-come, keyed random — is a theoretical assumption about the world, not an implementation detail. It appears in the manifest and can be ablated.

## 19.5 Invariants

Checked every tick, debug and release. Failure aborts; there is no repair-and-continue path.

| Invariant | Check |
|---|---|
| Resource conservation | Total per type equals initial + sources − sinks, **exactly** (integers, §21.4) |
| Non-negativity | No stock below zero |
| ID uniqueness | No duplicate live `AgentId` |
| Ledger balance | Each agent's ledger sums to its stock |
| Delta uniqueness | No plugin emits two deltas for one (target, kind) per phase |
| Tick monotonicity | Tick strictly increases |

## 19.6 The kernel MUST NOT

Compute metrics · know what a firm, constraint, or action means · read the wall clock · access network or filesystem outside the event log · use `HashMap` in any result-affecting path · allocate in a thread-scheduling-dependent way · contain `unsafe` (§24.5).

---

# 20. Plugin system

## 20.1 Design decision and the constraint behind it

**Compile-time plugin registration over a declarative configuration schema. No bespoke DSL. No dynamic library loading in Phases 0–3** (§34.2, §34.8).

**The Rust-specific reason for no dynamic loading:** Rust has no stable ABI. A `cdylib` compiled against one compiler version can be silently incompatible with a host compiled against another — producing undefined behaviour rather than a clean error. For a platform whose first priority is reproducibility, a loading mechanism that can silently corrupt state is disqualifying.

**Path to true dynamic plugins:** WebAssembly, Phase 4 (§20.7) — solving the ABI problem *and* sandboxing simultaneously.

## 20.2 The Rule trait

```rust
pub trait Rule: Send + Sync {
    fn id(&self) -> PluginId;              // stable across versions
    fn version(&self) -> Version;
    fn phase(&self) -> Phase;
    fn reads(&self) -> &[ComponentId];
    fn writes(&self) -> &[DeltaKind];      // emitting undeclared kind panics

    /// Pure: no I/O, no globals, no wall clock, no ambient randomness.
    fn apply(&self, view: &View, key: RngKey) -> Vec<Delta>;

    /// Plain-language statement of the theoretical assumption this rule
    /// encodes. Written verbatim into the run manifest. MUST NOT be empty.
    fn assumption(&self) -> &str;
}
```

**`assumption()` is not decorative.** It forces every plugin author to state in one sentence what about the world this rule claims. It goes into the manifest, so every run carries a plain-language description of its own theoretical commitments. A reader in ten years can reconstruct what a run assumed without reading the code.

**The trait is load-bearing for §20.7.** Every method added now must be marshalled across a WASM boundary later. **Resist growth.**

## 20.3 Plugin categories

| Category | Replaceable theory | MVP implementations |
|---|---|---|
| `Locality` | How agents interact | `wellmixed`, `network` |
| `Resource` | Resource dynamics | `patchy`, `constant` |
| `Constraint` | What bounds the firm | `solvency`, `compliance`, `scope`, `obligation` |
| **`Decision`** | **How the firm decides** | `satisficing` (§12.3), `random` (null), `greedy` |
| `ActionMarket` | Operating repertoire | `standard` |
| `ActionShaping` | Shaping repertoire | `rdt_standard` |
| `Shock` | Threat generation | `scheduled`, `stochastic` |
| `Observation` | What agents see | `full`, `noisy`, `delayed` |
| `ConflictResolver` | How contention is settled | `proportional`, `priority`, `keyed_random` |

**`Decision` is where competing theories of the firm plug in.** `satisficing` implements Cyert & March; a future `optimizing` implements rational choice; `coalitional` implements internal bargaining (Phase 5). Comparing them under identical everything-else is a first-class experiment (E7) — and is the modularity claim demonstrated rather than asserted.

## 20.4 Configuration

Typed, schema-validated, content-hashed. Unknown keys are an error, not a warning.

```yaml
experiment: rigidity-sweep-001
schema_version: 1.0.0
engine: ">=0.3, <0.4"
question_id: SQ5
hypotheses:
  - {id: H1b, predicted_sign: positive, primary_dv: repertoire_entropy}

world:
  ticks: 400
  locality:  {plugin: locality.wellmixed, version: 1.0.0}
  resources: {plugin: resource.patchy,    version: 1.1.0,
              params: {types: [capital, input, legitimacy], regen: 0.04}}

agents:
  firms:
    count: 12
    components: [Ledger, Capability, Constraints, Goals, ActionRepertoire,
                 ShapingCapability, Relations, Memory, Attention, Strategic]
    constraints: [solvency, compliance, scope, obligation]
    decision: {plugin: decision.satisficing, version: 2.0.0,
               params: {h_crit: 0.15, search_width: 6, alpha: 0.1, beta: 2.0}}
  regulator:
    count: 1
    components: [Constraints]        # passive: no Strategic

shocks:
  - {channel: regulatory, magnitude: 0.3, onset: 200, ramp: instant,
     persistence: permanent, observability: delayed(2), novelty: 0.8, targets: all}

interventions:
  - {at: 200, op: fork, label: no_shock, disable_shock: all}
  - {at: 0,   op: fork, label: no_shaping, remove_rule: action.shaping.rdt_standard}

observation:
  metrics: [repertoire_entropy, search_narrowing, shaping_abandonment,
            margin, in_kernel, dependence_hhi, survival_time]
  cadence: {events: always, snapshot: 25, metrics: offline}

design:
  type: full_factorial
  factors: {h_crit: [0.05,0.15,0.30], novelty: [0.0,0.4,0.8], shaping_lag: [1,4,12]}
  seeds: {range: [1, 200]}

analysis_plan:
  primary_outcome: repertoire_entropy
  primary_test:    "mixed-effects; margin and shortfall as fixed effects; seed random"
  multiplicity:    "FDR (Benjamini-Hochberg) across secondary metrics"
  preregistered:   true
```

## 20.5 Versioning

| Change | Bump |
|---|---|
| Anything altering numerical output for an existing config | **MAJOR** |
| New optional parameter with behaviour-preserving default | MINOR |
| Docs, performance, refactor with identical output | PATCH |

A MAJOR bump MUST come with a golden-trace update demonstrating the change is intentional (§25.7). The engine MUST refuse a config whose plugin MAJOR mismatches.

**No automatic migration.** Migrating an old experiment is a manual, reviewed, recorded act — a silently migrated assumption is an unrecorded change of theory.

## 20.6 Optional theory plugins (deferred, not rejected)

Keep the platform theory-open (§1.3.3). None is a default; each requires an ADR before implementation.

| Plugin | Theory | Phase |
|---|---|---|
| `decision.optimizing` | Rational choice — comparison baseline | 3 |
| `decision.coalitional` | Cyert & March internal bargaining | 5 |
| `transmission.imitative` | Horizontal practice diffusion between firms | 4 |
| `transmission.darwinian` | Variation-selection-retention over routines. **Contested (§3.4); use requires stating a position in the generalised-Darwinism dispute** | 5 |
| `locality.metric_space` | Geographic embedding | 4 |
| `regulator.strategic` | Two-sided lobbying game | 4 |

## 20.7 WASM plugin host (Phase 4)

- Plugins compile to `wasm32-unknown-unknown`, loaded via a host with a stable versioned interface.
- Guest gets **no** syscalls, filesystem, network, or clock, and a hard fuel budget per call.
- Determinism preserved: WASM arithmetic is specified; the host supplies all randomness through the keyed RNG.

**Binding rule: WASM is the only mechanism by which third-party or untrusted plugins may ever be executed. Native dynamic loading is prohibited permanently.**

---

# 21. Determinism and RNG

## 21.1 The four disciplines

**All mandatory. Any one violated breaks reproducibility regardless of language.**

| # | Discipline | Enforcement |
|---|---|---|
| **D1** | No hash-map iteration order may affect any result | Lint; use `BTreeMap` or sort by canonical key |
| **D2** | All randomness from keyed counter-based RNG. No stateful global, no OS entropy | Lint bans `thread_rng`, `SystemTime` |
| **D3** | Floating-point reduction order fixed; conserved quantities integer | §21.4 |
| **D4** | No wall-clock, no thread-scheduling-dependent logic in the simulation path | Lint + review |

## 21.2 RNG architecture

**Counter-based (Philox-4×32 or Threefry-4×64) with hierarchical key derivation.**

```
key  = H(run_seed ‖ stream_id ‖ plugin_id ‖ phase_id ‖ tick ‖ agent_id ‖ purpose_tag)
draw = philox(key, counter)
```

Five required properties; a stateful generator fails four:

1. **Order invariance** — results independent of agent processing order, so parallelism cannot change outcomes.
2. **Random access** — any draw regenerable from its key without replaying history.
3. **Common random numbers** — a fork reuses identical keys for identical `(plugin, phase, tick, agent, purpose)` tuples, so only the intervention differs.
4. **Stream isolation** — adding or removing a rule perturbs no other rule's draws. **This is what makes ablation valid. With a shared stateful RNG, removing a rule shifts every subsequent draw and the "ablation" is confounded with a seed change — silently.**
5. **Statistical quality** — Philox and Threefry pass standard batteries `[E]`.

**Authoring burden this creates:** every rule must thread keys correctly. Reusing a `purpose_tag` or omitting `agent_id` silently correlates draws that should be independent. `purpose_tag` uniqueness is a review checklist item.

## 21.3 Stream separation

Four independent, separately seeded streams: `mechanism` (decisions, action outcomes, shaping success), `environment` (resource dynamics, exogenous variation), `shock` (timing and magnitude), `init` (initial conditions).

A matched-environment design — same `environment` and `shock`, different `mechanism` configuration — removes a large variance component at zero cost.

## 21.4 Floating-point policy

| Rule | Reason |
|---|---|
| **Conserved resources are integers** (`i64`, fixed-point where fractions needed) | Conservation holds *exactly*, making §19.5 a real check |
| Reductions use **fixed order** (sorted by `AgentId`) or pairwise summation | FP addition is not associative |
| `fast-math` and FMA contraction **disabled** | They permit compiler reordering |
| Cross-platform bit-identity is a **goal, not a guarantee**; verified per-platform in CI | Transcendental implementations differ across libm versions |

**Rounding and remainder policy (normative).** Proportional rationing uses **floor division**; the remainder is distributed one unit at a time in ascending `AgentId` order. Deterministic, exactly conserving, and advantages low-ID agents by at most one unit — a small theoretical assumption recorded in §34.4 rather than buried. A `ConflictResolver` plugin MAY implement a different policy, but it MUST conserve exactly.

## 21.5 Parallelism

Permitted **only** within a phase, where A2 guarantees rules see identical input state. Delta reduction is sequential and ordered (§19.4). Sweeps parallelise across runs — embarrassingly parallel and safe by construction.

---

# 22. Persistence and provenance

## 22.1 Three streams

| Stream | Cadence | Contents | Purpose |
|---|---|---|---|
| **Event log** | Every tick | Births, deaths, actions, applied deltas, shocks, interventions, violations | Complete reconstruction; source of all offline metrics |
| **Snapshots** | Every *k* ticks (default 25) | Full serialised state | Fork points, replay-from-checkpoint |
| **Manifest** | Once per run | §22.3 | Identity and reproducibility |

Arrow/Parquet for the event log; compact binary for snapshots. **Target: logging below 15% of runtime**, measured in CI, regression fails the build.

## 22.2 Why offline metrics

Costs one constraint (metrics cannot influence the simulation — correct anyway) and buys three things: new metrics apply retroactively to every past run; experiment runtime is decoupled from analysis richness; metric bugs are fixed without re-running.

**This creates a standing obligation: the event log MUST be sufficient to reconstruct any quantity a metric might need.** Log sufficiency is a first-class requirement, not an emergent property. When in doubt, log it. **Phase 2 should include a deliberate "what would I need for a metric I have not thought of?" review before the first large sweep** — the cheapest time to add a field is before generating 40,000 runs.

## 22.3 Manifest

One JSON manifest per run whose SHA-256 **is** the run identifier. Required: engine git SHA and build hash; every plugin's `(id, version, content_hash, assumption_text)`; fully resolved config after defaults; seed and derivation scheme; complete intervention log with tick stamps; container digest; lockfile hash; host platform; duration; `ExperimentSpec` hash; **manual version in force**.

**Two runs from the same manifest MUST produce byte-identical event logs on the same platform** (DT-1).

## 22.4 The two reproducibility requirements

| | **R1 — Trajectory** | **R2 — Conclusion** |
|---|---|---|
| Question | "Can this exact run be regenerated?" | "Does the conclusion survive different seeds, hardware, implementer?" |
| Achieved by | Manifests, pinned versions, keyed RNG, integer ledgers, canonical ordering | Seed families, pre-registered analysis, published full sweeps, independent re-implementation |
| Status | Debugging and audit tool | **The scientific requirement** |

**R1 without R2 is worthless** — you can perfectly reproduce a trajectory that is an artefact of one seed. R1 exists to make R2 auditable.

**The headline result MUST undergo independent re-implementation** of its core mechanism by someone working from this manual alone, without reading the code (§25.8).

---

# 23. Orchestration, analysis, interface

## 23.1 `firma_lab` (Python, via PyO3)

| Module | Responsibility |
|---|---|
| `spec` | Build, validate, hash `ExperimentSpec`s |
| `runner` | Expand designs into jobs; execute; collect |
| `load` | Read event logs and manifests into DataFrames |
| `metrics` | Offline metric computation (§14) |
| `stats` | Mixed models, survival analysis, bootstrap, FDR, equivalence tests |
| `sensitivity` | Sobol' indices, Morris screening |
| `plot` | Seed-band figures, fork comparisons, parameter-space maps |
| `prereg` | Emit a pre-registration document from an `ExperimentSpec` |

**Figure policy (binding):** every published time series MUST be a band across seeds, never a single line. Single trajectories MAY appear only in appendices, explicitly labelled illustrative, never as evidence.

## 23.2 Interfaces

**Neither produces evidence.** Both are for debugging and hypothesis inspection. A screenshot MUST NOT appear in a results section.

**`firma-tui` — live monitor (MVP).** Built on **ratatui** with the **crossterm** backend; renders to any terminal including a headless server over SSH, matching the actual workload (unattended sweeps, no display attached). Panels: tick progress and ETA; agents alive/dead; margin distribution; action histogram; shock timeline; invariant status; throughput; log tail. Reads the event log by tailing; **MUST NOT** link against the kernel or influence it.

**`firma-inspect` — offline inspector (Phase 3).** Built on **egui** via **eframe**; native single binary, `egui_plot` for charts. Views: matched-fork comparison with divergence trace; viability-kernel slices; agent trajectory browser; relation-graph evolution; shock/intervention timeline; parameter-space map. Reads completed runs from disk.

**Blinding (mandatory).** The fork-comparison view MUST support a blinded mode hiding condition labels until the analyst records an interpretation. Cheap, rarely done, and directly counteracts the strongest bias in simulation research.

---

# 24. Code sustainability standards

The requirement: a competent stranger can understand, modify, and trust this code years from now.

## 24.1 Toolchain

| Item | Standard |
|---|---|
| Rust edition | 2021+; pinned in `rust-toolchain.toml` |
| MSRV | Declared, CI-tested, raised only via ADR |
| Formatting | `rustfmt`, committed config, CI-enforced |
| Linting | `clippy` at `deny(warnings)` plus §25.6 project lints |
| Python | Version pinned; `ruff` + `mypy --strict` |
| Dependencies | Locked; `cargo-deny` for licences and advisories; new dependency requires PR justification |

## 24.2 Naming

| Kind | Convention | Example |
|---|---|---|
| Crate | `firma-<role>` | `firma-viability` |
| Plugin ID | `<category>.<name>` | `decision.satisficing` |
| Metric ID | `snake_case` noun phrase | `repertoire_entropy` |
| Type | `PascalCase`, vocabulary from §5 | `ViabilityMargin` |
| Config key | `snake_case` matching the type | `viability_margin` |

**Every domain identifier MUST use the term defined in §5.** A type named `Fitness` is a defect (§7.3). Naming drift is how a codebase becomes incomprehensible.

## 24.3 Documentation

| Level | Requirement |
|---|---|
| Crate | `//!` doc: responsibility and what it deliberately does not do |
| Public item | `///` doc; missing docs fail CI |
| Plugin | Non-empty `assumption()` plus a doc section: theory, source, known limits |
| Non-obvious code | Comment explaining **why**, never what |
| Magic number | Forbidden. Named constant with documented source |

## 24.4 Architecture Decision Records

**The most important practice here for long-term comprehensibility.** Code records what; ADRs record why; the why evaporates first. Rules and index: §34.

## 24.5 `unsafe`

**`#![forbid(unsafe_code)]` in every crate. No exceptions in Phases 0–3.**

If a later phase requires it: confined to one named module; justified by a measured benchmark, not an assumption; safety comment per block; Miri in CI; ADR approval.

## 24.6 Review checklist

- [ ] Could this affect determinism? (hash iteration, float order, threading, ambient RNG)
- [ ] Does it add domain logic to the kernel? (A1)
- [ ] Does it mutate state outside the reconciler? (A2)
- [ ] Does it use §5 vocabulary correctly?
- [ ] Is there a magic number?
- [ ] Are new public items documented?
- [ ] Does it need an ADR?
- [ ] Does it change numerical output? If so, MAJOR bump and golden trace updated deliberately?
- [ ] Is the boring version available? (A7)

## 24.7 Repository hygiene

Conventional Commits; trunk-based with short-lived branches; every merge green on the full CI matrix; tagged releases with generated changelog; manual version referenced in release notes.

---

# 25. Testing strategy

Ordered from "the software works" to "the science is sound." **The scientific tests are worthless if the software tests fail**, so they run in that order.

## 25.1 Levels

| Level | Scope | Runs |
|---|---|---|
| Unit | Single function | Every commit |
| Property | Invariants over generated inputs (`proptest`) | Every commit |
| Determinism | Reproducibility guarantees | Every commit |
| Integration | Multi-crate, full ticks | Every commit |
| Validation | Analytic ground truth | Nightly |
| Golden trace | Output-change detection | Every commit |
| Performance | Regression thresholds | Nightly |

## 25.2 Determinism tests (mandatory, every commit)

| ID | Test |
|---|---|
| DT-1 | Same manifest twice → byte-identical event log |
| DT-2 | Shuffling internal agent storage order → identical output |
| DT-3 | Single- vs. multi-threaded phase execution → identical output |
| DT-4 | Fork with null intervention → identical to uninterrupted continuation |
| DT-5 | Snapshot → restore → continue equals uninterrupted run |
| **DT-6** | **Adding a randomness-consuming rule does not alter other rules' draws (stream isolation, §21.2 property 4)** |

**DT-6 is the one most likely to be omitted and most damaging if broken**, because it invalidates ablations silently rather than failing loudly. **If it ever fails, treat it as a five-alarm defect: every ablation result since the last passing run is suspect.**

## 25.3 Property tests

Conservation under arbitrary delta sets; non-negativity; reconciliation permutation-invariant; delta total order is a strict weak ordering; ledger balance; margin sign agrees with constraint satisfaction.

## 25.4 Validation tests — analytic ground truth

**MUST pass before any experiment.**

| ID | Test | Ground truth |
|---|---|---|
| VT-1 | Viability kernel, 1-D and 2-D linear system with box constraints | §15.2 (19 states, fixed point at $n=2$) |
| VT-2 | Backward iteration monotone decreasing, reaches fixed point | Theorem `[E]` |
| VT-3 | Margin proxy vs. exact kernel membership, all $d\le4$ configs | Report correlation (§9.2) |
| VT-4 | $\beta=0$, no shortfall ⇒ action never changes (inertia principle) | §3.1 |
| VT-5 | `decision.random` null ⇒ no margin/rigidity relationship | Establishes the null |
| VT-6 | Conservation across a 10,000-tick run, all rules active | Exact (§21.4) |
| VT-7 | Shaping actions costly, lagged, uncertain, every registered plugin | §11.3 |
| **VT-8** | **Independence of $h$ and $\varsigma$.** Across the full E1 Arm A grid, $h$ and $\varsigma$ can be set to any combination and neither mechanically determines the other. Verified by (i) empirical correlation across the grid under intervention, which MUST be near zero by construction; (ii) all four quadrants reachable and populated; (iii) an analytic trace showing no path in the decision procedure computing one from the other | **Without VT-8, H1a and H1b are guaranteed by construction and E1 is void** |

## 25.5 Encoding-artefact tests

| ID | Test |
|---|---|
| AT-1 | Halve tick length, double horizon → qualitative conclusions unchanged |
| AT-2 | Vary viability grid resolution → kernel boundary converges |
| AT-3 | Change conflict resolver → conclusion robust, or dependence reported |
| AT-4 | Change tie-break in action scanning → conclusion robust |
| AT-5 | Perturb initial conditions within a small ball → conclusions stable |

An AT failure is not necessarily a defect — it may be a genuine sensitivity finding. **But it MUST be reported, never suppressed.**

## 25.6 CI-enforced lints

| Lint | Rule |
|---|---|
| `no-kernel-domain-deps` | `firma-kernel`'s dependency graph contains no plugin crate |
| `no-cross-plugin-deps` | No plugin depends on another |
| `no-hashmap-iteration` | `HashMap`/`HashSet` iteration banned in result paths |
| `no-ambient-rng` | `thread_rng`, `random()`, `SystemTime`, `Instant` banned in the simulation path |
| `no-unsafe` | `forbid(unsafe_code)` in every crate |
| `no-magic-numbers` | Numeric literals outside tests require a named constant |
| `docs-required` | `deny(missing_docs)` on public items |
| `assumption-nonempty` | Every registered plugin's `assumption()` is non-empty |
| `no-dynamic-loading` | No `libloading`, `dlopen`, or equivalent |

## 25.7 Golden traces

Reference configurations whose full event-log hashes are committed. Any hash change fails CI and MUST be explicitly acknowledged in the PR with a MAJOR bump and a stated reason. **This makes accidental science-altering changes impossible to merge unnoticed.**

## 25.8 Independent re-implementation

Before the headline result is published, its core mechanism MUST be independently re-implemented by a person working from this manual alone, without reading the source. **Directional agreement required; bit-identity not expected.** This is the only real check on implementation-artefact findings and the single most valuable act for the result's credibility.

---

# PART IV — IMPLEMENTATION

# 26. Phase plan

## 26.1 How phases work

Each phase has objectives, deliverables, and a **gate** — a binary, checkable condition. **Phases are gated on demonstrated capability, not elapsed time.** A phase that has not passed its gate is not finished, regardless of schedule.

Durations assume one experienced developer working steadily, with Rust learning time included. Estimates, not commitments.

| Phase | Name | Duration | Gate |
|---|---|---|---|
| **0** | Foundations | 4–6 wks | Literature check complete; ADRs written; formal model on paper |
| **1** | Kernel | 8–12 wks | All DT tests green; VT-2, VT-6 pass |
| **2** | Model | 8–10 wks | All VT tests pass (incl. VT-8); sanity conditions satisfied |
| **3** | First result | 10–14 wks | H1a/H1b/H1c answered with full robustness |
| **4** | Platform | 12–16 wks | WASM plugins working; strategic regulator; rivalry |
| **5** | Extension | 16–20 wks | Coalitional decision plugin; comparative results |
| **6** | Empirical bridge | Separate project | Pre-registered directional test against external data |

## 26.2 Phase 0 — Foundations

**No code except throwaway exploration.**

**Status: COMPLETE.** Deliverables: this manual; §36 literature verification; §34 ADRs 0001–0009; §35 contracts TC-001–005; §30 pre-registration draft.

**Remaining before Phase 1 begins:** file the pre-registration once §30.9 conditions are met (which requires Phase 2 outputs — so filing happens at the Phase 2/3 boundary, not now); set up repository, CI skeleton, and lint infrastructure; decide licence and data-archival policy (§33.6).

**Risk note:** this phase feels like it is not progress. It is the phase most likely to be skipped and the one whose absence causes the most expensive later damage. **§34.9 records that it justified its cost twice within a single day.**

## 26.3 Phase 1 — Kernel

Build the deterministic substrate. **No domain logic whatsoever.**

**Deliverables:** `firma-core` (types, IDs, `Delta`, `View`, total ordering); `firma-rng` (Philox/Threefry, hierarchical keys, stream separation); `firma-kernel` (state store, phase scheduler, reconciler, invariant enforcer); `firma-io` (event log, snapshots, manifest); `firma-config` (schema, validation, hashing); `firma-registry`; `firma-cli` (`run`, `replay`, `verify`); trivial test plugins sufficient to exercise the kernel; full CI (format, lint, test, determinism, golden trace).

**Gate:** DT-1…DT-6 green. VT-2 and VT-6 pass. A 10,000-tick run with test plugins is byte-reproducible and conserves exactly.

**Risk:** scope creep into modelling. **The kernel is finished when it can move integers around deterministically.** Resist adding "just one" domain type.

## 26.4 Phase 2 — Model

**Deliverables:** `firma-viability` (margin; grid kernel for $d\le4$; kernel-set representation); constraint plugins (`solvency`, `compliance`, `scope`, `obligation`); `decision.satisficing` and `decision.random`; `action.market.standard`; `action.shaping.rdt_standard`; `shock.scheduled` and `shock.stochastic`; `observation.{full,noisy,delayed}`; `firma-tui`; `firma_lab` loading, offline metrics, basic plots.

**Gate:** all VT tests pass **including VT-8**; sanity conditions SC-1…SC-6 satisfied (§16.2); a single run is inspectable end-to-end and its behaviour explicable in terms of §12.3.

**Risk:** discovering degenerate behaviour — all firms die immediately, or nothing ever binds. **This is normal and expected.** Budget explicit time for parameter exploration to find a regime where constraints bite. If no such regime exists, that is a Phase 2 failure requiring model revision, **not a reason to fudge parameters** (§16.2).

**Also in Phase 2:** the log-sufficiency review (§22.2).

## 26.5 Phase 3 — First result

**This phase is where the project either becomes science or stops.**

**Deliverables:** `firma_lab.stats` and `.sensitivity`; the full pre-registered E1 sweep; all AT tests run and reported; all three rigidity and both novelty operationalisations reported; complete data release including negative regions; `firma-inspect`; draft manuscript.

**Gate:** H1a/H1b/H1c answered — supported or refuted — with robustness across three rigidity measures, two novelty measures, and the AT suite. **Either answer passes.** Only "the sweep is uninterpretable" fails.

**Kill criterion applies here** (§2.5).

## 26.6 Phase 4 — Platform

Only if Phase 3 passed. WASM plugin host with sandboxing and fuel limits; `regulator.strategic` two-sided lobbying; multi-firm rivalry and H5 externality experiments; `locality.metric_space`; `transmission.imitative`; `decision.optimizing`; sampling-based kernel approximation for $d>4$; public plugin-authoring guide.

**Gate:** a third party can write, compile, and run a WASM plugin against a published interface without modifying the host.

## 26.7 Phase 5 — Extension

Addresses the largest known validity gap (§8.4, §33.1). `decision.coalitional` with subunits, internal bargaining, side payments, sequential attention across subunit goals; comparative unitary-vs-coalitional experiments; `MemberOf`/`Subunits` components; optionally, and only with an ADR taking an explicit position in the generalised-Darwinism dispute, `transmission.darwinian`.

**Gate:** the coalitional plugin reproduces unitary results in the degenerate single-subunit case, and produces a documented, interpretable difference otherwise.

**Note:** this is where the deferred ENDOGENON project could reconnect as a higher-order layer over the same primitives.

## 26.8 Phase 6 — Empirical bridge

**A separate project with separate scope, funding, and authorship.** Initiated only when a directional prediction exists that survives Phase 3 robustness, is not already known, and has a plausible measurement pathway.

**Recommended first target: open-source software projects.** Public, longitudinal, observable module structure and contributor graphs, and — critically — *unanticipated* structural change through sudden maintainer departure, matching the no-anticipation scope condition in §35.1.

**Do not promise empirical validation in any funding case or publication before this phase begins.**

---

# 27. The MVP, defined

## 27.1 Definition

The state of the system at the **Phase 2 gate**: a running, deterministic, inspectable model with all validation tests passing, capable of executing the E1 sweep.

## 27.2 Contents

| Dimension | MVP value |
|---|---|
| Agents | 1–20 firms + 1 passive regulator |
| Constraint-carrying state dimension | ≤ 4 (§9.3) |
| Constraints | 4 |
| Resources | 3 types, integer, conserved |
| Decision | `satisficing`, ~8 parameters |
| Market actions | 6 |
| Shaping actions | 3 |
| Shock channels | 4 |
| Locality | `wellmixed`, `network` |
| Space | None |
| Learning | None |
| Interface | ratatui monitor |
| Horizon | 400 ticks |
| Performance target | ≤ 5 s per run, single core, 20 agents, 400 ticks |

**Not in the MVP:** coalitional firms; learned controllers; geography; WASM plugins; strategic regulator; evolutionary transmission; >20 agents; the egui inspector; any empirical data.

## 27.3 Acceptance criteria

All of:

1. All DT tests green.
2. All VT tests pass, including VT-3, VT-5, and **VT-8**.
3. A 400-tick, 20-agent run completes in ≤ 5 s single-core.
4. Two runs from the same manifest are byte-identical.
5. A CRN-matched fork with a null intervention is byte-identical to the unforked continuation.
6. Every registered plugin has a non-empty `assumption()` appearing in the manifest.
7. All three rigidity metrics compute from the event log without re-running.
8. **A parameter regime exists in which constraints genuinely bind** — firms neither all die immediately nor never approach a boundary (§16.2).

**Criterion 8 is not a software test. It is the check that the model is scientifically alive.**

---

# PART V — RESEARCH PROGRAMME

# 28. Experiments

## 28.1 Experiment specification

An `ExperimentSpec` is hashable and versioned. **Its hash is computed before any run executes and is the experiment's identity** — making pre-registration enforceable by tooling rather than discipline.

Required fields: `id`, `schema_version`, `engine_requirement`, `question_id`, `hypotheses[]` with **predicted signs**, `factors[]`, `design`, `seeds`, model configuration, `interventions[]` with `fork: true` for CRN-matched counterfactuals, `observation`, **`analysis_plan`** (specific test, primary outcome, multiplicity correction — fixed before running), `stopping_rules`, and **`alternative_explanation`** (§28.3).

## 28.2 Sequence

| # | Experiment | Type | Phase | Purpose |
|---|---|---|---|---|
| V1–V8 | Validation suite (§25.4) | Validation | 1–2 | Engine and model correctness |
| **Result 1** | **Shortfall × margin orthogonal sweep** | Mechanistic | 3 | **Primary — H1a, H1b, H1c** |
| Result 2 | Novelty vs. magnitude (H2) | Mechanistic | 3 | Threat-rigidity core |
| Result 3 | Shaping availability ablation (H3) | Ablation | 3 | The distinctive mechanism |
| Result 4 | Survival by response type (H4) | Robustness | 3 | Adaptive vs. maladaptive |
| E5 | Rigidity-measure convergent validity | Methodological | 3 | Construct validity; **publishable alone** |
| E6 | Encoding-artefact suite (AT-1…5) | Robustness | 3 | Artefact exclusion |
| E7 | Decision-theory comparison (satisficing / optimising / random) | Comparative | 4 | Does the behavioural assumption matter? |
| E8 | Rivalry externality (H5) | Mechanistic | 4 | Unique to endogenous kernels |
| E9 | Two-sided lobbying | Comparative | 4 | Strategic regulator |
| E10 | Path-dependence decomposition | History | 4 | How contingent is the outcome? |
| E11 | Unitary vs. coalitional | Comparative | 5 | Addresses §33.1 |

**Results 1–4 are reported together under one shared Phase-3 pre-registration
filing (§30, "Pre-registration: Experiment E1" in that document's own
naming) — they are not four independently registered experiments, and
"Result *N*" here is deliberately not an "E*N*" label so it cannot be
misread as one.** (PATCH, manual v1.0.1: renamed from E1–E4 to resolve a
naming collision with §30's own "E1" usage — see ADR-0052.)

**E5 deserves emphasis: methodological papers are the most likely early publications and are least dependent on the substantive result being interesting.** They are not overhead.

## 28.3 The alternative-explanation discipline

**Every experiment MUST name at least one alternative explanation and specify how it is discriminated, before running.** This field is required in `ExperimentSpec`; an experiment without it is not runnable.

For E1 the alternative is: narrowing could reflect *admissibility* shrinking (fewer affordable actions) rather than decision narrowing. **Discriminated by reporting $|A^{\text{used}}|$ against $|A^{\text{adm}}|$** — which is why §11.4 exists.

## 28.4 Null models

| Null | Construction | Purpose |
|---|---|---|
| Random decision | `decision.random` | Results require the decision mechanism |
| No narrowing | $\beta = 0$ | Isolates the narrowing mechanism |
| No-shock fork | CRN-matched, shock disabled | Isolates threat effect |
| Frozen constraints | $\boldsymbol{\theta}$ held fixed | Isolates the endogenous-kernel contribution |
| Shuffled shocks | Same shocks, permuted timing | Separates timing from magnitude |

## 28.5 Experiment types

| Type | Question | Design |
|---|---|---|
| Validation | Does the engine reproduce a known result? | Fixed, deterministic |
| Mechanistic | What changes when *X* changes? | Factorial + seeds |
| Ablation | Does removing ρ change *Y*? | Paired CRN forks |
| Threshold | Is there a qualitative transition? | Fine sweep, multiple scales, finite-size checks |
| Robustness | Does it persist under perturbation? | Perturbation × magnitude grid |
| Counterfactual | What if ρ were removed at *T*? | Mid-run fork |
| Path-dependence | How contingent on history? | Fork one trajectory at many *T*, fresh randomness after |
| Comparative | Mechanism A vs. B | Matched-function pairs |

**Path-dependence falls out free from the fork architecture** and is difficult in conventional ABM platforms: forking one long run at many time points and replaying decomposes outcome variance into a historical component and an ongoing-stochasticity component.

---

# 29. Statistical methodology

## 29.1 The unit of analysis is the run

Ticks within a run are massively autocorrelated. Treating them as observations inflates *n* by orders of magnitude and manufactures significance. **This is the most common statistical error in the ABM literature and MUST NOT appear here.**

## 29.2 Core requirements

| Requirement | Detail |
|---|---|
| Seeds | ≥100 per cell for effect estimation; ≥200 for tails or survival |
| Reporting | Full distributions (ECDF or violin), never means alone |
| Effect sizes | Standardised differences with bootstrap CIs, preferred to p-values |
| Pairing | Common random numbers wherever conditions are compared |
| Nesting | Mixed-effects models; do not aggregate away the variance structure |
| Multiplicity | Pre-registered primary outcome; FDR for exploratory, labelled as such |
| Sensitivity | Sobol' or Morris — global, never one-at-a-time |

## 29.3 Survival analysis

`survival_time` is right-censored. Kaplan–Meier with log-rank tests, or Cox models with design factors as covariates. **Analysing censored durations with a t-test is a defect.**

## 29.4 When p-values are inappropriate

When *n* is a compute-budget choice; when the null is a straw man; when the question concerns magnitude or shape; when screening many correlated metrics; when the outcome is a trajectory rather than a scalar.

**Where a null is genuinely interesting** — e.g. "decision theory makes no difference," which would be an important finding — use equivalence testing (TOST) or Bayes factors. Failing to reject is not evidence of absence.

## 29.5 Path-dependence decomposition

$$\mathrm{Var}(Y) = \mathrm{Var}_{\text{history}} + \mathrm{Var}_{\text{residual}}$$

estimated by forking a common trajectory at multiple times with fresh randomness. A large historical component is a substantive finding about the mechanism, not noise to average away.

---

# 30. Pre-registration: Experiment E1

**Status: DRAFT — not to be filed until §30.9 conditions are met.**
Template basis: OSF Preregistration, adapted; simulation-specific fields marked ★.

## 30.1 Title

Dissociating performance shortfall from viability proximity as drivers of organisational search: a pre-registered simulation study.

## 30.2 Research question and rationale

The behavioural theory of the firm predicts that performance below aspiration *increases* search; the threat-rigidity thesis predicts threat *decreases* it. Both are extensively supported. March & Shapira (1992) modelled two reference points — aspiration and survival — with opposite behavioural implications, and Miller & Chen (2004) found mixed empirical support for the resulting predictions `[E]`.

**The question is not whether two reference points exist — they are established.** It is whether they act on **search breadth** (not risk-taking), whether they act **simultaneously** rather than by attention-switching, and how an **endogenous** survival point changes the relationship (§31.2).

**Rationale for simulation ★.** The required manipulation — independently varying shortfall and viability proximity — is unavailable in field data, where they covary and distance-to-survival must be proxied, and unavailable in the laboratory, where genuine termination risk cannot ethically be induced. The model is the only instrument in which the orthogonal design is realisable.

**No external validity is claimed.** All conclusions are Tier 1 or Tier 2 (§4.1). No Tier 3 claim will be made.

## 30.3 Hypotheses

H1a, H1b, H1c, H2, H3, H4 exactly as §2.4, with predicted signs.

## 30.4 Design

**Arm A — Orthogonal manipulation (H1a, H1b).** $h$ and $\varsigma$ set directly by intervention at each measurement tick, independently, across the full grid including empirically rare quadrants.

**Arm B — Endogenous regime (H1c).** No manipulation; both evolve from the model's own dynamics under an applied shock.

**Arm C — Null.** $\beta = 0$ and `decision.random`. Both must show no relationship.

| Factor | Levels | Arm |
|---|---|---|
| Margin $h$ | 0.02, 0.05, 0.10, 0.20, 0.40 | A |
| Shortfall $\varsigma$ (normalised) | 0.0, 0.25, 0.50, 0.75, 1.0 | A |
| Narrowing $\beta$ | 0, 0.5, 1, 2, 4 | A, B, C |
| Novelty | 0.0, 0.4, 0.8 | B |
| Shock magnitude | 4 levels | B |
| Shaping lag | (1,1), (2,6), (8,16) | B |
| Decision plugin | `satisficing`, `random` | C |

Arm A: 125 cells. Arm B: 180. Arm C: 10. **Total 315 cells × 200 seeds = 63,000 runs.**

## 30.5 Randomisation and blinding ★

Counter-based RNG, four streams (§21.2–21.3). Seeds 1–200 per cell. Matched-environment design. **All condition comparisons use common random numbers.** Analysis of flagged events uses blinded mode (§23.2).

## 30.6 Sampling ★

Simulation runs, not human participants. **No runs of the registered design have been executed.** Phase 2 exploratory runs, executed to establish sanity conditions, MUST be disclosed as exploratory. **Any Phase 2 run touching the registered design grid invalidates this registration and requires re-registration with a fresh seed range.**

Seeds per cell: 200, chosen for survival-analysis power on H4, the most demanding outcome. **Stopping rule:** all 63,000 runs execute to completion or firm extinction. No interim analysis, no optional stopping.

## 30.7 Variables and analysis

**Primary outcome:** $\Delta H_{\text{rep}}$, the CRN-matched paired difference against the no-shock fork.

**Secondary:** $\tilde{H}_{\text{rep}}$; $|A^{\text{used}}|$ and $|A^{\text{adm}}|$; $w_{\text{eff}}$; shaping abandonment; survival time; dependence.

**Robustness requirement:** results must hold under R1, R2, R3 and under N1 and N2. Disagreement is the result and will be reported as such (§14.5).

**Primary analysis (fixed before execution):**

*H1a, H1b (Arm A):* `ΔH_rep ~ ς + h + ς:h + β + (1 | seed)`. H1a supported if the $\varsigma$ coefficient is positive with bootstrap 95% CI excluding zero; H1b if the $h$ coefficient is positive likewise. **Both must hold for the dissociation claim.**

*H1c (Arm B):* fit linear and quadratic specifications. Supported if the quadratic term is negative with CI excluding zero *and* the quadratic model is preferred by cross-validated predictive error.

*H2:* contrast novelty and magnitude coefficients at matched $h$. *H3:* three-way interaction $\beta \times \Delta_\ell \times$ time-to-boundary. *H4:* Cox proportional hazards.

**Inference:** effect sizes with bootstrap 95% CIs (10,000 resamples) primary; p-values secondary and never the sole basis for a conclusion. Equivalence testing (TOST, ±0.1 standardised) where a null is of interest.

**Multiplicity:** primary outcome uncorrected; all secondary outcomes Benjamini–Hochberg FDR at $q=0.05$. Anything not specified here is **exploratory** and labelled so in every report.

**Exclusions:** runs failing an engine invariant are excluded with counts and reasons reported. **No exclusion on the basis of outcome values.**

## 30.8 Falsification

| Hypothesis | Falsified if |
|---|---|
| H1a | Shortfall coefficient zero or negative across the grid |
| H1b | Margin coefficient zero or negative across the grid |
| **H1c** | Endogenous relationship monotonic, or quadratic model not preferred |
| **The dissociation** | H1a and H1b do not both hold |
| H2 | Novelty and magnitude coefficients statistically equivalent |
| H3 | No interaction between shaping lag and time-to-boundary |
| H4 | No interaction between narrowing and novelty in survival |

**Joint failure of H1a and H1b would contradict an established model** (March & Shapira), which is a *stronger* result than failing to confirm a conjecture — but it requires distinguishing model defect from genuine boundary condition before publication (§2.5).

**A negative result is publishable and will be reported with the same completeness as a positive one.** The full sweep including all null regions will be released regardless.

## 30.9 Conditions for filing ★

**Not to be filed until:**

- [ ] All §25.4 validation tests pass, **including VT-8**, with evidence attached.
- [ ] Sanity conditions SC-1…SC-6 satisfied (§16.2).
- [ ] AT-1…AT-5 run and reported.
- [ ] All DT tests green.
- [ ] Seed range 1–200 confirmed unused by any exploratory Phase 2 run.
- [ ] All attached documents content-hashed and hashes recorded.

**Filing before VT-8 passes would pre-register a study whose central manipulation has not been shown to be possible.**

## 30.10 Declared limitations

Recorded in advance so they cannot later be presented as discoveries: unitary firm (§33.1); historical aspiration only (§35.4); fixed action alphabet (§35.2); passive regulator (§35.3); exact self-observation of margin (§35.1); no calibration (§33.3).

## 30.11 Availability

Code, configurations, manifests, complete sweep output including null regions, and analysis scripts released under an OSI-approved licence and archived with a DOI. Every reported run reproducible from its manifest hash (R1); every conclusion re-testable from the released sweep (R2).

---

# 31. Positioning, prior art, publication

## 31.1 Contribution, stated narrowly

> A formalisation of the two-reference-point model of organisational response, applied to **search breadth** rather than risk-taking, with both mechanisms operating **simultaneously** rather than by attention-switching, in a model where the **survival point is endogenous** to the firm's own action.

**This statement has been narrowed twice** (§34.9, §36). Overclaiming will be caught immediately by anyone who knows the performance-feedback literature.

## 31.2 What is genuinely new

| Element | Novelty | Assessment |
|---|---|---|
| The aspiration/survival dissociation itself | **None.** March & Shapira (1992); Audia & Greve (2006); Miller & Chen (2004) `[E]` | Cite as parent, never claim |
| **A formal model at all** | **None.** March & Shapira (1992) is itself a formal random-walk model, not a verbal theory `[E]`. FIRMA is not "the first formalisation" | Do not claim it |
| **Search breadth as DV** | **Genuine.** All prior two-reference-point work measures risk-taking. Threat-rigidity's actual claim is information processing and behavioural variety. A firm can search broadly and choose safely, or search narrowly and choose recklessly | Moderate |
| **Simultaneous vs. attention-switching** | **Genuine.** March & Shapira model attention focusing on one reference point *or* the other; FIRMA has both acting additively on separate variables. Different shapes in the intermediate region | Moderate; H1c tests it |
| **Endogenous survival point** | **Genuine and specific to FIRMA.** All prior work treats the survival point as fixed and proxies distance to it. No prior model lets a firm move its own boundary (§9.4, §35.3) | **Largest surviving gap** |
| Independent manipulation of $h$ and $\varsigma$ | Genuine as method; empirical work necessarily proxies | Methodological |
| Interventional construct definition (§6.6) | Uncommon in organisational ABM | Methodological |

## 31.3 Prior art

| Work | Relationship |
|---|---|
| **March & Shapira (1992), *Psychological Review* 99(1):172–183** | **The direct theoretical parent.** Random-walk model of risk-taking with aspiration and survival reference points; attention focus determines direction of response `[E]`. **MUST be cited in the first paragraph of any paper.** |
| **Miller & Chen (2004), *AMJ* 47(1):105–115** | Organisational tests of the March–Shapira model; found variables affecting risk, and effect *sizes* but not *signs*, differed across performance categories, with poorly performing organisations showing **increased** risk as they neared bankruptcy `[E]` — partially contrary to the survival-focus prediction. **The parent model's empirical support is mixed, which makes this a live question.** |
| **Audia & Greve (2006), *Management Science*** | "Less Likely to Fail" — resource endowment as boundary condition; buffered firms take more risk under shortfall, resource-limited firms less `[E]`. FIRMA manipulates directly what this proxies |
| **Staw, Sandelands & Dutton (1981), *ASQ* 26(4):501–524** | The threat-rigidity thesis; supplies the DV (information restriction, behavioural variety) and the novelty moderator `[E]` |
| **Billinger, Stieglitz & Schumacher (2013), *Organization Science*** | Experimental NK search; failure promotes more exploratory search `[E]` |
| **Axtell's ABM of firms** | **Different level and direction.** Bottom-up: workers optimise, firms emerge, population statistics (Zipf sizes) are the payoff. FIRMA is top-down: the firm is given and has interiority. Cite and distinguish |
| **Dosi–Fagiolo–Roventini K+S** | Heterogeneous boundedly-rational firms in macro; shares the behavioural lineage, different question |
| **NK organisation models** (Levinthal; Rivkin; Ethiraj & Levinthal) | Established for structure and search; fixed exogenous interdependence, no viability constraint, no shaping |
| **Repast Simphony, MASON** | Mature Java ABM toolkits; Repast's plugin-layered architecture is a direct architectural precedent worth studying |

## 31.4 Publication targets

| Venue | Fit | Best for |
|---|---|---|
| **Computational and Mathematical Organization Theory** | **Best first target.** Scope explicitly organisational research via simulation and formal modelling; welcomes system/algorithm papers `[E]` | Platform paper; first result |
| **Strategy Science** (INFORMS) | Strong. Has welcomed analytic and computational models since founding `[E]`. **Read their editorial on modelling papers before drafting** | Strategy-framed result |
| **JASSS** | Good, open access, Q1 | Methods paper; the viability formalism |
| **JOSS** | Straightforward | Software paper |
| *Organization Science*, *Management Science*, *SMJ*, *AMR* | Aspirational, not first targets | Only after a robust surprising result |

**Recalibrated:** with the narrowed contribution, CMOT and Strategy Science are correct first targets. *Organization Science* is plausible only if the endogenous-survival-point result proves robust — and **that belongs to a second paper, not the first.** The first establishes the base relationship; the second modifies it.

## 31.5 The deferred project

ENDOGENON — a multi-level laboratory for organisational emergence, encapsulation, and synergy — was analysed separately and deferred. It shares no code with FIRMA at present. §26.7 notes it could reconnect as a higher-order layer over the same primitives if Phase 5 succeeds. **It is not part of this manual's scope and MUST NOT be conflated with FIRMA in any funding case or publication.**

---

# 32. Risks

## 32.1 Tier 1 — project-threatening

| # | Risk | Mitigation | Residual |
|---|---|---|---|
| 1 | **The model produces nothing interesting.** Constraints never bind, or everything dies | Criterion 8 (§27.3); Phase 2 exploration budgeted; kill criterion (§2.5) | **Medium. Partly irreducible** |
| 2 | **Excessive degrees of freedom** — the model can produce any result | Hard cap on active rules; pre-registration; publish full sweeps including negative regions | **High. Not fully solvable; disclose it** |
| 3 | **Construct invalidity** — "rigidity" measures something else | Three operationalisations, all reported (§14.2, §14.5) | Medium |
| 4 | **Solo-builder overrun.** Rust plus a full platform is large for one person | Phase gates; MVP is Phase 2 not Phase 4; ADRs preserve context across interruptions | **High. See §32.4** |
| 5 | **Implementation artefact reported as a finding** | VT suite; AT suite; independent re-implementation (§25.8) | Medium |
| 6 | **VT-8 fails** — $h$ and $\varsigma$ turn out mechanically linked | Tested before filing; failure means model revision, not a fudge | Medium |

## 32.2 Tier 2 — serious

Nondeterminism via parallelism or float order (mitigated: DT suite in CI); viability kernel intractable at needed dimension (mitigated: $d\le4$ requirement, margin proxy); parameter-space explosion (mitigated: Sobol' screening before full factorial); scope creep into ENDOGENON (mitigated: §3.4 exclusions, ADR requirement).

## 32.3 Tier 3 — manageable

Logging bottleneck; version incompatibility; Rust learning curve; visualisation seduction (mitigated: §23.2, §23.1 figure policy).

## 32.4 The solo-builder consideration

Three specific protections beyond phase gates:

1. **ADRs are load-bearing for you personally.** After a three-month interruption, §34.4 telling you why the ledger is integer saves an afternoon of re-derivation and prevents a well-intentioned "improvement" that breaks conservation.
2. **The Phase 2 gate is the real milestone.** A running, validated model is the point at which the project has produced value even if everything after stalls.
3. **Consider recruiting a methodologist** — someone whose job is pre-registration, statistical design, and saying no to interesting-looking single runs. The hardest role to fill for yourself, because the bias it guards against is invisible from the inside.

---

# 33. Open questions and acknowledged gaps

## 33.1 The unitary-firm gap

The MVP models the firm as a single satisficer. Threat-rigidity may be substantially coalitional — centralisation *is* a redistribution of internal decision rights, unrepresentable in a unitary model. Phase 5 addresses this. **Until then, every result is a result about a unitary firm and MUST be described that way.**

## 33.2 Learning

FIRMA models adaptation as within-lifetime action reselection, not learning in any rich sense. Real firms learn. Deliberately deferred: introducing a learned controller destroys the ablation design supplying internal validity — a network reorganises to compensate for a removed mechanism, so removing it no longer isolates it. **If learning is added, it MUST always run alongside the simplest-controller arm.**

## 33.3 Calibration

Parameters have no empirical units. There is no principled way to set $h_{\text{crit}}$ or shaping costs to values corresponding to anything real. **Consequence: only directional and ordinal claims are supportable** (§4.1 Tier 2). Whether a defensible calibration strategy exists is open for Phase 6.

## 33.4 Novelty operationalisation

N1 and N2 may diverge. Which better matches the threat-rigidity thesis's notion of novelty is not settled by the source literature — a genuine construct question, not merely a measurement choice.

## 33.5 Kernel tractability beyond four dimensions

Sampling-based and learned approximations exist but introduce approximation error into what is meant to be ground truth. A real barrier to richer models with no clean solution at present.

## 33.6 Licence and data management — decide before Phase 1

Two decisions to make explicitly and record as ADRs:

- **Licence.** An OSI-approved licence is strongly recommended — effectively required for a JOSS paper and materially strengthening any reproducibility claim.
- **Data release.** Sweep outputs are large. Decide the archival plan (Zenodo or equivalent, with DOIs) and retention policy **before** generating the first large sweep.

## 33.7 Ethics

Phases 0–5 involve no human subjects and raise no issues beyond ordinary research integrity. **Phase 6 does.** Any use of organisational communication data requires ethics review, consent, and access agreements obtained *before* collection begins, not retrofitted.

---

# PART VI — DECISION RECORDS

# 34. Architecture Decision Records

## 34.0 Rules and index

**ADRs are immutable.** Once accepted, an ADR is never edited except to change its `Status` line. A reversed decision gets a *new* ADR; the old is marked superseded. **The record of a wrong turn is as valuable as the record of a right one.**

**An ADR is REQUIRED for:** any deviation from a SHOULD; a new primitive (§7.2); a new `Delta` variant; a new dependency category; any MSRV or toolchain change; enabling any optional theory plugin (§20.6); any MAJOR version bump of this manual.

| § | ID | Title | Status |
|---|---|---|---|
| 34.1 | 0001 | Rust for the simulation kernel | Accepted |
| 34.2 | 0002 | No bespoke DSL | Accepted |
| 34.3 | 0003 | Counter-based RNG with hierarchical keys | Accepted |
| 34.4 | 0004 | Integer resource ledger | Accepted |
| 34.5 | 0005 | Offline metric computation | Accepted |
| 34.6 | 0006 | Unitary firm in the MVP | Accepted |
| 34.7 | 0007 | Non-spatial by default | Accepted |
| 34.8 | 0008 | Compile-time plugins; WASM deferred | Accepted |
| 34.9 | 0009 | Research question revision after Phase 0 | Accepted |

---

## 34.1 ADR 0001 — Rust for the simulation kernel

**Context.** Reproducibility is priority 1 (§1.3). The workload is batch throughput, not interactive latency — nobody watches a single run — which removes "GC stutter" from the decision. The binding risk is a data race in parallel execution silently changing a result in one run in ten thousand, dormant for months, surfacing after publication.

**Decision.** **Rust for kernel and plugins; Python for orchestration and analysis via PyO3.** The Rust surface is confined to `crates/`; everything a researcher touches daily is Python.

The primary reason is narrow: **in safe Rust a data race between threads is a compile error.** No garbage-collected alternative offers this. Secondary: no GC allocation-timing variability; explicit floating-point control (§21.4); a type system able to encode the rule-purity contract.

**Alternatives.** *Go* — lightest to write, very mature; rejected on the one discriminating criterion, no compile-time race prevention (its map-iteration randomisation is symmetric with Rust's, so not a differentiator). *C#/.NET* — competitive, strong deterministic-simulation precedent in game engines; same gap; reasonable given existing expertise, of which there is none here. *Java* — **strongest domain precedent** (Repast, MASON); rejected on the race guarantee and heavier ceremony; **the alternative with the best claim, not dismissed lightly**. *Julia* — excellent scientifically; rejected on reproducibility tooling maturity. *Python only* — 10–100× too slow for sweeps; retained for orchestration. *Zig* — pre-1.0, shifting ecosystem; fails "mature enough". *OCaml* — mature, fast, lighter than Rust; no equivalent race guarantee, smaller simulation ecosystem. *C++* — worse safety and build hygiene for a small team.

**Consequences.** *Positive:* a category of nondeterminism bug becomes impossible; compile-time enforcement of §18.1 dependency rules. *Negative, accepted:* **steep learning curve** — the largest risk this ADR creates for a solo builder; slow compiles lengthening the edit–test cycle; **thin ABM precedent**, so patterns must be adapted from Repast/MASON rather than reused; smaller collaboration pool. *Neutral:* **Rust does not give reproducibility for free.** `std::collections::HashMap` uses a randomly seeded hasher with no iteration-order guarantee — the same hazard as Go. The four disciplines in §21.1 are mandatory regardless of language; this ADR removes one risk, not the need for discipline.

**Compliance.** Lints `no-hashmap-iteration`, `no-unsafe`; DT-1…DT-6 every commit; A7 review item.

**Note.** If the learning curve proves prohibitive in Phase 1, the fallback is Go with §21.1 applied rigorously plus a runtime race detector in CI. That would be a superseding ADR, not a quiet substitution.

---

## 34.2 ADR 0002 — No bespoke DSL

**Context.** Theories must be swappable without touching the engine (§1.3.3). The instinctive solution is a DSL. It fails two ways: too weak, and the first novel mechanism forces an engine patch, destroying the auditability the DSL existed for; or Turing-complete, and it becomes a programming language with no debugger, profiler, type checker, editor support, or ecosystem. The actual requirement is narrower than "a language": assumptions must be **declarable, diffable, versionable, auditable**.

**Decision.** **Typed, schema-validated, content-hashed declarative configuration over a registry of compiled Rust plugins. No new syntax.** Composition is declarative; primitives are ordinary code. Three properties satisfy the requirement without a parser: every rule is a registered plugin with semver and content hash; every experiment is a hashed document; the kernel contains no domain logic (A1), CI-enforced. `Rule::assumption()` (§20.2) supplies the plain-language audit trail.

**Alternatives.** *Full DSL with parser* — both failure modes; also months of work producing no science. *Embedded scripting (Lua, Rhai, Starlark)* — avoids writing a parser, but adds an FFI boundary in the hot path and, decisively, **a second determinism surface to audit**. Retained as a candidate for sandboxed untrusted plugins, where §34.8 selects WASM instead. *Pure code, no configuration* — assumptions invisible in diff and manifest; fails A6. *Config as executable Python* — destroys hashability; **config must be data.**

**Consequences.** *Positive:* mechanism authorship becomes a code-review event, which is correct — a new mechanism *is* a new theoretical assumption; full tooling support; no grammar to maintain; configs are pure data, therefore hashable, therefore pre-registerable. *Negative, accepted:* **adding a new primitive mechanism requires writing Rust and recompiling** — more friction than a DSL promises, deliberate but real, and it raises the barrier for a domain-expert collaborator who does not program. Composition is limited to what plugin categories anticipate; a novel *category* requires a manual change.

**Compliance.** Lints `no-kernel-domain-deps`, `assumption-nonempty`; schema validation with unknown keys as errors.

**Note.** **If the configuration format ever acquires conditionals, loops, or expression evaluation, it has become a DSL by accretion.** That is the moment to stop and write a superseding ADR, not to continue drifting.

---

## 34.3 ADR 0003 — Counter-based RNG with hierarchical key derivation

**Context.** §6.6 defines every construct as a functional of a matched trajectory pair. This works only if branches differ *solely* by the intervention. Separately, ablation removes a rule and compares — and with a shared stateful generator, removing a randomness-consuming rule shifts every subsequent draw for every other rule. The ablation is then confounded with an uncontrolled seed change, **and the failure is silent: results look fine and are wrong.**

**Decision.** **Counter-based RNG (Philox-4×32 or Threefry-4×64) with hierarchical key derivation. No stateful global generator in the simulation path.** Four separately seeded streams (§21.3). Five required properties, four of which a stateful generator fails (§21.2).

**Alternatives.** *Single global stateful RNG* — fails order invariance, random access, CRN, and stream isolation; the last is disqualifying because it invalidates ablations invisibly. *Per-agent stateful streams* — fixes order invariance; still fails CRN on agent birth/death (a fork where an agent dies earlier desynchronises every subsequent stream) and requires serialising generator state in every snapshot. *Splittable RNG* — genuinely good; rejected because reproducing a specific draw requires reproducing the split tree, making CRN matching across structurally different branches fragile. *Hardware/OS entropy* — non-reproducible; excluded absolutely.

**Consequences.** *Positive:* forks exactly matched by construction, not by care; safe parallelism; snapshots need not carry generator state; a specific draw reproducible in isolation while debugging. *Negative, accepted:* slightly slower per draw (key derivation costs a hash), irrelevant at this scale but real; **every rule author must thread keys correctly** — reusing a `purpose_tag` or omitting `agent_id` silently correlates draws that should be independent. This is the main new failure mode this decision introduces.

**Compliance.** Lint `no-ambient-rng`; **DT-6** (stream isolation) and **DT-4** (null-intervention fork identity); `purpose_tag` uniqueness on the review checklist.

**Note.** **If DT-6 ever fails, treat it as a five-alarm defect.** Every ablation result since the last passing run is suspect, because the failure mode produces plausible numbers rather than crashes.

---

## 34.4 ADR 0004 — Integer resource ledger

**Context.** Conservation is the strongest correctness invariant available (§19.5, VT-6). Its value depends on being checkable *exactly*. With floating point, conservation can only be asserted within a tolerance — and choosing that tolerance is choosing how much silent leakage to permit. A drift of 10⁻¹² per tick over 400 ticks and 200 seeds is invisible to any reasonable tolerance and is exactly the defect this invariant exists to catch. Floating-point addition is also non-associative, so reordered or parallel reduction changes the sum.

**Decision.** **All conserved, transferable resources are `i64` integers.** Fixed-point where fractional units are needed, with the scale recorded in the manifest. Non-conserved real state (capability, legitimacy, margin) remains `f64` under the fixed-reduction-order rule.

**Rounding and remainder policy (normative, because rationing produces remainders):** floor division; remainder distributed one unit at a time in ascending `AgentId` until exhausted. Deterministic, exactly conserving, advantages low-ID agents by at most one unit. A `ConflictResolver` plugin may implement a different policy but MUST conserve exactly.

**Alternatives.** *`f64` with tolerance* — the tolerance licenses silent drift; also non-associative. *Rational/arbitrary precision* — exact and associative, but denominators grow unboundedly under repeated division and performance degrades unpredictably. *Decimal* — solves representation, not associativity; a dependency for no gain over fixed-point. *`i128`* — headroom beyond any plausible need; doubles memory for the largest component.

**Consequences.** *Positive:* conservation checkable exactly, making §19.5 a real invariant; integer addition is associative so reduction order cannot change a total; overflow detectable rather than silently absorbed. *Negative, accepted:* **divisibility becomes a modelling concern** — "split three ways" has a remainder and the policy must be applied consistently. This is friction `f64` would hide, but hiding it was never correctness. Fixed-point scale is chosen once and awkward to change (MAJOR bump, golden traces invalidated). Naturally continuous quantities (rates, elasticities) must be `f64` parameters acting on integer stocks — a slightly awkward split.

**Compliance.** Property tests on conservation and permutation-invariance; VT-6; checked arithmetic (`checked_add`/`checked_sub`) enforced by review.

**Note.** The remainder policy is a small theoretical assumption wearing implementation clothes. It is recorded here rather than buried in code precisely because a reader in five years will otherwise wonder why low-ID agents do marginally better under tight rationing.

---

## 34.5 ADR 0005 — Offline metric computation

**Context.** FIRMA's metrics include expensive quantities — kernel membership, entropy over trailing windows, dependence concentration, paired fork comparisons. Computing them in the tick loop would couple experiment runtime to analysis richness and mean that adding a metric requires re-running every past experiment. Worse: a metric bug found after a sweep would invalidate the sweep rather than the analysis.

**Decision.** **The kernel writes a complete event log; all metrics are computed afterwards from it, in Python.** The kernel exposes no metric API. Three streams (§22.1).

This creates an explicit design obligation: **the event log MUST be sufficient to reconstruct any quantity a metric might need.** Log sufficiency is a first-class requirement, not an emergent property. **When in doubt, log it.**

**Alternatives.** *Inline computation* — couples runtime to analysis; new metrics require re-running; metric bugs invalidate experiments. *Hybrid (cheap inline, expensive offline)* — two code paths for one concept, which drift; also blurs A5, making it unclear whether a number came from kernel or analysis. *Full state snapshot every tick* — sufficient but storage-prohibitive; event log plus periodic snapshots reconstructs the same information far more cheaply.

**Consequences.** *Positive:* **new metrics apply retroactively to every past run** — over a multi-year project worth more than any runtime optimisation; runtime decoupled from analysis richness; metric bugs fixed in minutes; metrics live where the statistical ecosystem is; A5 enforced cleanly because metrics do not exist while the simulation runs. *Negative, accepted:* **log sufficiency is a standing risk** — if something was not logged it cannot be reconstructed and the run must repeat. **Expect at least one such discovery in Phase 2.** Storage grows with sweep size, requiring an archival policy (§33.6). The live TUI can only display quantities cheap enough to derive from a tailed log.

**Compliance.** Kernel exposes no metric API (review); CI performance test, logging under 15%; integration test that log replay reconstructs state matching snapshots; MVP criterion 7.

**Note.** Phase 2 should include a deliberate "what would I need to log for a metric I have not thought of?" review before the first large sweep. **The cheapest time to add a field is before generating 40,000 runs.**

---

## 34.6 ADR 0006 — Unitary firm in the MVP

**Context.** FIRMA's decision mechanism derives from Cyert & March. But their central claim is not merely that firms satisfice — it is that **the firm is a coalition of subunits with partially conflicting goals**, and that "the firm's objective" is the output of internal bargaining. Implementing the unitary version adopts the decision rule while discarding the mechanism that, in the source theory, *generates the goals that rule operates on*.

This matters unusually much here, because **centralisation of control is one of the threat-rigidity thesis's two limbs** — and centralisation is by definition a redistribution of internal decision rights. A unitary firm has none to redistribute.

**Decision.** **The MVP firm is unitary. Coalitional structure is deferred to Phase 5** as `decision.coalitional` with `MemberOf` and `Subunits`. A sequencing decision, not a theoretical rejection.

The reason is attribution: building both constraint dynamics and internal bargaining at once means any result has two candidate explanations and no way to separate them. The unitary version establishes a baseline against which the coalitional version is compared under otherwise identical conditions — a designed experiment (E11), and a better use of the machinery than having it present from the start as an untested confound.

**Alternatives.** *Coalitional from the outset* — doubles the hard problems simultaneously; roughly doubles Phase 2 for a solo builder. *Unitary permanently* — leaves the largest known validity gap permanently unaddressed and makes any centralisation claim unsupportable. *Two subunits as a minimal coalition* — tempting middle path; **rejected because two subunits with a fixed split is not bargaining**, it is a unitary firm with extra bookkeeping, and would create a false impression of having addressed the gap.

**Consequences.** *Positive:* Phase 2 achievable; results attributable to the constraint mechanism; the `Decision` plugin category is exercised from day one, proving the modularity claim rather than asserting it. *Negative, accepted and stated in every publication:* **the largest known validity gap** (§33.1). The model cannot represent centralisation as authority redistribution — only repertoire narrowing (R1, R2) and shaping retreat (R3). **If real threat-rigidity is substantially coalitional, the MVP measures a proxy, not the construct.** Any Phase 2–4 publication MUST state this explicitly, not in a generic limitations paragraph.

**Compliance.** §8.4 and §33.1 record the gap; publication checklist (§39C) includes the statement; Phase 5 gate requires degenerate-case equivalence.

**Note.** This is the ADR most likely to be cited by a reviewer. Having it written, dated, and reasoned in advance is materially better than the alternative.

---

## 34.7 ADR 0007 — Non-spatial by default

**Context.** ABMs conventionally embed agents in space, and it is the first thing most readers expect. For FIRMA it would be a mistake: none of the §2 questions is spatial, and geography would multiply cost, introduce confounds (every result acquires a competing explanation in terms of density, clustering, or diffusion, which must then be ruled out), and consume Phase 2 time on machinery answering no current question. There is also a subtler risk: **spatial models are visually compelling, and visual compellingness is the bias §23.2 exists to counteract.**

**Decision.** **Non-spatial by default. Locality is a plugin behind one interface:** `neighbours(agent, t) -> Set<AgentId>`. MVP: `wellmixed`, `network`. Phase 4: `tag_affinity`, `metric_space`. Nothing in the kernel, constraints, or actions assumes a spatial embedding.

**Alternatives.** *Continuous 2-D from the start* — highest cost, most confounds, answers no §2 question. *Grid/lattice* — cheaper, but still imposes a neighbourhood structure requiring elimination as an explanation for every result. *Network-only, hard-coded* — nearly right, and `network` is an MVP implementation; rejected as the *only* option because it forecloses `wellmixed`, the cleaner null for isolating mechanism from structure. *No locality concept at all* — simplest, but forecloses SQ3, SQ4, and H5, which need some notion of who interacts with whom.

**Consequences.** *Positive:* Phase 2 smaller and faster; results unconfounded by spatial structure; `wellmixed` vs. `network` becomes a clean two-level manipulation rather than a modelling commitment; "does interaction structure matter?" is a one-line ablation. *Negative, accepted:* **the MVP cannot address geographic questions** — regional regulatory variation, distance-dependent supply cost, entry by location. Legitimate questions, simply out of scope until Phase 4. Readers expecting a map may find its absence surprising; state the rationale briefly in any paper.

**Compliance.** No coordinate or position component in §8.4; no plugin outside `Locality` may query spatial information; §27.2 records "Space: None".

**Note.** If a Phase 4 question genuinely requires geography, implement `locality.metric_space` against the existing interface and re-run an existing experiment under both. **The difference is then a measured result rather than a design assumption** — a better outcome than having had space all along.

---

## 34.8 ADR 0008 — Compile-time plugins; WASM deferred to Phase 4

**Context.** §34.2 establishes that theory lives in plugins composed by declarative configuration. How are plugins loaded? The obvious answer — dynamic loading of shared libraries — has a Rust-specific problem that is easy to underestimate. **Rust has no stable ABI.** A `cdylib` compiled against one compiler version, feature set, or dependency version can be silently incompatible with the host. The failure is not a clean load error but undefined behaviour: mismatched struct layouts, incorrect vtable dispatch, memory corruption. Systems that appear to work generally require host and plugin built in lockstep, defeating the purpose. For a platform whose first priority is reproducibility, **a loading mechanism that can silently corrupt state is disqualifying.** Separately, native dynamic libraries offer no isolation — a loaded plugin has full process privileges.

**Decision.** **Phases 0–3: compile-time registration.** Plugins are workspace crates registered via macro into `firma-registry`, which verifies ID, semver, and content hash and refuses on mismatch. Adding a plugin requires a recompile.

**Phase 4: a WebAssembly plugin host**, solving both problems at once — ABI stability (WASM has a specified, stable binary interface, so a plugin compiled today loads correctly against a host compiled in five years) and sandboxing (no syscalls, filesystem, network, or clock; hard fuel budget per call). Determinism preserved: WASM arithmetic is specified and the host supplies all randomness through the keyed RNG, so guests cannot introduce an unaudited entropy source.

**Binding rule: WASM is the only mechanism by which third-party or untrusted plugins may ever be executed. Native dynamic loading is prohibited permanently.**

**Alternatives.** *`cdylib` dynamic loading now* — undefined behaviour on version skew; no sandboxing; incompatible with priority 1. *C ABI with `extern "C"`* — technically stable, but forces every plugin interface through `repr(C)` types, losing the type-level enforcement of the rule contract that is a principal reason for choosing Rust (§34.1). *Embedded scripting* — solves dynamism with some sandboxing; rejected for the §34.2 reason (a second determinism surface) and because WASM gives stronger isolation with a compiled-language authoring experience. *WASM immediately in Phase 1* — correct destination, wrong time; it adds host-integration complexity to the phase whose gate is a working deterministic kernel, and there are no third-party plugins yet to justify it.

**Consequences.** *Positive:* no ABI hazard in Phases 0–3; the compiler verifies every plugin interface; resolution errors are compile-time rather than runtime-undefined; content hashing is straightforward for the manifest; Phase 4's WASM work has a single clear motivation rather than being speculative infrastructure. *Negative, accepted:* **until Phase 4, adding or modifying any plugin requires recompiling the whole binary** — with Rust compile times, the concrete daily cost of this decision. No third party can contribute a plugin without building from source until Phase 4.

**Compliance.** Lint `no-dynamic-loading`; registry rejects unknown IDs, version mismatches, hash mismatches; Phase 4 gate.

**Note.** **The `Rule` trait is the load-bearing interface for this decision.** Every method added now must be marshalled across the WASM boundary later. Keeping it small — `apply(&View, RngKey) -> Vec<Delta>` — is what makes that possible. **Resist growth.**

---

## 34.9 ADR 0009 — Research question revision after Phase 0 literature verification

**Context.** §26.2 required, as the first Phase 0 deliverable, a check on whether a computational model of threat-rigidity already existed, with the instruction that a close match would shift the contribution before any code was written. The check fired twice (§36).

*First:* the BTOF and threat-rigidity theses make opposite predictions about the same observable, both extensively supported, with the contradiction attributed in the literature to search dynamics not being directly observable. This suggested a sharper question than the original: are the theories responding to two distinct, confounded stimuli — aspiration-relative shortfall and boundary-relative viability proximity?

*Second:* a mandated follow-up search found **this dissociation is not novel.** March & Shapira (1992) proposed exactly two reference points with opposite consequences; Audia & Greve (2006) tested it empirically; Miller & Chen (2004) tested it with mixed results `[E]`. Further, **March & Shapira (1992) is itself a formal random-walk model, not a verbal theory** — so FIRMA cannot claim to be "the first formalisation" either.

**Decision.** **Adopt the revised question and hypotheses (§2), with the narrowed contribution claim (§31.2).**

Three narrow gaps survive: **search breadth rather than risk-taking as DV**; **simultaneous operation rather than attention-switching**, testable via H1c; and the **endogenous survival point**, which no prior model represents and which is the largest surviving gap.

**New validation test VT-8** (§25.4): $h$ and $\varsigma$ must be independently manipulable. Without it, H1a and H1b are guaranteed by construction and E1 is void.

**Alternatives.** *Keep the original question* — weaker; no named contradiction, no established parent. *Keep the intermediate framing (dissociation as novel)* — **not available; it is not novel**, and publishing it as such would be caught in review and is not honest. *Abandon the project* — disproportionate; three genuine gaps remain, one squarely FIRMA's own. *Pivot entirely to the endogenous-survival-point question* — most novel element, and tempting; **rejected because the dissociation is the foundation that mechanism modifies**, and testing the modification without establishing the base relationship in this model would leave the result uninterpretable. Retained as the emphasis for the *second* paper.

**Consequences.** *Positive:* an **established theoretical parent** rather than a novel conjecture — reviewers trust formalisations of known theory more than simulations of new ones, so this is a stronger publication position despite the narrower claim; H1a/H1b failing would now contradict an established model, a *more* interesting result. *Negative, accepted:* the contribution is materially narrower than either earlier framing; publication ambition recalibrated (CMOT/Strategy Science first; *Organization Science* only for a second paper). *Neutral:* **no engineering change.** Both required variables already existed in the model.

**Compliance.** §2 and §31 revised; VT-8 added and made a hard precondition in §30.9; **new standing rule in §39C:** any finding that substantially raises the project's apparent value triggers a mandatory adversarial literature search before being written into any document as a contribution claim.

**Note.** The Phase 0 gate justified its cost twice within a single day, at a stage where revision costs only document edits. Had either finding arrived after the kernel was built, the cost would have been months; after publication, a retraction risk. **This ADR should be cited whenever anyone proposes shortening Phase 0.**

---

# PART VII — TRANSLATION CONTRACTS

# 35. Contracts

**Rules.** Any Tier 2 claim (§4.1) requires a contract. Contracts are versioned and content-hashed and MUST be cited by ID in any publication using them. Six fields: model mechanism, abstract mechanism, organisational construct, preserved, discarded, introduced, known non-transfer with empirical scope condition.

| § | ID | Title | Used by |
|---|---|---|---|
| 35.1 | TC-001 | Viability margin → proximity to organisational failure | H1b, H1c, H3 |
| 35.2 | TC-002 | Search width → organisational information processing | H1a, H1b, H1c, H2 |
| 35.3 | TC-003 | Constraint-shaping → nonmarket strategy | H3, H5 |
| 35.4 | TC-004 | Aspiration shortfall → performance feedback | H1a, H1c |
| 35.5 | TC-005 | Repertoire entropy → behavioural rigidity | H1a–c, H2, H4 |

---

## 35.1 TC-001 — Viability margin → proximity to organisational failure

**Model mechanism.** $h = -\max_j g_j/s_j$ (§9.2).
**Abstract mechanism.** Proximity to a feasibility boundary in a constrained dynamical system, where crossing terminates or fundamentally alters the trajectory.
**Organisational construct.** Proximity to organisational failure: covenant breach, liquidity exhaustion, licence revocation, contractual default.

**Preserved.** Ordinal proximity. Multiplicity — several constraints operate independently. Identity switching — which constraint binds can change. Discontinuity at the boundary, distinguishing proximity-to-failure from poor-performance. **Independence from performance** — a firm can be near a boundary while performing acceptably. **This separation is load-bearing for H1a/H1b and is why VT-8 exists.**

**Discarded.** Managerial perception. Reporting lags. Strategic misrepresentation. Stakeholder heterogeneity. **Negotiability at the boundary** — real covenant breaches are frequently waived, real violations settled; FIRMA's boundaries are hard.

**Introduced.** Commensurability via scale factors $s_j$, with no empirical basis for the §9.2 values — `[D]` choices. Exact self-observation (§12.2). Min-aggregation — only the nearest constraint matters; a smooth norm would represent simultaneous multi-constraint pressure differently. Untested.

**Equivalence claim.** **Formal analogy.** Both are distance from a system's state to conditions whose violation terminates normal operation. No mechanistic homology. No claim that firms compute anything resembling $h$.

**Known non-transfer and scope condition.** Real response is driven by *perceived* proximity, systematically biased by optimism, escalation of commitment, and incentive to avoid disclosure, and sometimes deliberately distorted. **Scope: FIRMA's predictions apply to the limiting case of accurate proximity perception. Empirical tests SHOULD target settings where proximity is unambiguous and externally verified** — covenant thresholds, regulatory capital ratios, licence renewal deadlines — **rather than subjective assessments of organisational health or survey measures of perceived threat.** Where perception is inaccurate, predictions should be read as applying to *perceived* margin — a different test that MUST NOT be conflated with the objective one.

---

## 35.2 TC-002 — Search width → organisational information processing

**Model mechanism.** $w_{\text{eff}} = \max(1,\lceil w_{\max}\psi(h)\rceil)$, with inadmissible actions not consuming budget (§12.3).
**Abstract mechanism.** Size of the alternative set actually evaluated by a decision process under a variable evaluation budget.
**Organisational construct.** Breadth of organisational information processing — the range of alternatives generated and considered before commitment. **The precise quantity in threat-rigidity's "restriction in information processing" and in BTOF's "problemistic search."**

**Preserved.** Boundedness — evaluation is costly. Variability with circumstances. Ordering — what is examined late may never be reached. **The generated/available distinction** — which is why inadmissible actions do not consume scan budget (§11.4).

**Discarded.** **Alternative generation.** FIRMA's action set is fixed; real organisations invent alternatives, and the inventive act may be exactly what threat suppresses. **The largest single omission in this contract.** Parallel search across units. Evaluation quality variation. Social process — alternatives advocated by people with interests, absent from a unitary agent (§34.6).

**Introduced.** Integer countability — real organisations have no such quantity. A single scan order per focus (§12.3 Step 4), `[D]` with no empirical basis; AT-4 tests sensitivity. Deterministic smooth narrowing; real restriction is likely lumpy and threshold-driven.

**Equivalence claim.** **Formal correspondence** — closer than most in this project, because the threat-rigidity thesis is itself stated in terms of information-processing restriction.

**Known non-transfer and scope condition.** Real organisations restrict processing partly by *not generating* alternatives. **FIRMA models restriction of evaluation, not restriction of imagination.** Scope: predictions transfer to settings where the alternative set is externally given and enumerable — regulated choice sets, formal procurement, standardised operational or clinical protocols, menu-based strategic options — **not to open-ended strategic invention.** Direction of change transfers; *how many* alternatives a real organisation considers does not, and no calibration would make it so.

---

## 35.3 TC-003 — Constraint-shaping actions → nonmarket strategy

**Model mechanism.** Actions 6–8 modifying $\boldsymbol{\theta}$ rather than state within $K$; costly at commitment, lagged, probabilistic (§11.2–11.3).
**Abstract mechanism.** Agent actions altering the constraint set governing the agent's own admissible state space.
**Organisational construct.** Nonmarket and boundary-shaping strategy: corporate political activity, lobbying, regulatory engagement, standard-setting, long-term contracting, alliance formation, supplier diversification, vertical integration.

**Preserved.** **The categorical distinction** — acting *on* constraints differs in kind from acting *within* them. Sunk cost, empirically accurate for lobbying. Lag. Uncertainty, with $p_{\max}<1$ strictly. Opportunity cost, which is what makes H3 non-trivial. Path dependence via legitimacy — shaping capability is built, not purchased.

**Discarded.** **Contestation.** In the MVP the regulator is passive; real nonmarket strategy is opposed by rivals, NGOs, other regulators, public opinion. **The single largest omission**, and why `regulator.strategic` is a Phase 4 deliverable. Coalition politics — real lobbying is overwhelmingly collective via trade associations. Legitimacy and legality gradients — no distinction between lobbying, capture, and corruption. Information and expertise channels — much real influence operates by supplying technical information, not expenditure. Reputational spillover.

**Introduced.** Additive simultaneity (§16.3 item 3). A single global regulatory parameter. Monotone effect — successful lobbying always relaxes in the intended direction; real regulatory change frequently produces unintended tightening elsewhere.

**Equivalence claim.** **Abstraction** — a deliberately coarse abstraction over a heterogeneous family of real activities. **Weaker than the formal correspondence claimed in TC-002 and should be described as such.**

**Known non-transfer and scope condition.** The MVP models nonmarket strategy as **one-sided**. Contestation may reverse findings: with opposed rivals, expenditure could rise while net effect falls to zero, making shaping a resource sink rather than an escape route. **Scope (MVP only): predictions transfer to settings with a non-strategic or slow-moving regulatory counterparty and weak rival opposition** — early-stage regulation of a novel activity, technical standard setting with a single dominant proponent, bilateral supplier contracting. **Predictions about competitive lobbying MUST NOT be made from MVP results;** that is what Phase 4 exists for, and H5 is flagged accordingly.

---

## 35.4 TC-004 — Aspiration shortfall → performance feedback

**Model mechanism.** $\varsigma_j = A_j - v_j$ with $A_{j,t+1}=A_{j,t}+\alpha(v_{j,t}-A_{j,t})$ (§12.1).
**Abstract mechanism.** Discrepancy between a realised outcome and an endogenously adapting reference level tracking recent experience.
**Organisational construct.** Negative performance feedback relative to **historical** aspiration — the central IV of BTOF and the performance-feedback literature.

**Preserved.** Reference dependence. Adaptivity — the standard moves toward experience, so sustained underperformance eventually becomes "satisfactory"; the model's most behaviourally important property. Multiplicity with sequential attention. **Independence from viability — the dissociation on which H1a/H1b rest, which VT-8 must confirm is not an artefact of construction.**

**The two reference points must be held apart.** March & Shapira's model has *both* an aspiration level and a survival point `[E]`. In FIRMA these are $\varsigma$ (this contract) and $h$ (TC-001). **Conflating them is the specific error this contract exists to prevent**, and it is the error that makes the BTOF/threat-rigidity contradiction look irresolvable in field data where they covary.

**Discarded.** **Social aspiration.** FIRMA models only historical aspiration; real firms also compare to peers, and social and historical shortfall are reported to drive *different* search responses `[P]`. **The most significant omission in this contract.** Goal weighting and politics (§34.6). Aspiration setting as a strategic act. Forward-looking aspiration — evidence suggests firms weight forward-looking factors heavily `[P]`; FIRMA's are purely backward-looking.

**Introduced.** A single adaptation rate $\alpha$ across all goals and firms. Symmetric adaptation — real aspirations plausibly ratchet asymmetrically. Exact self-knowledge of performance.

**Equivalence claim.** **Formal correspondence.** The exponential aspiration update is the standard formulation throughout the performance-feedback literature. Of all contracts here this is closest to a direct implementation of an existing formal model rather than an abstraction of a verbal one.

**Known non-transfer and scope condition.** Empirical studies typically construct a composite of historical and social aspiration, whose components have been reported to produce different — sometimes opposite — effects. **Scope: predictions transfer to settings where historical aspiration dominates** — firms without close comparable peers, novel markets, non-public organisations without published benchmarks, or designs that decompose shortfall and test the historical component separately. **A comparison with social aspiration MUST NOT be made from MVP results.** Adding social aspiration is a small model change (compare $v_j$ to a peer mean) and a strong Phase 4 candidate precisely because the literature predicts a divergence FIRMA could then test.

---

## 35.5 TC-005 — Repertoire entropy → behavioural rigidity

**Model mechanism.** $H_{\text{rep}}$ over the trailing action window, reported with $\tilde{H}_{\text{rep}}$, $|A^{\text{used}}|$, $|A^{\text{adm}}|$ (§14.2).
**Abstract mechanism.** Concentration of a realised behavioural distribution over a discrete action alphabet.
**Organisational construct.** Behavioural rigidity — reliance on habitual, well-learned responses and reduced behavioural variety.

**Preserved.** Variety as the object — rigidity concerns the *range* of behaviour, not its content or quality. Realised, not intended — measured from what was done. Availability separated from use. Window dependence made explicit — $L_W$ is declared and swept, not a hidden analytic choice.

**Discarded.** **Centralisation.** The thesis predicts *both* narrowed information processing *and* constriction of control. **FIRMA's unitary agent cannot represent constriction of control at all** (§34.6). **This contract covers one of the thesis's two limbs.** Action magnitude — a firm doing the same thing more intensely registers identical entropy. Novelty of action — entropy over a fixed alphabet cannot detect that an organisation stopped doing genuinely *new* things. Content — a firm shifting from three exploratory to three exploitative actions shows no entropy change.

**Introduced.** A discrete, fixed, equally-weighted action alphabet. **Entropy is sensitive to alphabet granularity** — splitting one action into two sub-actions changes the measure without changing behaviour. **The main construct-validity hazard here, and why R2 and R3 are reported alongside R1** (§14.5). Equal salience. Log base 2, declared for reproducibility.

**Equivalence claim.** **Abstraction with a known measurement hazard.** Defensible as a proxy for behavioural variety, but it is one of three measures precisely because no single measure of rigidity is trustworthy alone.

**Known non-transfer and scope conditions.** *First and most important:* FIRMA measures repertoire narrowing, **not centralisation of control.** Empirical comparison MUST target the information-restriction and behavioural-variety limbs and **MUST NOT be presented as a test of the centralisation prediction.** If real threat-rigidity is substantially a centralisation phenomenon, FIRMA measures a correlate, not the construct. *Second:* entropy over a fixed alphabet cannot detect suppression of novelty. Empirical proxies SHOULD be variety measures over a **fixed, externally defined action taxonomy** — patent classes, product categories, procurement lines, documented procedure invocations — not counts of novel actions. **Scope: settings where the behavioural alphabet is externally defined and stable across the observation window. Where the alphabet itself changes, entropy comparisons across periods are not meaningful and MUST NOT be made.**

---

# PART VIII — LITERATURE VERIFICATION

# 36. Phase 0 literature verification report

## 36.1 Verdict

**PROCEED — with a contribution narrower than initially claimed, but better grounded.**

The check fired twice and materially changed the project both times. **This is the process working, not failing** (§34.9).

## 36.2 Finding 1 — Threat-rigidity citation verified

| Field | Verified |
|---|---|
| Authors | Barry M. Staw, Lance E. Sandelands, Jane E. Dutton |
| Title | Threat-rigidity effects in organizational behavior: A multilevel analysis |
| Journal | *Administrative Science Quarterly* 26(4):501–524 |
| Year | 1981 (December) |
| DOI | 10.2307/2392337 |
| Citations | ~3,500 |

Core claim verified against the abstract: a general threat-rigidity effect across individual, group, and organisational levels, summarising evidence of **restriction in information processing** and **constriction of control** under threat. **Critical qualifier confirmed:** the functional/dysfunctional split turns on **whether the threat is a known, anticipated, or trained condition** — validating the novelty manipulation in §13.3 as the moderator the thesis itself names. `[E]`

## 36.3 Finding 2 — No direct computational model of threat-rigidity

Searched: threat-rigidity × {agent-based, simulation, formal model, computational model, NK landscape, search narrowing, crisis, firm decision-making}.

Extensive empirical threat-rigidity literature, ongoing through 2025–26. Organisational ABM generally (Carley; Repast/MASON work). NK search models. The IDEA model (*AMR*) as the closest computational prior art on problemistic search — performance feedback without a survival boundary.

`[P]` Absence from a targeted search is not proof of absence. But **threat-rigidity appears not to have been formalised as a generative computational model.** Alone, however, this is a weak contribution — "nobody has simulated X" invites "because simulating X is not interesting."

## 36.4 Finding 3 — The BTOF / threat-rigidity contradiction

| | BTOF | Threat-rigidity |
|---|---|---|
| Trigger | Performance below aspiration | Threat |
| Search | **Increases** | **Decreases** |
| Status | `[E]` Extensively supported | `[E]` Extensively supported |

Billinger, Stieglitz & Schumacher (*Organization Science*, 2013) found experimentally that success narrows search toward the status quo while **failure promotes more exploratory search** — the BTOF direction `[E]`.

A 2023 *JPART* treatment attributes the contradiction's persistence to prior research emphasising the organisational level while **decision-makers' search dynamics are not directly empirically observed**, so disjointed analyses cannot examine whether both mechanisms are simultaneously at play — leaving reconciliation "unattainable" by those means `[E]`.

## 36.5 Finding 4 — The dissociation has prior art

The follow-up search mandated by Finding 3's caution returned a hit.

**March & Shapira (1992), *Psychological Review* 99(1):172–183** — verified exactly, including volume, issue, pages, and January 1992 publication; ISSN 0033-295X; PsycNET record 1992-15277-001. Uses random-walk conceptions of performance, aspirations, and risk preferences to model risk-taking responsive to changing fortune, exploring the impact of **adaptive aspirations and attention focus**. Two reference points — aspiration level and **survival point** — with focus on the survival point producing *decreasing* risk-taking as shortfall grows, and focus on the aspiration level producing *increasing* risk-taking. `[E]`

**Audia & Greve (2006), *Management Science*** — "Less Likely to Fail: Low Performance, Firm Size, and Factory Expansion in the Shipbuilding Industry." Conflicting predictions hinge on whether decision-makers perceive negative performance as a **repairable gap** or a **threat to firm survival**; resource-buffered firms take more risk under shortfall, resource-limited firms less. `[E]`

**Miller & Chen (2004), *Academy of Management Journal* 47(1):105–115**, DOI 10.2307/20159563 — "Variable Organizational Risk Preferences: Tests of the March–Shapira Model." Found variables affecting risk, and effect *sizes* but not *signs*, differed across performance categories, with **poorly performing organisations showing increased risk as they neared bankruptcy** — partially contrary to the survival-focus prediction. `[E]`

## 36.6 Honest assessment of the damage

**Two claims were overstated and are withdrawn.**

1. The dissociation is **not novel**. It was stated in 1992 and tested repeatedly since.
2. FIRMA is **not "the first formalisation."** March & Shapira (1992) is itself a formal random-walk model, not a verbal theory.

Both overstatements lasted hours because the check that caught them was built into the process. Caught by a reviewer instead, the cost would have been a rejected paper.

## 36.7 What survives

Three genuine gaps, each narrower than initially claimed, detailed in §31.2: **search breadth rather than risk-taking as DV**; **simultaneous operation rather than attention-switching**; **an endogenous survival point**, the largest surviving gap.

A fourth point strengthens rather than weakens the case: **the parent model's empirical support is mixed** (Miller & Chen found partially contrary results). The two-reference-point *structure* is established; its behavioural *consequences* are not settled. This is a live question, not a closed one.

## 36.8 Process note

The caution recorded before the follow-up search read: *"This finding makes the project look considerably more promising than it did this morning. That is exactly when to be careful."* It was correct, and the search it mandated found the prior art in one query.

**Standing rule, now in §39C: any finding that substantially raises the project's apparent value triggers a mandatory adversarial literature search before it is written into any document as a contribution claim.**

## 36.9 Remaining verification obligation

A systematic review is not required for Phase 0 but **is required before submission.** Individual empirical threat-rigidity studies cited in passing, and the full reconciliation literature (including the "shifting focus of attention" inverted-U models), have not been exhaustively verified.

---

# APPENDICES

# 37. Appendix A — Notation

| Symbol | Meaning | § |
|---|---|---|
| $\mathbf{x}_{i,t}$ | State of agent *i* at tick *t* | 8.1 |
| $X_t$ | Global state | 6.3 |
| $\boldsymbol{\theta}_t$ | Constraint parameters | 8.2 |
| $\mathbf{e}_t$ | Environment state | 8.3 |
| $\pi^{I}, \pi^{O}$ | Input price, output price | 8.3 |
| $\Sigma_t$ | Set of active shocks | 8.3 |
| $G_t$ | Relation graph | 8.3 |
| $r^L, r^I$ | Liquid capital, input stock | 8.1 |
| $c$, $q$ | Capability, outstanding obligation | 8.1 |
| $\lambda$, $u$ | Legitimacy, regulated-activity intensity | 8.1 |
| $K(\boldsymbol{\theta})$ | Constraint set | 9.1 |
| $g_j$, $s_j$ | Constraint *j*, its scale factor | 9.1–9.2 |
| $h$ | Viability margin | 9.2 |
| $\mathrm{Viab}(K)$, $\mathcal{V}_t$ | Viability kernel, time-varying kernel | 9.3–9.4 |
| $U(\mathbf{x})$ | Admissible action set | 11.4 |
| $\mathbf{a}^m$, $\mathbf{a}^c$ | Market, constraint-shaping actions | 11 |
| $\Delta_c$, $\gamma$ | Shaping lag, constraint-parameter update | 9.4 |
| $A_j$, $v_j$, $\varsigma_j$ | Aspiration, realised value, shortfall | 12.1 |
| $\alpha$ | Aspiration adaptation rate | 12.1 |
| $\psi(h)$, $\beta$, $w_{\text{eff}}$, $w_{\max}$ | Narrowing function, sharpness, effective and max search width | 12.3 |
| $h_{\text{crit}}$ | Survival-attention threshold | 12.3 |
| $D_i$, $D_i^k$ | Dependence, per-resource dependence | 13.1 |
| $\omega_k$ | Criticality weight (**not** $\varsigma_k$ — see warning in §13.1) | 13.1 |
| $b_\lambda$, $b_\kappa$ | Shaping success coefficients (**not** $\beta_\lambda$, $\beta_\kappa$ — see warning in §11.2) | 11.2 |
| $H_{\text{rep}}$, $\tilde{H}_{\text{rep}}$ | Repertoire entropy, normalised | 14.2 |
| $\Pi$, $\kappa$, $\rho$ | Reconciliation, RNG key derivation, a rule | 6.4 |
| $\Phi_c$ | An interventional construct | 6.6 |

## 37.1 Reserved symbols

These three symbols carry one meaning each throughout this manual and MUST NOT be reused, because each collides with a quantity central to the research question.

| Symbol | Reserved for | § | Never used for |
|---|---|---|---|
| $\varsigma$ | Aspiration shortfall — one half of the central dissociation | 12.1 | Criticality weights (use $\omega$) |
| $\beta$ | Narrowing sharpness — the primary independent variable | 12.3 | Regression or success-model coefficients (use $b$) |
| $h$ | Viability margin — the other half of the dissociation | 9.2 | Any other distance or threshold |

A fourth convention: $\sigma$ denotes observation noise only (§12.2). The set of active shocks is $\Sigma_t$ (§8.3).

# 38. Appendix B — Directory layout

```
firma/
├── Cargo.toml
├── crates/
│   ├── firma-core/            # types, traits, IDs. NO logic, NO sibling deps
│   ├── firma-rng/             # counter-based RNG, key derivation
│   ├── firma-kernel/          # state store, scheduler, reconciler, invariants
│   ├── firma-viability/       # kernel computation, margin proxy
│   ├── firma-registry/        # plugin registration and resolution
│   ├── firma-io/              # event log, snapshots, manifests
│   ├── firma-config/          # schema, validation, hashing
│   ├── firma-plugins/
│   │   ├── firma-plugin-locality/
│   │   ├── firma-plugin-resource/
│   │   ├── firma-plugin-constraint/
│   │   ├── firma-plugin-decision/
│   │   ├── firma-plugin-action-market/
│   │   ├── firma-plugin-action-shaping/
│   │   ├── firma-plugin-shock/
│   │   └── firma-plugin-observation/
│   ├── firma-cli/             # run, sweep, replay, verify
│   ├── firma-tui/             # ratatui live monitor
│   └── firma-py/              # PyO3 bindings
├── python/firma_lab/          # orchestration, analysis, statistics, plots
├── configs/
│   ├── schema/                # JSON Schema for all config types
│   └── experiments/           # committed ExperimentSpecs
├── docs/
│   ├── MANUAL.md              # this document
│   └── adr/                   # new ADRs from 0010 onward
└── tests/
    ├── determinism/  ├── validation/  └── integration/
```

**New ADRs from 0010 onward live in `docs/adr/` as separate files and are indexed in §34.0 at the next manual version bump.**

# 39. Appendix C — Checklists

**39A — Before writing code.** ADR exists if required · §5 vocabulary used · determinism implications considered · test written or specified.

**39B — Before running an experiment.** `ExperimentSpec` hashed and committed · hypotheses carry predicted signs · alternative explanation named (§28.3) · analysis plan fixed · VT suite green including VT-8 · ≥100 seeds (≥200 for survival) · null arm included.

**39C — Before publishing.**
- [ ] All operationalisations reported (§14.5)
- [ ] AT suite run and reported
- [ ] Full sweep released including negative regions
- [ ] Claim tier correct (§4.1); no Tier 3 claim from FIRMA alone
- [ ] Translation contract cited by ID for every Tier 2 claim
- [ ] Independent re-implementation done for the headline result (§25.8)
- [ ] Forbidden framings absent (§5.3)
- [ ] Unitary-firm limitation stated explicitly, not generically (§34.6)
- [ ] March & Shapira cited in the first paragraph (§31.3)
- [ ] **Adversarial literature search performed** — any finding that substantially raises the project's apparent value triggers a targeted prior-art search *before* it is written as a contribution claim (§36.8)

# 40. Appendix D — Citation status

**Verified in preparation of this manual (volume, issue, pages, and core claim checked):**
Staw, Sandelands & Dutton (1981), *ASQ* 26(4):501–524, DOI 10.2307/2392337 · **March & Shapira (1992), *Psychological Review* 99(1):172–183**, ISSN 0033-295X, PsycNET 1992-15277-001 · Miller & Chen (2004), *AMJ* 47(1):105–115, DOI 10.2307/20159563 · Audia & Greve (2006), *Management Science*, "Less Likely to Fail" · Billinger, Stieglitz & Schumacher (2013), *Organization Science*, "Search on Rugged Landscapes" · Davis, Eisenhardt & Bingham (2007), *AMR* 32(2):480–499 · Aubin, viability theory (1991, 1997, 2011) · Pfeffer & Salancik (1978/2003) · the *JPART* (2023) treatment of the BTOF/threat-rigidity contradiction · Repast Simphony and MASON · CMOT, Strategy Science, JASSS scope · ratatui, egui/eframe status.

**Cited from background knowledge, NOT re-verified — check before use in any submitted manuscript:**
Cyert & March (1963) · Simon (1947) · Morgan, *Images of Organization* · Nelson & Winter (1982) · Hodgson & Knudsen on generalised Darwinism · Levinthal (1997) · Rivkin (2000) · Ethiraj & Levinthal (2004) · Axtell's firm models and the Zipf result · Dosi/Fagiolo/Roventini K+S · the IDEA model (*AMR*) · Hannan & Freeman (1977) · the "shifting focus of attention" reconciliation literature · the *Organization Science* paper "Motivation and Ability: Unpacking Underperforming Firms' Risk Taking".

**No result, benchmark, or consensus claim in this manual is invented.** Duration and probability estimates are labelled as judgement, not measurement.

---

*End of FIRMA Project Manual v1.0.1*
