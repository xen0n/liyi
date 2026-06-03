<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->
<!-- AIGC: drafted with AI assistance (Claude Opus 4.8 via OpenCode); reviewed by a human maintainer before merge. -->

# The Meeting Host: A Review Agent for the Advisory Profile

**Status:** 🔵 Proposed (exploratory — depends on the advisory profile)
**Target:** post-1.0
**Design authority:** this document
**Related:** `docs/advisory-profile-design.md` (the profile this host serves);
`docs/prompt-mode-design.md` (`liyi check --prompt`, the consumption surface);
`docs/note-context-design.md` (`liyi context`, the retrieval surface);
`docs/liyi-design.md` — *Security model*, *Without review 立意 degenerates*,
*The 立意 teacher challenges the thesis*; `docs/next-steps.md` (challenge mode)

---

## Summary

The advisory profile (`docs/advisory-profile-design.md`) keeps 立意 useful in an
unattended repository, but it cannot manufacture the one thing such a repository
lacks: a reviewer. This document specifies the **meeting host** — an LLM review
agent that stands in the *facilitator's* chair, not the *approver's*. It reads
what changed, what 立意 says is missing or drifting, and what governing context
applies, then produces an agenda, pointed questions, and minutes for the *primary*
coding agent to act on. It **proposes; it never commits, and it never vouches.**

The host is deliberately defined **outside** the deterministic `liyi` binary. 立意's
core guarantee is that the guesser is not the verifier (`docs/liyi-design.md`); a
no-LLM binary that anyone can audit must not grow an LLM inside it. The host is a
*consumer* of the binary's deterministic surfaces, not an extension of them.

This is an honest-but-partial substitute. It is recorded plainly here: a meeting
host **is not a human reviewer** and does not restore the trust property human
review provides. It narrows the gap; it does not close it.

---

## Why a host, and why only a host

立意 without review "degenerates to auto-updating specs" (`docs/liyi-design.md`):
the agent writes intent, the agent updates it when code changes, and nothing is
ever checked against a standard outside the code. The advisory profile mitigates
this by keeping the *prescriptive* pillar — requirements, whose oracle is their
own human-authored prose — gating. But in a vibe-selected repository those
requirements may not exist yet. Someone has to *elicit* them.

A human does this in review by asking "did you consider X?" and "what should
happen when Y?". The meeting host automates the *asking*, not the *deciding*. It
is valuable precisely where it stays a facilitator:

- It **surfaces** what changed against what is governed, so nothing slips by
  unconsidered.
- It **challenges** the implementation against the declared and the *implied*
  design space, eliciting requirements that were never written down.
- It **drafts** the resulting requirements, edges, and notes as proposals.

It is dangerous the moment it stops facilitating and starts approving — at which
point the guesser has become the verifier and the whole convention collapses into
a self-approving loop.

---

## The host is out of the binary

<!-- @liyi:requirement host-is-out-of-binary -->
**The review host is an LLM agent outside the deterministic `liyi` binary, which
remains free of model inference.** All model-driven facilitation — reading diffs,
asking questions, drafting requirement prose — lives in the host, not in `liyi`
or `liyi-lsp`. The binary's role is unchanged: deterministic, auditable, no
network, no model. The host communicates with 立意 only through the binary's
existing surfaces (below); it never embeds 立意's checking logic, and 立意 never
embeds the host's judgment. This preserves the cardinal separation that the
guesser (the LLM that proposes intent and code) must not be the verifier (the
deterministic tool that checks recorded claims).
<!-- @liyi:end-requirement host-is-out-of-binary -->

### Consumption surfaces

<!-- @liyi:requirement host-consumes-deterministic-surfaces -->
**The host's interface to 立意 is the existing deterministic surfaces; it requires
no new binary capability.** The host assembles its working context from four
inputs: the version-control **diff** of the change under review (obtained
externally — 立意 does not own diffing), the structured coverage and staleness
report from `liyi check --prompt` (`docs/prompt-mode-design.md`), the applicable
governing prose from `liyi context <path:line>` (`docs/note-context-design.md`),
and the advisory profile's **coverage gauge**. Diff-to-finding correlation is the
host's responsibility, performed over these deterministic outputs. No
`liyi check --diff` mode or other new binary surface is introduced; keeping
diff-awareness in the host honors the guesser/verifier split and lets the binary
stay whole-tree and deterministic.
<!-- @liyi:end-requirement host-consumes-deterministic-surfaces -->

---

## Authority: propose, never commit, never vouch

<!-- @liyi:requirement host-propose-only -->
**The host proposes changes as diffs and minutes; the primary agent applies them.
The host never writes tracked 立意 state itself.** Requirements, `@liyi:related`
edges, intent prose, and notes that the host derives are emitted as *proposals* —
a diff to apply, an agenda item to resolve — for the primary coding agent to
accept, modify, or reject. The host has no commit authority over the staleness
graph. This bounds the host to the facilitator role and keeps a single locus of
change (the primary agent) that the gauge can hold accountable.
<!-- @liyi:end-requirement host-propose-only -->

Propose-only has an explicit, accepted implication: **it trusts the primary
agent's instruction-following.** The primary agent may decline to apply some or
all of the host's proposals. This is tolerable, not a hole, because of the
gauge-as-backstop:

- A declined proposal leaves the corresponding coverage gap **in place**.
- The gap re-surfaces on the next `liyi check --prompt` and the next gauge
  reading (`docs/advisory-profile-design.md` — *The coverage gauge*).
