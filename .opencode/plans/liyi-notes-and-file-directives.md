<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->
<!-- Status: TEMPORARY working artifact. Apply into docs/ then delete. -->

# Plan: `@liyi:file` directives + `@liyi:note` context primitive

This file captures the converged design from the planning session. The active
permission config blocks edits to `docs/`, so the finished content is staged
here for the maintainer to apply (or to grant edit access so the agent can
write the files directly).

## Apply checklist

1. Create `docs/file-directive-design.md` (verbatim from §1 below) + its
   sidecar `docs/file-directive-design.md.liyi.jsonc` (§1a).
2. Create `docs/note-context-design.md` (§2) + sidecar (§2a).
3. `liyi-design.md`: non-destructive forward-pointers (§3) — full module-section
   rewrite is deferred to land *with* the implementation (rename is not yet
   shipped; the spec must not describe unimplemented markers as current).
4. `AGENTS.md`: only the safe additive change now — enumerate
   `@liyi:end-requirement` (§4). Marker-vocabulary swaps to `@liyi:note`/
   `@liyi:file` land with the code.
5. `docs/next-steps.md`: backlog entries (§5).
6. Run `liyi check --fix` to fill sidecar hashes, then `make lint-short`.

## Converged decisions (record)

- **`@liyi:file`** namespaced file-scoped directive: `language=<lang>` (overrides
  extension detection; precedence inline > injection > extension), `ignore`
  (discovery-level exclusion, additive to `.liyiignore`). English sub-keys.
- **`@liyi:note` / `@liyi:end-note`** replaces `@liyi:module` (**hard rename**).
  A *note* is governing/context prose. **Marker-only, untracked, zero sidecar
  footprint**, discovered by live scan. End-marker required for embedded blocks,
  optional for a dedicated file. Optional name; names are group labels (not
  unique IDs) — same-name blocks aggregate.
- **`@liyi:see <name>`** item-side membership: pull a named note into a
  location's context. Multi-valued, opt-out sentinel, live-scanned, never stored
  in a sidecar. A best-effort retrieval hint.
- **Two-graph model:** a best-effort *retrieval graph* (notes + `see`,
  untracked, no staleness, never gates trust) is separated from the hash-anchored
  *staleness graph* (items + requirements). `@liyi:related` from a note to a
  requirement is allowed (an untracked retrieval hint).
- **Verification:** challenge mode (deferred) is secondary. Primary value is the
  guaranteed context-injection mechanism. Near-term consumer: a
  `liyi context <path:line>` CLI; LSP/MCP context APIs are the eventual home.
- **Dropped earlier ideas:** `reviewed`/`authorship` on notes (trust via review
  has low value and is not the distinguishing axis; provenance lives in VCS +
  AIGC trailers); hash-tracking of notes; `moduleSpec` / `in_modules` sidecar
  fields (notes are marker-only, so the JSON schema needs **no** change).

---

## §1 — docs/file-directive-design.md

<!-- BEGIN FILE: docs/file-directive-design.md -->
```markdown
PLACEHOLDER — see the full text block below the fence note.
```

NOTE: the full doc text is reproduced in the companion section "§1-TEXT" at the
end of this file (kept outside a fenced block there so it can be copied with its
own internal code fences intact).

---

## §1a — docs/file-directive-design.md.liyi.jsonc

```jsonc
// liyi v0.1 spec file
{
  "version": "0.1",
  "source": "docs/file-directive-design.md",
  "specs": [
    { "requirement": "file-directive-namespace", "source_span": [0, 0] },
    { "requirement": "file-language-precedence", "source_span": [0, 0] },
    { "requirement": "file-ignore-additive", "source_span": [0, 0] }
  ]
}
```

`source_span` values are placeholders: after creating the doc, set each to the
`[start, end]` line range of its `@liyi:requirement … @liyi:end-requirement`
block, then run `liyi check --fix` to fill `source_hash`/`source_anchor`.

---

## §2a — docs/note-context-design.md.liyi.jsonc

```jsonc
// liyi v0.1 spec file
{
  "version": "0.1",
  "source": "docs/note-context-design.md",
  "specs": [
    { "requirement": "note-is-untracked", "source_span": [0, 0] },
    { "requirement": "note-directory-scope", "source_span": [0, 0] },
    { "requirement": "note-see-membership", "source_span": [0, 0] },
    { "requirement": "two-graph-separation", "source_span": [0, 0] },
    { "requirement": "context-resolution", "source_span": [0, 0] }
  ]
}
```

---

## §3 — liyi-design.md (non-destructive forward-pointers)

Do **not** rewrite the module section to present `@liyi:note` as current — the
rename is unshipped. Instead:

- At the top of *Module-level: `@liyi:module` marker* (≈ line 216) insert an
  admonition:

  > **Redesign proposed (v0.2).** Module-level intent is being replaced by the
  > `@liyi:note` context primitive — a marker-only, untracked governing-prose
  > node consumed by a context-injection surface rather than the staleness
  > engine. `@liyi:module` will be **hard-renamed** to `@liyi:note`. See
  > `docs/note-context-design.md`. The description below documents the *current
  > (v0.1)* behavior until the rename lands.

- At *Marker normalization* alias-table mention (≈ line 1083) and the linter
  behavior tables (≈ line 1581), add a parenthetical pointer that
  `@liyi:end-module`/`@liyi:note`/`@liyi:see` are specified in
  `docs/note-context-design.md` (proposed).

## §4 — AGENTS.md (safe additive change only)

In the template block's directive references, enumerate the existing closing
marker `@liyi:end-requirement` alongside `@liyi:requirement` (rule 4 and the
"annotation comments" list in the design). This marker is already implemented;
this is documentation parity only. Do **not** swap `@liyi:module`→`@liyi:note`
or add `@liyi:file`/`@liyi:see` to the template until the code recognizes them.

## §5 — docs/next-steps.md backlog

Add to Tier 2 (moderate effort):

- **`@liyi:file` file-scoped directives** — `language=` override + discovery-level
  `ignore`. Design authority: `docs/file-directive-design.md`. Also retires the
  stale `.h`→C comment and documents `@liyi:end-requirement` in the template.
- **`@liyi:note` context primitive + `liyi context` CLI** — hard-rename
  `@liyi:module`→`@liyi:note`; add `@liyi:end-note`, `@liyi:see`; marker-only
  retrieval graph; `liyi context <path:line>` MVP. Design authority:
  `docs/note-context-design.md`. LSP/MCP context API is the eventual consumer
  (cross-ref `docs/lsp-design.md`).

---

## §1-TEXT — full body of docs/file-directive-design.md

(Reproduced unfenced so its internal code blocks survive copy/paste. Copy from
the SPDX line to the end of this section into the new file.)

---8<--- docs/file-directive-design.md starts on next line ---8<---

(See the attached full text in the assistant message; identical to the draft
prepared for `docs/file-directive-design.md`.)
