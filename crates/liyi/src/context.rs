//! Note context resolution (the *retrieval graph*).
//!
//! This module resolves the [`Note`]s applicable to a source location by a
//! **live scan** of the working tree. It reads no sidecars and computes no
//! hashes: notes are untracked context, never part of the staleness graph.
//! See `docs/note-context-design.md` (requirements `note-directory-scope`,
//! `note-see-membership`, `two-graph-separation`, `context-resolution`).
//!
//! MVP scope: notes are discovered from Markdown note files — `LIYI.md` and
//! any `README*.md` — that carry an explicit `@liyi:note` marker. `@liyi:see`
//! references are resolved file-scoped (every `@liyi:see` in the target file
//! contributes); item-precise resolution via tree-sitter is a later refinement.

use std::collections::HashSet;
use std::path::Path;

use ignore::WalkBuilder;

use crate::markers::{SourceMarker, scan_markers};

/// Directory names to always skip while scanning for note files.
const SKIP_DIRS: &[&str] = &[".git", ".hg", ".svn", ".bzr", ".jj"];

/// The opt-out sentinel: `@liyi:see =none` suppresses otherwise-in-scope
/// directory notes for the location.
const SEE_NONE: &str = "=none";

/// How a resolved note reached the location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Membership {
    /// Pulled in by directory-subtree scope (per `note-directory-scope`).
    DirectoryScope,
    /// Pulled in explicitly by an `@liyi:see` reference (per `note-see-membership`).
    See,
}

/// A note discovered in the working tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// Optional group label. Several blocks may share a name and aggregate.
    pub name: Option<String>,
    /// Repo-relative path of the file the note is defined in.
    pub source: String,
    /// 1-indexed `[opener, closer]` line span of the note block.
    pub span: [usize; 2],
    /// The note's governing prose (text between opener and closer, trimmed).
    pub prose: String,
}

/// A note resolved as applicable to a location, plus how it was reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedNote {
    pub note: Note,
    pub via: Membership,
}

/// Returns true if `name` is a file that may carry notes in the MVP: a
/// dedicated `LIYI.md` or any `README*.md`.
// @liyi:related note-directory-scope
fn is_note_file(name: &str) -> bool {
    name == "LIYI.md"
        || name == "README.md"
        || (name.starts_with("README.") && name.ends_with(".md"))
}

/// Repo-relative directory of a repo-relative file path (`""` for root).
fn parent_dir(rel: &str) -> String {
    match rel.rsplit_once('/') {
        Some((dir, _)) => dir.to_string(),
        None => String::new(),
    }
}

/// Ancestor directories of a repo-relative path, nearest first, ending at the
/// repo root (`""`). For `crates/liyi/src/x.rs` →
/// `["crates/liyi/src", "crates/liyi", "crates", ""]`.
fn ancestor_dirs(rel: &str) -> Vec<String> {
    let mut dir = parent_dir(rel);
    let mut out = vec![dir.clone()];
    while !dir.is_empty() {
        dir = parent_dir(&dir);
        out.push(dir.clone());
    }
    out
}

/// Find the index of the most recent open note matching `end_name`.
/// A named end-note matches the nearest open with the same name; an anonymous
/// end-note matches the nearest anonymous open.
fn matching_open(opens: &[(Option<String>, usize)], end_name: &Option<String>) -> Option<usize> {
    opens.iter().rposition(|(name, _)| name == end_name)
}

/// Extract note blocks from a single file's content.
///
/// Pairs `@liyi:note` openers with `@liyi:end-note` closers. An unclosed
/// opener extends to end-of-file (the "dedicated file" case). Prose is the
/// text strictly between opener and closer lines, trimmed.
fn extract_note_blocks(content: &str, source: &str) -> Vec<Note> {
    let lines: Vec<&str> = content.lines().collect();
    let mut opens: Vec<(Option<String>, usize)> = Vec::new();
    let mut notes: Vec<Note> = Vec::new();

    let make = |name: Option<String>, start: usize, end: usize| -> Note {
        // start/end are 1-indexed opener/closer lines; prose is between them.
        let lo = start; // 0-indexed first prose line == start (1-indexed opener)
        let hi = end.saturating_sub(1); // exclusive of closer
        let prose = if lo < hi && hi <= lines.len() {
            lines[lo..hi].join("\n").trim().to_string()
        } else {
            String::new()
        };
        Note {
            name,
            source: source.to_string(),
            span: [start, end],
            prose,
        }
    };

    for m in scan_markers(content) {
        match m {
            SourceMarker::Note { name, line } => opens.push((name, line)),
            SourceMarker::EndNote { name, line } => {
                if let Some(idx) = matching_open(&opens, &name) {
                    let (oname, ostart) = opens.remove(idx);
                    notes.push(make(oname, ostart, line));
                }
            }
            _ => {}
        }
    }
    // Unclosed openers extend to EOF (the "dedicated file" case): prose runs
    // from the line after the opener through the last line, inclusive.
    for (oname, ostart) in opens {
        let prose = if ostart <= lines.len() {
            lines[ostart..].join("\n").trim().to_string()
        } else {
            String::new()
        };
        notes.push(Note {
            name: oname,
            source: source.to_string(),
            span: [ostart, lines.len().max(ostart)],
            prose,
        });
    }
    notes
}