- The skip is therefore **bounded and visible**, never silent: a primary agent
  that systematically ignores the host shows up as a gauge that fails to improve.

<!-- @liyi:requirement host-never-vouches -->
**The host facilitates and challenges but never substitutes for human review;
intent it elicits is not marked reviewed on its own authority.** The host may
draft requirement prose and propose specs, but it must not set `"reviewed": true`
or otherwise represent its own output as human-vouched. Intent that originates
with the host carries no more authority than agent-inferred intent does in the
classic flow; it is a candidate awaiting whatever review the repository's profile
affords (human approval where available, the gauge and the prescriptive pillar
where not). A host that stamped its own proposals as reviewed would launder
unverified intent into the trusted set — exactly the failure 立意 exists to
prevent.
<!-- @liyi:end-requirement host-never-vouches -->

### Legitimate and illegitimate reviewer agents

An agent in a review seat is legitimate only as a **challenger holding an
external standard** — a different model with different blind spots, a persona
carrying the founder's vision or an industry standard, an adversary probing the
negative design space. Such an agent adds value through *diversity* and
*challenge*, not through *authority*. The illegitimate move is to let any agent —
however role-played as "architect" or "staff engineer" — *vouch*, because no
amount of org-chart cosplay converts a guess into verified intent. Role diversity
helps; role authority launders.

---

## Eliciting requirements: challenge over the design space

The host's highest-value function in a vibe-selected repository is manufacturing
the requirements that were never written down.

<!-- @liyi:requirement host-elicitation-challenge -->
**The host elicits requirements by challenging each item over both the positive
and the negative design space, and classifies the response.** For an item under
review, the host enumerates the behaviors the item *should* and *should not*
exhibit — boundary conditions, error paths, and properties the surrounding context
(notes, neighboring requirements) implies — and surfaces each as a question or a
proposed requirement. Each elicited implication resolves into one of three
outcomes: **capture** (the implication is real and becomes a proposed requirement),
**violation** (the code already contradicts the implication — a bug found early,
raised as a finding), or **genuinely-undecided** (no prior intent exists and the
choice is open — the highest-value case, because it manufactures intent that never
existed rather than echoing the code). Elicitation targets items with reviewed or
source-declared intent first, then risk-bearing items the gauge flags as uncovered;
it does not attempt an exhaustive census.
<!-- @liyi:end-requirement host-elicitation-challenge -->

This extends classic **challenge mode** (`docs/next-steps.md`) from "critique the
implementation against existing intent" to "interrogate the implementation to
discover intent." It is the elicitation engine the prompt-engineer-unattended and
vibe-selector-unattended personas (`docs/advisory-profile-design.md`) both need.

---

## Closing the silent-edge hole

A review that only checks code against the requirements it is *already linked to*
has a blind spot: if the unreviewed author-agent never wrote the `@liyi:related`
edge, the requirement is silently uncovered and the review never notices.

<!-- @liyi:requirement host-proposes-missing-edges -->
**The host proposes missing `@liyi:related` edges, not only checks existing ones.**
The host treats the coverage gaps that `liyi check --prompt` already reports —
`missing_related_edge` (an item that should link a requirement but does not) and
`req_no_related` (a requirement no item references) — as first-class agenda items,
proposing the edge the author-agent omitted. Reviewing code only against its
declared edges would let an unlinked requirement pass unverified; surfacing and
proposing the missing edge is what makes coverage honest under the advisory
profile. The detection is deterministic (it comes from the binary); the host's job
is to *act* on it by proposing the link.
<!-- @liyi:end-requirement host-proposes-missing-edges -->

---

## Residual risk: no human is still no human

The threat model for agent-consumed 立意 output treats repository content as
untrusted and names **diligent human review of inferred intent as the
authoritative security boundary** (`docs/prompt-mode-design.md` — *Security
considerations*). The advisory profile, by definition, operates where that
boundary is absent or thin. The meeting host narrows the resulting gap — it forces
consideration, challenges the design space, and keeps coverage visible — but it
does **not** reconstitute the boundary:

- The host is itself an LLM consuming untrusted repository content; it can be
  misled by adversarial markers or prose (the "browser rendering untrusted
  content" model).
- The host cannot vouch, so nothing it produces is verified in the sense human
  review verifies.

A repository running the advisory profile with a meeting host is therefore
**better governed than an ungoverned vibe-coded repository, and less trustworthy
than one under genuine human review.** That position is the honest one, and it is
stated so that adopters with standing-audit or compliance obligations — who need a
real human boundary — do not mistake the host for one.

---

## Non-goals

- **Not a verifier.** The host does not check claims; the binary does. The host
  asks questions and drafts proposals.
- **Not in the binary.** No part of the host ships inside `liyi`/`liyi-lsp`.
- **Not an approver.** The host never sets `"reviewed": true` or gates anything.
- **Not a diff tool.** Diffing is external; the host correlates 立意's
  deterministic findings with a diff it is given.

---

## Open questions

- The minutes format (does it reuse the `--prompt` JSON shape, or wrap it?).
- Whether elicitation's risk-ordering is seeded by the binary (deterministic
  heuristics) or computed entirely by the host.
- How the host is invoked in practice (PR bot, pre-commit agent, editor agent)
  and where its proposals surface to the primary agent.
