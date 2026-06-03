<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->
<!-- AIGC: drafted with AI assistance (Claude Opus 4.8 via OpenCode); reviewed by a human maintainer before merge. -->

# Inline-Hash Storage Backend (Deferred)

**Status:** 🟡 Deferred / optional (a future serialization, not scheduled)
**Target:** post-1.0, after the advisory profile
**Design authority:** this document
**Related:** `docs/advisory-profile-design.md` — *Storage-backend seam*,
*Recognition and rejection*; `docs/sidecar-merge-design.md`; `docs/liyi-design.md`
— *Quoting and injection* (note: "injection" there is tree-sitter grammar
embedding, unrelated to writing into source)

---

## Summary

The advisory profile (`docs/advisory-profile-design.md`) defines a storage-backend
seam: staleness-graph records are an abstract *spec record*, with the
`.liyi.jsonc` sidecar as the v1 serialization. This document sketches a **second
serialization** for the advisory profile — staleness state carried **inline in
source comments** so an unattended repository can run **sidecar-free**.

It is recorded as **deferred and optional**. The advisory profile is fully usable
on sidecars alone; inline hashes are a convenience whose cost/benefit, after the
analysis below, does not justify building it for v1. The design is written down so
the seam in `docs/advisory-profile-design.md` is shaped to admit it later without
an engine fork.

---

## The idea

立意 already supports the relevant source annotations: `@liyi:intent` (descriptive
intent in source), `@liyi:requirement` / `@liyi:end-requirement` (a tracked
requirement block), and `@liyi:related` (a dependency edge). The only staleness
state that lives *exclusively* in the sidecar today is the set of tool-managed
hashes. Inline-hash mode carries those hashes in the annotation itself, e.g.:

```text
// @liyi:related billing-currency hash=sha256:1c387d16...
```

With intent expressible via `@liyi:intent`, requirements via the block markers,
and edge-acknowledgment hashes inline, a repository needs **no `.liyi.jsonc` file
at all**.

---

## What is actually worth persisting in an unattended repo

Inline-hash mode is only attractive because the advisory profile already let go of
most per-item hashing.

<!-- @liyi:requirement inline-hash-requirement-scoped -->
**Only requirement-side hashes are persisted inline; the descriptive item
`source_hash` is dropped.** Under the advisory profile, descriptive item staleness
is advisory noise (its oracle is the code), so the item `source_hash` carries no
gating signal and the inline backend simply omits it. What remains worth
persisting is requirement-side: the requirement-block baseline hash and the
per-edge acknowledgment hash on `@liyi:related`, both of which gate because the
requirement's oracle is its own human-authored prose. This is what keeps the
inline footprint small — it stores the prescriptive pillar's hashes and nothing
else.
<!-- @liyi:end-requirement inline-hash-requirement-scoped -->

---

## The decisive cost: the tool must write into source

<!-- @liyi:requirement inline-hash-tool-writes-source -->
**Realizing inline hashes requires 立意 to write into source files — a capability
the tool does not have today — and a self-referential exclusion rule.** Today the
tool writes only sidecars; source files are author-owned and tool-read-only. (The
"injection" framework in 立意 is tree-sitter grammar embedding, not source
mutation.) Inline hashes mean `liyi check --fix` edits source comments to insert
and update `hash=` tokens, introducing a new risk surface: per-language comment
syntax, idempotent formatting, and injection safety. It also requires a
self-referential exclusion rule — the hashed span must exclude the `hash=` token
itself, or writing the hash would change the content it hashes. Because the hash
is still tool-managed (agents produce wrong hashes), there is no way to keep
hashes inline *and* keep the tool out of source; the two are in tension and inline
mode resolves it toward tool-writes-source.
<!-- @liyi:end-requirement inline-hash-tool-writes-source -->

This is the primary reason the backend is YOLO-only and deferred: a classic,
human-reviewed repository wants its source author-owned and its hashes in
tool-managed sidecars, kept apart from the human's reviewed prose.

---

## Merge: still conflicts, still needs isomorphic resolution

A motivating hope for inline hashes was dodging sidecar merge pain
(`docs/sidecar-merge-design.md`). That hope is only partly realized.

<!-- @liyi:requirement inline-hash-merge-isomorphism -->
**Inline annotations still conflict on merge and require a structurally-isomorphic
resolution shared with the sidecar backend.** Two branches editing the same source
region will conflict on inline `hash=` tokens exactly as they would on sidecar
fields. The resolution must be the same structure-aware operation — re-derive
tool-managed fields from the merged source against the abstract spec record — but
now performed over host-language source comments rather than dedicated JSONC. That
is *harder*, not easier, than the sidecar merge (it must parse the host language to
locate annotation regions), so inline mode trades "no separate files" for "a more
complex merge driver." The merge logic is defined once against the abstract spec
record and reused across serializations; only the read/write adapter differs.
<!-- @liyi:end-requirement inline-hash-merge-isomorphism -->

So the surviving benefits of inline hashes are modest: a cleaner working tree (no
sidecar files) and diff locality (the edge and its hash travel in the same hunk as
the code). They do **not** include freedom from structure-aware merging.

---

## Classic rejects it; advisory owns it

<!-- @liyi:requirement inline-hash-is-advisory-only -->
**Inline-hash annotations are advisory-profile-only metadata; the gated profile
recognizes and rejects them.** Inline hashes are a serialization of the advisory
profile and are not valid under the gated (classic) profile, which keeps source
author-owned and hashes in sidecars. Per the directional tolerance in
`docs/advisory-profile-design.md` (*Recognition and rejection*), the gated profile
parses inline-hash annotations (recognition lives in the shared library) and
treats them as an error rather than silently honoring advisory-only state. This is
the construct that makes the gated profile's rejection machinery load-bearing:
without inline hashes, the two profiles differ only in policy, not in metadata
vocabulary.
<!-- @liyi:end-requirement inline-hash-is-advisory-only -->

---

## Why deferred

- The advisory profile works fully on the sidecar backend; inline hashes add no
  capability, only a storage convenience.
- The convenience is reduced by the merge analysis above (no merge-pain dodge).
- It introduces tool-writes-source — a genuinely new risk surface — that deserves
  its own focused design and review rather than riding along with the profile.

If built, it attaches as the second serialization of the spec record defined by
`storage-backend-seam` (`docs/advisory-profile-design.md`), reusing the shared
merge/recovery/hashing logic, with only read/write adapters added.

---

## Open questions

- Per-language placement rules for the inline `hash=` token (which comment, where
  relative to the annotated item) and the exact span-exclusion algorithm.
- Whether a repository may mix backends (some items inline, some in sidecars) or
  must choose one per repo.
- The migration path inline ↔ sidecar, and how graduation to the gated profile
  rewrites inline state into sidecars.