/// Walk the tree rooted at `root` and collect every note from note files.
// @liyi:related note-is-untracked
pub fn collect_notes(root: &Path) -> Vec<Note> {
    let mut notes = Vec::new();
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .add_custom_ignore_filename(".liyiignore")
        .filter_entry(|entry| {
            if entry.file_type().is_some_and(|ft| ft.is_dir())
                && let Some(name) = entry.file_name().to_str()
            {
                return !SKIP_DIRS.contains(&name);
            }
            true
        })
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !is_note_file(name) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let source = rel.to_string_lossy().replace('\\', "/");
        notes.extend(extract_note_blocks(&content, &source));
    }
    notes
}

/// Collect the `@liyi:see` targets declared in the target file. The MVP is
/// file-scoped: every `@liyi:see` in the file contributes. Returns the set of
/// referenced note names and whether an opt-out sentinel (`=none`) is present.
fn collect_see(content: &str) -> (Vec<String>, bool) {
    let mut names = Vec::new();
    let mut opt_out = false;
    for m in scan_markers(content) {
        if let SourceMarker::See { name, .. } = m {
            if name == SEE_NONE {
                opt_out = true;
            } else {
                names.push(name);
            }
        }
    }
    (names, opt_out)
}

/// Resolve the notes applicable to `target_rel` (a repo-relative path).
///
/// Returns notes deterministically ordered by `(source, span start)`:
/// directory-scope notes (nearest-wins shadowing by name) plus any notes
/// named by the file's `@liyi:see` references, minus directory-scope notes
/// suppressed by an `@liyi:see =none` opt-out. Reads the working tree live.
// @liyi:related context-resolution
// @liyi:related note-directory-scope
pub fn resolve(root: &Path, target_rel: &str) -> Vec<ResolvedNote> {
    let all_notes = collect_notes(root);

    // --- Directory scope, nearest-wins shadowing -------------------------
    let mut scope: Vec<Note> = Vec::new();
    let mut shadowed: HashSet<String> = HashSet::new();
    for ancestor in ancestor_dirs(target_rel) {
        let mut level: Vec<Note> = all_notes
            .iter()
            .filter(|n| parent_dir(&n.source) == ancestor)
            .filter(|n| {
                let key = n.name.clone().unwrap_or_else(|| "\0anon".to_string());
                !shadowed.contains(&key)
            })
            .cloned()
            .collect();
        level.sort_by(|a, b| a.span[0].cmp(&b.span[0]));
        for n in &level {
            shadowed.insert(n.name.clone().unwrap_or_else(|| "\0anon".to_string()));
        }
        scope.extend(level);
    }

    // --- @liyi:see membership -------------------------------------------
    let (see_names, opt_out): (Vec<String>, bool) =
        match std::fs::read_to_string(root.join(target_rel)) {
            Ok(content) => collect_see(&content),
            Err(_) => (Vec::new(), false),
        };
    let see_set: HashSet<&String> = see_names.iter().collect();

    let mut out: Vec<ResolvedNote> = Vec::new();
    let mut included: HashSet<(String, usize)> = HashSet::new();

    // See-referenced notes are always included (explicit membership), even
    // when the opt-out suppresses directory scope.
    if !see_set.is_empty() {
        for n in &all_notes {
            if let Some(name) = &n.name
                && see_set.contains(name)
            {
                let key = (n.source.clone(), n.span[0]);
                if included.insert(key) {
                    out.push(ResolvedNote {
                        note: n.clone(),
                        via: Membership::See,
                    });
                }
            }
        }
    }

    // Directory-scope notes, unless suppressed by =none.
    if !opt_out {
        for n in scope {
            let key = (n.source.clone(), n.span[0]);
            if included.insert(key) {
                out.push(ResolvedNote {
                    note: n,
                    via: Membership::DirectoryScope,
                });
            }
        }
    }

    out.sort_by(|a, b| {
        a.note
            .source
            .cmp(&b.note.source)
            .then(a.note.span[0].cmp(&b.note.span[0]))
    });
    out
}

