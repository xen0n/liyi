<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->
<!-- AIGC: drafted with AI assistance (Claude Opus 4.8 via OpenCode); reviewed by a human maintainer before merge. -->

# The Advisory Profile: 立意 for Unattended Repositories

**Status:** 🔵 Proposed (exploratory — gated on a decision to build the mode at all)
**Target:** post-1.0 (the `liyi.toml` declaration reverses a pre-1.0 stance; see *Declaration*)
**Design authority:** this document
**Related:** `docs/liyi-design.md` — *Linter behavior*, *Security model*, *Scope
discovery*; `docs/note-context-design.md` — *Two graphs*; `docs/prompt-mode-design.md`;
`docs/lsp-design.md`; `docs/meeting-host-design.md` (the review host that consumes
this profile)

---

## Summary

立意's default ("classic") flow assumes a human reviewer: an agent infers intent,
a human reviews and vouches for it, and that human judgment is what makes the
recorded intent *authoritative* rather than a self-approving echo of the code.
Some repositories have no such reviewer in the loop — unattended, agent-driven,
"vibe-coded" projects where code lands without a human reading every diff.

This document specifies an **advisory profile**: a configuration of the *one*
立意 engine that keeps the parts of the convention that survive the loss of a
human reviewer (the prescriptive pillar — requirements — and the retrieval graph
— notes) and demotes the parts that do not (descriptive item-level staleness)
from a **gate** to a **surfaced signal**. It is a profile, not a fork: the same
binary, the same library, the same `liyi-lsp`. The mode is selected by a
committed, reviewable declaration so that no pipeline can silently slip into
self-vouching.

The advisory profile is deliberately scoped as **post-1.0 and uncertain**. It is
recorded here so the design space is explored and the seams it needs (a storage-
backend abstraction, a coverage gauge, a directional graduation path) are present
in the core design rather than retrofitted later.

---

## Why a separate profile is needed

### The descriptive pillar is circular without a reviewer

立意 tracks two kinds of claim. A **descriptive** item spec says "this function
*should* do X"; a **prescriptive** requirement says "the code must satisfy this
external text." The two degrade very differently when the human reviewer leaves:

- A descriptive intent inferred by an agent and never reviewed by a human is an
  **echo of the code**. Its oracle is the code it was read from, so hashing it
  and gating on drift mostly manufactures churn: when the code changes, the
  agent re-infers, the intent "agrees," and nothing was verified. Without review,
  descriptive *item staleness* is noise, not signal. (See `docs/liyi-design.md`
  — *Without review, 立意 degenerates to auto-updating specs*.)
- A prescriptive requirement's oracle is **its own prose**, authored as an
  axiom/acceptance-criterion the code derives from. That oracle does not vanish
  when the reviewer leaves. Requirement-text drift and the `@liyi:related` edges
  that depend on it remain meaningful even in a fully unattended repo.

So the advisory profile is not "立意 with checking turned off." It is "立意 with
the **prescriptive** pillar still gating and the **descriptive** pillar demoted
to advisory," plus the retrieval graph (notes) which never gated anything to
begin with.

### Who this serves

Unattended repositories split by *how much of their governing intent is
mechanically expressible*, not by developer sophistication:

- **Prompt-engineer-unattended.** Requirements exist but are latent in prompts,
  issues, and chat. The work is *harvesting* them into `@liyi:requirement`
  blocks — the prescriptive pillar applies directly.
- **Vibe-selector-unattended.** Intent was never formalized; the author selected
  outputs by taste. There is no latent requirement to harvest, only behavior to
  describe. These repos lean on notes (retrieval) and on *elicitation* (see
  `docs/meeting-host-design.md`) to manufacture requirements that never existed.

---

## The mode model

### One engine, one binary

The advisory profile is a behavior of the existing `liyi` binary and the
existing `liyi-lsp`, both built on the `liyi` library crate. There is **no
second binary**. A separate `liyi-yolo` artifact was considered and rejected: the
LSP is the real delivery surface for unattended users (who do not type CLI
commands), and it must serve both gated and advisory diagnostics from one process
— two LSPs, or an LSP that shells out to a second binary, buys nothing and
complicates the editor path. Keeping one engine also preserves a free graduation
path (below) and honors `lsp-diagnostics-match-cli` (`docs/lsp-design.md`): the
CLI and the editor read the same profile and report the same severities.

### Declaration

