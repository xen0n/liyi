use std::collections::HashMap;

/// Source-file marker scanner with full-width normalization and multilingual aliases.
///
/// A discovered marker in a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceMarker {
    /// A context note opener. `name` is an optional group label; several
    /// blocks may share a name and aggregate. Untracked — never enters the
    /// staleness graph.
    Note {
        name: Option<String>,
        line: usize,
    },
    /// Closes an embedded note block so its extent is exact.
    EndNote {
        name: Option<String>,
        line: usize,
    },
    /// Item-side membership hint pulling a named note into context. The
    /// sentinel name `=none` suppresses otherwise-in-scope notes.
    See {
        name: String,
        line: usize,
    },
    Requirement {
        name: String,
        line: usize,
    },
    EndRequirement {
        name: String,
        line: usize,
    },
    Related {
        name: String,
        line: usize,
    },
    Intent {
        prose: Option<String>,
        is_doc: bool,
        line: usize,
    },
    Trivial {
        line: usize,
    },
    Ignore {
        reason: Option<String>,
        line: usize,
    },
    Nontrivial {
        line: usize,
    },
}

/// Replace full-width punctuation with half-width equivalents.
// @liyi:related marker-normalization
pub fn normalize_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    for ch in line.chars() {
        match ch {
            '\u{FF20}' => out.push('@'),
            '\u{FF1A}' => out.push(':'),
            '\u{FF08}' => out.push('('),
            '\u{FF09}' => out.push(')'),
            _ => out.push(ch),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Marker table — the set of accepted marker strings.
// ---------------------------------------------------------------------------

/// Canonical marker keywords (without the leading `@`).
///
/// NOTE: The `@` is escaped as `\x40` to avoid the linter's own marker
/// scanner matching these string constants — see *Self-hosting and the
/// quine problem* in the design doc.
const CANON_IGNORE: &str = "\x40liyi:ignore";
const CANON_TRIVIAL: &str = "\x40liyi:trivial";
const CANON_NONTRIVIAL: &str = "\x40liyi:nontrivial";
const CANON_NOTE: &str = "\x40liyi:note";
const CANON_END_NOTE: &str = "\x40liyi:end-note";
const CANON_SEE: &str = "\x40liyi:see";
const CANON_REQUIREMENT: &str = "\x40liyi:requirement";
const CANON_END_REQUIREMENT: &str = "\x40liyi:end-requirement";
const CANON_RELATED: &str = "\x40liyi:related";
const CANON_INTENT: &str = "\x40liyi:intent";

/// All recognized canonical marker keywords.
///
/// Order matters: `end-requirement` must precede `requirement` and
/// `end-note` must precede `note`, because `find_marker` scans by substring
/// and the longer keyword must match first.
// @liyi:related marker-normalization
// @liyi:related quine-escape-in-source
const MARKER_KEYWORDS: &[&str] = &[
    CANON_IGNORE,
    CANON_TRIVIAL,
    CANON_NONTRIVIAL,
    CANON_END_NOTE,
    CANON_NOTE,
    CANON_SEE,
    CANON_END_REQUIREMENT,
    CANON_REQUIREMENT,
    CANON_RELATED,
    CANON_INTENT,
];

/// Try to find a known marker at any position in `normalized`.
/// Returns `(keyword, byte-offset of match start, byte-offset past the matched keyword)` on success.
// @liyi:related marker-normalization
fn find_marker(normalized: &str) -> Option<(&'static str, usize, usize)> {
    for &keyword in MARKER_KEYWORDS {
        if let Some(pos) = normalized.find(keyword) {
            return Some((keyword, pos, pos + keyword.len()));
        }
    }
    None
}

/// Maximum allowed length (in bytes) for requirement / related names.
/// Bounds the size of attacker-controlled text that flows into `--prompt`
/// instruction strings.  See `docs/prompt-mode-design.md` §Security.
// @liyi:intent=doc
const MAX_NAME_LEN: usize = 128;

/// Extract a name from the remainder after a keyword.
/// Rules: if first non-WS char is `(`, take everything up to matching `)`;
/// otherwise take the first whitespace-delimited token.
/// Names longer than [`MAX_NAME_LEN`] bytes are rejected.
fn extract_name(rest: &str) -> Option<String> {
    let trimmed = rest.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    let name = if let Some(inner) = trimmed.strip_prefix('(') {
        let end = inner.find(')')?;
        let n = inner[..end].trim();
        if n.is_empty() {
            return None;
        }
        n.to_string()
    } else {
        let token = trimmed.split_whitespace().next()?;
        token.to_string()
    };
    if name.len() > MAX_NAME_LEN {
        return None;
    }
    Some(name)
}

/// Extract an optional group label following a `@liyi:note` / `@liyi:end-note`
/// marker. Returns `None` when no label is present.
///
/// A label must begin with an alphanumeric character. This deliberately
/// rejects trailing comment terminators (`-->`, `*/`, `--`) so that a
/// label-less note written as `<!-- @liyi:note -->` is treated as anonymous
/// rather than mis-parsing the closer as its name.
fn extract_label(rest: &str) -> Option<String> {
    let token = rest.split_whitespace().next()?;
    if !token.chars().next()?.is_alphanumeric() {
        return None;
    }
    if token.len() > MAX_NAME_LEN {
        return None;
    }
    Some(token.to_string())
}

/// Extract the target of an `@liyi:see` marker. Like [`extract_label`] but
/// also accepts the opt-out sentinel `=none` (and any future `=`-prefixed
/// sentinel), while still rejecting comment terminators.
fn extract_see_target(rest: &str) -> Option<String> {
    let token = rest.split_whitespace().next()?;
    let first = token.chars().next()?;
    if !(first.is_alphanumeric() || first == '=') {
        return None;
    }
    if token.len() > MAX_NAME_LEN {
        return None;
    }
    Some(token.to_string())
}

// ---------------------------------------------------------------------------
// NL-quoting quine suppression
// ---------------------------------------------------------------------------

/// Characters that, when immediately preceding the `@` of a marker, cause
/// the marker to be rejected as a documentary mention rather than a real
/// directive.  Covers ASCII quotes, typographic quotes, CJK brackets, and
/// guillemets.
const QUOTE_CHARS: &[char] = &[
    '\'',       // U+0027 apostrophe
    '"',        // U+0022 quotation mark
    '`',        // U+0060 grave accent (backtick) — defense-in-depth with span check
    '\u{2018}', // ' left single quotation mark
    '\u{2019}', // ' right single quotation mark
    '\u{201C}', // " left double quotation mark
    '\u{201D}', // " right double quotation mark
    '\u{300C}', // 「 left corner bracket
    '\u{300D}', // 」 right corner bracket
    '\u{00AB}', // « left guillemet
    '\u{00BB}', // » right guillemet
];

/// Returns true if `byte_pos` falls inside an inline backtick span.
/// Determined by counting backtick characters before the position —
/// an odd count means we are inside a code span.
fn is_in_inline_code(line: &str, byte_pos: usize) -> bool {
    let mut count = 0u32;
    for (i, ch) in line.char_indices() {
        if i >= byte_pos {
            break;
        }
        if ch == '`' {
            count += 1;
        }
    }
    !count.is_multiple_of(2)
}

/// Returns true if the character immediately before `byte_pos` in `line`
/// is a quotation mark (ASCII, typographic, CJK, or guillemet).
fn preceded_by_quote(line: &str, byte_pos: usize) -> bool {
    // Find the last char before byte_pos.
    let prefix = &line[..byte_pos];
    match prefix.chars().next_back() {
        Some(ch) => QUOTE_CHARS.contains(&ch),
        None => false,
    }
}

/// Returns true if `byte_pos` falls inside a quoted span.
///
/// This intentionally tracks only quote forms that are unlikely to appear as
/// apostrophes in prose: ASCII double quotes plus paired typographic/CJK
/// double-quote forms. Single-quote mentions are still handled by the
/// immediate-preceding-quote suppression above.
fn is_in_quoted_span(line: &str, byte_pos: usize) -> bool {
    let mut close_quote: Option<char> = None;
    let mut escaped = false;

    for (i, ch) in line.char_indices() {
        if i >= byte_pos {
            break;
        }

        if let Some(close) = close_quote {
            if close == '"' && escaped {
                escaped = false;
                continue;
            }
            if close == '"' && ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == close {
                close_quote = None;
            }
            continue;
        }

        close_quote = match ch {
            '"' => Some('"'),
            '\u{201C}' => Some('\u{201D}'),
            '\u{2018}' => Some('\u{2019}'),
            '\u{300C}' => Some('\u{300D}'),
            '\u{00AB}' => Some('\u{00BB}'),
            _ => None,
        };
    }

    close_quote.is_some()
}

/// Returns true if a trimmed line opens or closes a fenced code block.
// @liyi:related markdown-fenced-block-skip
fn is_fence_delimiter(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

/// Scan all lines of `content` and return discovered `@liyi:*` markers.
/// Line numbers are 1-indexed.
///
/// Markers are suppressed (not returned) when they appear inside fenced
/// code blocks, inside inline backtick spans, inside quoted spans, or
/// immediately after a quotation-mark character.  See *Self-hosting and the
/// quine problem* in the design doc.
// @liyi:related markdown-fenced-block-skip
// @liyi:related quine-escape-in-source
pub fn scan_markers(content: &str) -> Vec<SourceMarker> {
    let mut markers = Vec::new();
    let mut in_fenced_block = false;

    for (idx, raw_line) in content.lines().enumerate() {
        let line_num = idx + 1;

        // Fenced code block toggle (``` or ~~~).
        if is_fence_delimiter(raw_line) {
            in_fenced_block = !in_fenced_block;
            continue;
        }
        if in_fenced_block {
            continue;
        }

        let normalized = normalize_line(raw_line);

        let (canon, match_start, after) = match find_marker(&normalized) {
            Some(triple) => triple,
            None => continue,
        };

        // NL-quoting suppression: inline backtick span.
        if is_in_inline_code(&normalized, match_start) {
            continue;
        }

        // NL-quoting suppression: marker appears later inside a quoted span
        // such as a JSON string value or quoted prose.
        if is_in_quoted_span(&normalized, match_start) {
            continue;
        }

        // NL-quoting suppression: preceding quote character.
        if preceded_by_quote(&normalized, match_start) {
            continue;
        }

        let rest = &normalized[after..];

        match canon {
            CANON_NOTE => markers.push(SourceMarker::Note {
                name: extract_label(rest),
                line: line_num,
            }),
            CANON_END_NOTE => markers.push(SourceMarker::EndNote {
                name: extract_label(rest),
                line: line_num,
            }),
            CANON_SEE => {
                if let Some(name) = extract_see_target(rest) {
                    markers.push(SourceMarker::See {
                        name,
                        line: line_num,
                    });
                }
            }
            CANON_TRIVIAL => markers.push(SourceMarker::Trivial { line: line_num }),
            CANON_NONTRIVIAL => markers.push(SourceMarker::Nontrivial { line: line_num }),
            CANON_IGNORE => {
                let reason = {
                    let t = rest.trim();
                    if t.is_empty() {
                        None
                    } else {
                        Some(t.to_string())
                    }
                };
                markers.push(SourceMarker::Ignore {
                    reason,
                    line: line_num,
                });
            }
            CANON_REQUIREMENT => {
                if let Some(name) = extract_name(rest) {
                    markers.push(SourceMarker::Requirement {
                        name,
                        line: line_num,
                    });
                }
            }
            CANON_END_REQUIREMENT => {
                if let Some(name) = extract_name(rest) {
                    markers.push(SourceMarker::EndRequirement {
                        name,
                        line: line_num,
                    });
                }
            }
            CANON_RELATED => {
                if let Some(name) = extract_name(rest) {
                    markers.push(SourceMarker::Related {
                        name,
                        line: line_num,
                    });
                }
            }
            CANON_INTENT => {
                let trimmed = rest.trim();
                if trimmed == "=doc" {
                    markers.push(SourceMarker::Intent {
                        prose: None,
                        is_doc: true,
                        line: line_num,
                    });
                } else {
                    let prose = if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    };
                    markers.push(SourceMarker::Intent {
                        prose,
                        is_doc: false,
                        line: line_num,
                    });
                }
            }
            _ => {}
        }
    }

    markers
}

/// Build a map from requirement name to `[start_line, end_line]` spans
/// by pairing `Requirement` and `EndRequirement` markers from a scan result.
///
/// Only requirements that have a matching `EndRequirement` with the same
/// name are included.  Unpaired markers are silently skipped (the linter
/// can diagnose those separately).
pub fn requirement_spans(markers: &[SourceMarker]) -> HashMap<String, [usize; 2]> {
    let mut opens: HashMap<String, usize> = HashMap::new();
    let mut spans: HashMap<String, [usize; 2]> = HashMap::new();

    for m in markers {
        match m {
            SourceMarker::Requirement { name, line } => {
                opens.insert(name.clone(), *line);
            }
            SourceMarker::EndRequirement { name, line } => {
                if let Some(start) = opens.remove(name) {
                    spans.insert(name.clone(), [start, *line]);
                }
            }
            _ => {}
        }
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_fullwidth() {
        assert_eq!(normalize_line("\u{FF20}立意\u{FF1A}忽略"), "\x40立意:忽略");
        assert_eq!(
            normalize_line("\u{FF20}liyi\u{FF1A}intent\u{FF08}x\u{FF09}"),
            "\x40liyi:intent(x)"
        );
    }

    #[test]
    fn scan_note() {
        let m = scan_markers("<!-- \x40liyi:note -->\n");
        assert_eq!(m.len(), 1);
        assert!(matches!(
            &m[0],
            SourceMarker::Note {
                name: None,
                line: 1
            }
        ));
    }

    #[test]
    fn scan_note_named() {
        let m = scan_markers("<!-- \x40liyi:note billing-currency -->\n");
        assert!(
            matches!(&m[0], SourceMarker::Note { name: Some(n), line: 1 } if n == "billing-currency")
        );
    }

    #[test]
    fn scan_end_note_named() {
        let m = scan_markers("<!-- \x40liyi:end-note billing-currency -->\n");
        assert_eq!(m.len(), 1);
        assert!(
            matches!(&m[0], SourceMarker::EndNote { name: Some(n), line: 1 } if n == "billing-currency")
        );
    }

    #[test]
    fn scan_see() {
        let m = scan_markers("// \x40liyi:see money-rounding\n");
        assert!(matches!(&m[0], SourceMarker::See { name, line: 1 } if name == "money-rounding"));
    }

    #[test]
    fn scan_see_opt_out() {
        let m = scan_markers("// \x40liyi:see =none\n");
        assert!(matches!(&m[0], SourceMarker::See { name, line: 1 } if name == "=none"));
    }

    #[test]
    fn note_and_end_note_not_confused() {
        let m = scan_markers("<!-- \x40liyi:note n -->\n<!-- \x40liyi:end-note n -->\n");
        assert_eq!(m.len(), 2);
        assert!(matches!(&m[0], SourceMarker::Note { name: Some(n), line: 1 } if n == "n"));
        assert!(matches!(&m[1], SourceMarker::EndNote { name: Some(n), line: 2 } if n == "n"));
    }

    #[test]
    fn scan_trivial_and_nontrivial() {
        let m = scan_markers("x\n// \x40liyi:trivial\ny\n// \x40liyi:nontrivial\n");
        assert_eq!(m.len(), 2);
        assert!(matches!(&m[0], SourceMarker::Trivial { line: 2 }));
        assert!(matches!(&m[1], SourceMarker::Nontrivial { line: 4 }));
    }

    #[test]
    fn scan_ignore_with_reason() {
        let m = scan_markers("// \x40liyi:ignore generated code\n");
        assert!(
            matches!(&m[0], SourceMarker::Ignore { reason: Some(r), line: 1 } if r == "generated code")
        );
    }

    #[test]
    fn scan_requirement_paren() {
        let m = scan_markers("// \x40liyi:requirement(currency-match) ...\n");
        assert!(
            matches!(&m[0], SourceMarker::Requirement { name, line: 1 } if name == "currency-match")
        );
    }

    #[test]
    fn scan_requirement_space() {
        let m = scan_markers("// \x40liyi:requirement currency-match\n");
        assert!(
            matches!(&m[0], SourceMarker::Requirement { name, line: 1 } if name == "currency-match")
        );
    }

    #[test]
    fn scan_end_requirement_paren() {
        let m = scan_markers("<!-- \x40liyi:end-requirement(exit-codes) -->\n");
        assert_eq!(m.len(), 1);
        assert!(
            matches!(&m[0], SourceMarker::EndRequirement { name, line: 1 } if name == "exit-codes")
        );
    }

    #[test]
    fn scan_end_requirement_space() {
        let m = scan_markers("<!-- \x40liyi:end-requirement exit-codes -->\n");
        assert_eq!(m.len(), 1);
        assert!(
            matches!(&m[0], SourceMarker::EndRequirement { name, line: 1 } if name == "exit-codes")
        );
    }

    #[test]
    fn scan_requirement_and_end_requirement_pair() {
        let input = "\
<!-- \x40liyi:requirement(exit-codes) -->\n\
Exit codes: 0 = clean, 1 = failures.\n\
<!-- \x40liyi:end-requirement(exit-codes) -->\n\
";
        let m = scan_markers(input);
        assert_eq!(m.len(), 2);
        assert!(
            matches!(&m[0], SourceMarker::Requirement { name, line: 1 } if name == "exit-codes")
        );
        assert!(
            matches!(&m[1], SourceMarker::EndRequirement { name, line: 3 } if name == "exit-codes")
        );
    }

    #[test]
    fn scan_related() {
        let m = scan_markers("// \x40liyi:related some_req\n");
        assert!(matches!(&m[0], SourceMarker::Related { name, line: 1 } if name == "some_req"));
    }

    #[test]
    fn scan_intent_doc() {
        let m = scan_markers("// \x40liyi:intent =doc\n");
        assert!(matches!(
            &m[0],
            SourceMarker::Intent {
                prose: None,
                is_doc: true,
                line: 1
            }
        ));
    }

    #[test]
    fn scan_intent_prose() {
        let m = scan_markers("// \x40liyi:intent Must reject negative amounts\n");
        assert!(
            matches!(&m[0], SourceMarker::Intent { prose: Some(p), is_doc: false, line: 1 } if p == "Must reject negative amounts")
        );
    }

    #[test]
    fn scan_fullwidth_normalization() {
        let m = scan_markers("// \u{FF20}liyi\u{FF1A}ignore\n");
        assert_eq!(m.len(), 1);
        assert!(matches!(
            &m[0],
            SourceMarker::Ignore {
                reason: None,
                line: 1
            }
        ));
    }

    // -----------------------------------------------------------------------
    // NL-quoting quine suppression tests
    // -----------------------------------------------------------------------

    #[test]
    fn fenced_block_suppresses_markers() {
        let input = "before\n```\n// \x40liyi:note\n```\nafter\n";
        let m = scan_markers(input);
        assert!(
            m.is_empty(),
            "marker inside fenced block should be suppressed"
        );
    }

    #[test]
    fn fenced_block_tilde_suppresses_markers() {
        let input = "before\n~~~\n// \x40liyi:trivial\n~~~\nafter\n";
        let m = scan_markers(input);
        assert!(
            m.is_empty(),
            "marker inside ~~~ fenced block should be suppressed"
        );
    }

    #[test]
    fn marker_after_fenced_block_still_found() {
        let input = "```\n// \x40liyi:note\n```\n// \x40liyi:trivial\n";
        let m = scan_markers(input);
        assert_eq!(m.len(), 1);
        assert!(matches!(&m[0], SourceMarker::Trivial { line: 4 }));
    }

    #[test]
    fn inline_backtick_suppresses_marker() {
        let input = "use `\x40liyi:note` in your code\n";
        let m = scan_markers(input);
        assert!(
            m.is_empty(),
            "marker inside inline backticks should be suppressed"
        );
    }

    #[test]
    fn inline_backtick_with_surrounding_text() {
        // Pattern from design doc: `<!-- @liyi:note -->`
        let input = "The `<!-- \x40liyi:note -->` comment marks the block\n";
        let m = scan_markers(input);
        assert!(
            m.is_empty(),
            "marker inside backtick span with surrounding text should be suppressed"
        );
    }

    #[test]
    fn preceding_double_quote_suppresses() {
        let input = "the string \"\x40liyi:intent\" is used\n";
        let m = scan_markers(input);
        assert!(
            m.is_empty(),
            "marker preceded by double quote should be suppressed"
        );
    }

    #[test]
    fn preceding_single_quote_suppresses() {
        let input = "the string '\x40liyi:note' is used\n";
        let m = scan_markers(input);
        assert!(
            m.is_empty(),
            "marker preceded by single quote should be suppressed"
        );
    }

    #[test]
    fn preceding_curly_quote_suppresses() {
        let input = "mention \u{201C}\x40liyi:intent\u{201D} in docs\n";
        let m = scan_markers(input);
        assert!(
            m.is_empty(),
            "marker preceded by curly quote should be suppressed"
        );
    }

    #[test]
    fn marker_inside_json_string_is_suppressed() {
        let input = concat!(
            "  \"description\": ",
            "\"Repo-relative path to the source file containing the ",
            "\x40liyi:requirement annotation.\"\n"
        );
        let m = scan_markers(input);
        assert!(
            m.is_empty(),
            "marker inside JSON string value should be suppressed"
        );
    }

    #[test]
    fn marker_after_quoted_span_still_found() {
        let input = "quoted \"\x40liyi:intent\" mention // \x40liyi:note\n";
        let m = scan_markers(input);
        assert_eq!(m.len(), 1);
        assert!(matches!(
            &m[0],
            SourceMarker::Note {
                name: None,
                line: 1
            }
        ));
    }

    #[test]
    fn preceding_cjk_bracket_suppresses() {
        let input = "use \u{300C}\x40liyi:requirement\u{300D}\n";
        let m = scan_markers(input);
        assert!(
            m.is_empty(),
            "marker preceded by CJK bracket should be suppressed"
        );
    }

    #[test]
    fn html_comment_marker_not_suppressed() {
        // Real markers inside HTML comments should be detected
        let input = "<!-- \x40liyi:note -->\n";
        let m = scan_markers(input);
        assert_eq!(m.len(), 1);
        assert!(matches!(
            &m[0],
            SourceMarker::Note {
                name: None,
                line: 1
            }
        ));
    }

    #[test]
    fn source_comment_marker_not_suppressed() {
        // Normal source comment markers should be detected
        let input = "// \x40liyi:requirement(auth-check)\n";
        let m = scan_markers(input);
        assert_eq!(m.len(), 1);
        assert!(
            matches!(&m[0], SourceMarker::Requirement { name, line: 1 } if name == "auth-check")
        );
    }

    #[test]
    fn mixed_real_and_documentary_markers() {
        // A realistic Markdown file: real marker + fenced example + inline mention
        let input = "\
<!-- \x40liyi:requirement(exit-codes) -->\n\
Exit codes: 0 = clean, 1 = failures.\n\
<!-- /requirement -->\n\
\n\
### Example\n\
\n\
```\n\
// \x40liyi:intent Add two amounts\n\
```\n\
\n\
Use `\x40liyi:intent` to annotate functions.\n\
";
        let m = scan_markers(input);
        assert_eq!(
            m.len(),
            1,
            "only the real requirement marker should be found"
        );
        assert!(
            matches!(&m[0], SourceMarker::Requirement { name, line: 1 } if name == "exit-codes")
        );
    }
}