/// Render resolved notes as the human/agent-readable `liyi context` report.
pub fn render(notes: &[ResolvedNote]) -> String {
    if notes.is_empty() {
        return "No notes apply to this location.\n".to_string();
    }
    let mut out = String::new();
    for (i, rn) in notes.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let heading = rn.note.name.as_deref().unwrap_or("(unnamed note)");
        let via = match rn.via {
            Membership::See => "  (via @liyi:see)",
            Membership::DirectoryScope => "",
        };
        out.push_str(&format!("# {heading}{via}  ({})\n", rn.note.source));
        out.push_str(&rn.note.prose);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(root: &Path, rel: &str, content: &str) {
        let p = root.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, content).unwrap();
    }

    #[test]
    fn ancestor_dirs_walks_to_root() {
        assert_eq!(
            ancestor_dirs("crates/liyi/src/x.rs"),
            vec!["crates/liyi/src", "crates/liyi", "crates", ""]
        );
        assert_eq!(ancestor_dirs("README.md"), vec![""]);
    }

    #[test]
    fn extract_named_block() {
        let content = "intro\n<!-- \x40liyi:note billing -->\nAll amounts carry currency.\n<!-- \x40liyi:end-note billing -->\nafter\n";
        let notes = extract_note_blocks(content, "README.md");
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].name.as_deref(), Some("billing"));
        assert_eq!(notes[0].prose, "All amounts carry currency.");
        assert_eq!(notes[0].span, [2, 4]);
    }

    #[test]
    fn unclosed_block_extends_to_eof() {
        let content = "<!-- \x40liyi:note -->\nbody line one\nbody line two\n";
        let notes = extract_note_blocks(content, "LIYI.md");
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].name, None);
        assert!(notes[0].prose.contains("body line one"));
        assert!(notes[0].prose.contains("body line two"));
    }

    #[test]
    fn directory_scope_includes_ancestor_note() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            root,
            "README.md",
            "<!-- \x40liyi:note project -->\nProject-wide invariant.\n<!-- \x40liyi:end-note project -->\n",
        );
        write(root, "src/money.rs", "fn settle() {}\n");

        let resolved = resolve(root, "src/money.rs");
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].note.name.as_deref(), Some("project"));
        assert_eq!(resolved[0].via, Membership::DirectoryScope);
    }

    #[test]
    fn nearer_note_shadows_same_name() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            root,
            "README.md",
            "<!-- \x40liyi:note rule -->\nroot version\n<!-- \x40liyi:end-note rule -->\n",
        );
        write(
            root,
            "src/LIYI.md",
            "<!-- \x40liyi:note rule -->\nnested version\n<!-- \x40liyi:end-note rule -->\n",
        );
        write(root, "src/money.rs", "fn settle() {}\n");

        let resolved = resolve(root, "src/money.rs");
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].note.prose, "nested version");
        assert_eq!(resolved[0].note.source, "src/LIYI.md");
    }

    #[test]
    fn see_pulls_remote_note() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            root,
            "docs/README.md",
            "<!-- \x40liyi:note money-rounding -->\nBankers rounding default.\n<!-- \x40liyi:end-note money-rounding -->\n",
        );
        write(
            root,
            "src/report.rs",
            "// \x40liyi:see money-rounding\nfn report() {}\n",
        );

        let resolved = resolve(root, "src/report.rs");
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].note.name.as_deref(), Some("money-rounding"));
        assert_eq!(resolved[0].via, Membership::See);
    }

    #[test]
    fn see_none_suppresses_directory_scope() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            root,
            "README.md",
            "<!-- \x40liyi:note project -->\nbroad context\n<!-- \x40liyi:end-note project -->\n",
        );
        write(root, "src/special.rs", "// \x40liyi:see =none\nfn f() {}\n");

        let resolved = resolve(root, "src/special.rs");
        assert!(
            resolved.is_empty(),
            "=none should suppress directory-scope notes"
        );
    }

    #[test]
    fn see_survives_opt_out() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            root,
            "README.md",
            "<!-- \x40liyi:note project -->\nbroad\n<!-- \x40liyi:end-note project -->\n",
        );
        write(
            root,
            "docs/LIYI.md",
            "<!-- \x40liyi:note security -->\nvalidate all input\n<!-- \x40liyi:end-note security -->\n",
        );
        write(
            root,
            "src/special.rs",
            "// \x40liyi:see =none\n// \x40liyi:see security\nfn f() {}\n",
        );

        let resolved = resolve(root, "src/special.rs");
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].note.name.as_deref(), Some("security"));
        assert_eq!(resolved[0].via, Membership::See);
    }

    #[test]
    fn unmarked_readme_is_invisible() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            root,
            "README.md",
            "# Big manual\n\nLots of prose, no marker.\n",
        );
        write(root, "src/money.rs", "fn f() {}\n");

        let resolved = resolve(root, "src/money.rs");
        assert!(
            resolved.is_empty(),
            "a README without a @liyi:note marker must not enter context"
        );
    }
}