<!-- @liyi:requirement advisory-profile-declaration -->
**The active profile is declared in a committed, reviewable `liyi.toml`; its
absence is the gated default.** The advisory profile is selected by
`profile = "advisory"` in a `liyi.toml` at the repository root. When no
`liyi.toml` is present, or it does not set `profile`, the engine runs the gated
(classic) flow — so the default path remains zero-config and the classic adopter
sees no new file. Because the declaration is committed and version-controlled, a
downgrade from gated to advisory is a reviewable diff, never a silent invocation.
A transient mechanism (CLI flag, environment variable, uncommitted sentinel) must
not be the source of truth for the profile, because none of them appear in a
diff; such mechanisms may only *tighten* the committed profile (advisory →
gated), never loosen it. This keeps the no-laundering guarantee — unreviewed
intent cannot reach a gated pipeline without a visible, reviewable change.
<!-- @liyi:end-requirement advisory-profile-declaration -->

This reverses, narrowly, the "no config file" stance recorded in
`docs/liyi-design.md` (*Scope discovery* / *Non-goals*). The reversal is
deliberately bounded and deferred to **post-1.0** for two reasons: it is not yet
certain the advisory profile will be built at all, and — more importantly — it
does not contradict the *deeper* intent behind that stance, which was that the
**default, classic flow require no configuration** for minimal adoption friction.
That invariant survives intact: `liyi.toml` is an *opt-in marker for a departure
from the default*, read only when present, and the unmarked classic repository
still reads no config. In v1 of this profile, `liyi.toml` carries **policy/mode
only** (`profile`); file selection stays in `.liyiignore`, kept orthogonal so the
config surface does not creep.

Why `liyi.toml` over the alternatives considered:

| Surface | Committed & diff-visible | Cannot be silently flipped | Extensible | Verdict |
|---|---|---|---|---|
| `liyi.toml` | yes | yes | yes | **chosen** |
| `.liyi-mode` sentinel | yes | yes | no (one bit; litters dotfiles) | dominated |
| `LIYI_*` env / `.envrc` | no (per-machine, gitignored) | **no** | n/a | rejected as source of truth |

### Recognition and rejection are profile-keyed

<!-- @liyi:requirement advisory-graduation-direction -->
**Profile tolerance is directional: gated rejects advisory-only metadata, while
advisory honors classic reviewed metadata.** The gated profile treats any
advisory-only metadata it recognizes (e.g. inline-hash annotations, once that
backend exists — see `docs/inline-hash-backend-design.md`) as an error, refusing
to launder unreviewed constructs into a gated pipeline. The advisory profile, by
contrast, fully honors classic metadata: a `"reviewed": true` spec, a sidecar, a
requirement edge all keep their meaning, so a repository can accumulate reviewed
intent while running advisory and then *graduate* to gated without rewriting its
specs. Tightening (advisory → gated) is always available; loosening is only ever
a reviewable `liyi.toml` change. The recognition logic lives in the shared
library (it must parse the full metadata grammar to detect advisory-only
constructs); only the *policy* — act-on versus reject — is profile-specific.
<!-- @liyi:end-requirement advisory-graduation-direction -->

---

## Profile semantics

Relative to the gated default, the advisory profile changes two engine knobs and
nothing else:

- **`inference = off`.** The engine never demands that every item carry a spec
  and never treats a missing descriptive spec as a defect. (Inference inflation —
  spending effort to spec trivial items while missing the high-value ones — is a
  known failure mode; `docs/liyi-design.md` — *Inference inflation*.)
- **`staleness = advisory`.** Descriptive *item* staleness (STALE / SHIFTED /
  UNREVIEWED on item specs) is reported but **does not gate** the exit code.

<!-- @liyi:requirement advisory-staleness-scope -->
**Demotion to advisory applies to descriptive item staleness only; prescriptive
requirement drift still gates.** Under the advisory profile, STALE / SHIFTED /
UNREVIEWED diagnostics on *item specs* are emitted as warnings and do not affect
the exit code. Requirement-side diagnostics — a `@liyi:requirement` block whose
text changed (REQ-CHANGED) and the `@liyi:related` edges that depend on it,
together with the coverage gaps `missing_requirement_spec`, `missing_related_edge`,
and `req_no_related` — retain their gated severity, because the requirement's
oracle is its own human-authored prose and survives the absence of item review.
Demoting item staleness must not silently demote requirement drift; the two are
separately configured and the advisory profile changes only the former.
<!-- @liyi:end-requirement advisory-staleness-scope -->

This is the crux of why the profile is coherent: it does not abandon
verification, it **narrows verification to the claims that are still verifiable**
without a human in the loop.

---

## Gates

An unattended repository still needs an honest floor — conditions under which the
tool refuses to pretend it is providing value.

<!-- @liyi:requirement advisory-gate-vcs -->
**Gate 0: a repository not under version control fails the advisory profile.** The
advisory profile's entire safety story rests on declarations and downgrades being
*reviewable diffs* (per `advisory-profile-declaration`). A working tree that is
not under version control has no diff surface, so none of those guarantees hold.
When the advisory profile is requested for a tree with no VCS, the engine fails
with a clear diagnostic rather than running in a mode whose invariants it cannot
uphold.
<!-- @liyi:end-requirement advisory-gate-vcs -->

<!-- @liyi:requirement advisory-requirement-bootstrap -->
**Gate 1: a repository with zero requirements triggers a risk-ordered bootstrap
to at least one, not a comprehensive census.** With `inference = off`, an
advisory repository can legitimately carry few or no descriptive item specs — but
a repository with *zero* requirements has no prescriptive pillar at all, leaving
the profile with nothing to gate and no anti-commodity-decay anchor. In that
state the engine directs the operator (or the review host) to bootstrap
requirements in risk-priority order, stopping at the honest floor of **at least
one** requirement; it does **not** demand comprehensive coverage. Whether the
bootstrapped set is *adequate* is surfaced by the coverage gauge (below), not
gated — gating adequacy would resurrect the inference-inflation failure mode this
profile exists to avoid.
<!-- @liyi:end-requirement advisory-requirement-bootstrap -->

---

## The coverage gauge

Turning item staleness from a gate into a warning removes a number the project
used to watch. The advisory profile replaces it with a different instrument.

<!-- @liyi:requirement coverage-gauge-surfaced -->
**Coverage density is a surfaced metric, never a gate.** The engine reports how
much of the codebase's governing intent is captured — requirement count, the
fraction of risk-bearing items reachable through `@liyi:related` edges, and
unaddressed coverage gaps — as a *gauge* the operator and the review host can
watch, but it never affects the exit code under the advisory profile. The gauge
is the profile's anti-commodity-decay instrument: a repository whose captured
intent thins out toward zero is sliding back toward an ungoverned vibe-coded
state, and the gauge makes that visible without forcing a gate that would punish
legitimately sparse-but-stable code. Adequacy is a judgment the gauge *informs*,
not a threshold the tool *enforces*.
<!-- @liyi:end-requirement coverage-gauge-surfaced -->

The gauge is also what makes the review host's "propose-only, never commit" stance
safe (`docs/meeting-host-design.md`): if a primary agent declines to apply a
proposed requirement or edge, the gap persists and re-surfaces on the next gauge
reading — the skip is bounded and visible, never silent.

---

## Storage-backend seam

<!-- @liyi:requirement storage-backend-seam -->
**Staleness-graph records are defined against an abstract spec-record, with the
`.liyi.jsonc` sidecar as the v1 serialization.** The engine operates on an
abstract notion of a *spec record* (item or requirement: name, span, intent or
requirement text, edges, tool-managed hashes) rather than hard-coding the sidecar
file format. The co-located `.liyi.jsonc` sidecar is the canonical serialization
for both the gated and advisory profiles in v1. Defining the seam now lets a
future advisory-only serialization — inline-hash annotations carried in source,
for sidecar-free repositories (see `docs/inline-hash-backend-design.md`) — be
added as a second serialization of the same record without forking the engine.
The merge, recovery, and hashing logic is written against the abstract record so
it is shared across serializations; only read/write adapters differ.
<!-- @liyi:end-requirement storage-backend-seam -->

The inline-hash backend is **deferred and optional**; the advisory profile is
fully usable on sidecars alone. See *Note C* (`docs/inline-hash-backend-design.md`)
for why it is sequenced separately.

---

## Relationship to the two-graph model

`docs/note-context-design.md` separates the **staleness graph** (item specs and
requirements, hash-anchored, gating) from the **retrieval graph** (notes, untracked,
best-effort, never gating). The advisory profile maps cleanly onto that split:

- The **retrieval graph** is already exactly the posture an unattended repo wants
  for descriptive context — untracked, never gating. Broad governing prose belongs
  in notes regardless of profile; the advisory profile changes nothing here.
- The **staleness graph** is where the profile acts: it keeps requirement nodes
  and their edges gating, and demotes descriptive item nodes to advisory.

So the advisory profile is, in two-graph terms, "the full retrieval graph, plus
the *prescriptive half* of the staleness graph still gating, plus the descriptive
half demoted to a surfaced gauge."

---

## Non-goals

- **Not a way to silence 立意.** The profile narrows gating to what stays
  verifiable; it does not turn checking off. A repo that wants no 立意 should not
  adopt 立意, not run it in advisory mode.
- **Not a redefinition of core 立意.** Classic semantics are unchanged; this is a
  configuration of the same engine.
- **Not perceptual/aesthetic verification.** Taste-selected UI/UX correctness is
  out of scope (consistent with `docs/liyi-design.md`); the profile governs
  expressible intent, not look-and-feel.

---

## Open questions

- Exact gauge formula and presentation (CLI summary line, LSP status, both).
- Whether Gate 1's "risk-ordered" priority is computed by the engine
  (deterministic heuristics over the dependency/diff surface) or delegated
  entirely to the review host.
- Whether a future `liyi.toml` should absorb `.liyiignore` semantics, or keep
  file-selection permanently separate from mode.
