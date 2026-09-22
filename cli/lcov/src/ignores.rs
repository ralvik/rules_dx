//! Source-level exclusion markers for the coverage gate.
//!
//! Split from `super` (`lib.rs`): owns [`Ignores`], [`is_ignored`], and
//! [`find_ignores`] plus the comment-style scanner (`CommentStyle`,
//! `comment_style`, `line_comment_with`, `line_comment`, `hash_comment`,
//! `html_comments`, `comment_text`, `take_word`, `reason_value`,
//! `nearby_reason`). Re-exported through `super` so the public path
//! stays `dx_lcov::{Ignores, is_ignored, find_ignores}`. Distinct from the
//! `parse` module (combined-LCOV parsing), the `verdict` module (gate
//! evaluation), and the `inventory`/`run` modules (repo inventory and CLI).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use regex::Regex;

use super::LcovError;

/// Validated source-level exclusions for one file.
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Ignores {
    /// Singly excluded lines with their reason text.
    pub singles: BTreeMap<u32, String>,
    /// Excluded ranges (inclusive start/end) with the opening reason text.
    pub ranges: Vec<(u32, u32, String)>,
}

/// Whether `line` (1-based) is excluded by `ignores`.
pub fn is_ignored(ignores: &Ignores, line: u32) -> bool {
    if ignores.singles.contains_key(&line) {
        return true;
    }
    for range in &ignores.ranges {
        if range.0 <= line && line <= range.1 {
            return true;
        }
    }
    false
}

/// Short-reason cap for exclusion markers (`reason:` plus `issue:` plus `policy:`).
///
/// Full rationale lives once in `docs/testing/strategy-details.md#coverage`;
/// marker reasons stay short and specific so the coverage denominator is
/// argued away nowhere. Paragraph-long reasons fail the gate via
/// [`LcovError::ReasonTooLong`]. Bare `policy:` pointers without a specific
/// `reason:` fail via [`LcovError::BarePolicyWithoutReason`]; reasons without
/// `issue:` tracking fail via [`LcovError::MissingIssue`].
pub const MAX_REASON_LEN: usize = 120;

/// Non-empty value text after `key` (`reason:`/`policy:`/`issue:`) on `line`, if present.
fn key_value(line: &str, key: &str) -> Option<String> {
    let offset = line.find(key)?;
    let value = line[offset + key.len()..].trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Non-empty reason text after `reason:` on `line`, if present.
///
/// Only `reason:` counts: a bare `policy:` pointer without a specific
/// `reason:` is rejected (see [`LcovError::BarePolicyWithoutReason`]).
fn reason_value(line: &str) -> Option<String> {
    key_value(line, "reason:")
}

/// Non-empty issue tracking text after `issue:` on `line`, if present.
///
/// The value must reference the tracking issue number (contains a digit)
/// so blanket excludes stay budgeted with expiry review (see
/// `tools/coverage/excludes-budget.txt`).
fn issue_value(line: &str) -> Option<String> {
    let value = key_value(line, "issue:")?;
    if value.chars().any(|c| c.is_ascii_digit()) {
        Some(value)
    } else {
        None
    }
}

/// Whether `line` carries a `policy:` pointer (optional companion to `reason:`).
fn has_policy(line: &str) -> bool {
    line.contains("policy:")
}

/// Reason for `directive` at 1-based `lineno`: specific `reason:` plus
/// `issue:` tracking on the same line or the line directly above it.
///
/// Both keys must appear across the two nearby lines (split form allowed:
/// `reason:` on one line and `issue:` on the other). A bare `policy:`
/// pointer without `reason:` fails closed; a specific `reason:` without
/// `issue:` fails closed for budget/expiry review.
fn nearby_reason(
    path: &str,
    directive: &str,
    lineno: usize,
    lines: &[&str],
) -> Result<String, LcovError> {
    let current = lines[lineno - 1];
    let previous = if lineno >= 2 {
        Some(lines[lineno - 2])
    } else {
        None
    };
    let reason = if let Some(reason) = reason_value(current) {
        reason
    } else if let Some(prev) = previous {
        if let Some(reason) = reason_value(prev) {
            reason
        } else if has_policy(current) || has_policy(prev) {
            return Err(LcovError::BarePolicyWithoutReason {
                path: path.to_string(),
                lineno,
                directive: directive.to_string(),
            });
        } else {
            return Err(LcovError::MissingReason {
                path: path.to_string(),
                lineno,
                directive: directive.to_string(),
            });
        }
    } else if has_policy(current) {
        return Err(LcovError::BarePolicyWithoutReason {
            path: path.to_string(),
            lineno,
            directive: directive.to_string(),
        });
    } else {
        return Err(LcovError::MissingReason {
            path: path.to_string(),
            lineno,
            directive: directive.to_string(),
        });
    };
    if reason.len() > MAX_REASON_LEN {
        return Err(LcovError::ReasonTooLong {
            path: path.to_string(),
            lineno,
            directive: directive.to_string(),
            len: reason.len(),
            max: MAX_REASON_LEN,
        });
    }
    let has_issue = issue_value(current).is_some()
        || previous.is_some_and(|prev| issue_value(prev).is_some());
    if !has_issue {
        return Err(LcovError::MissingIssue {
            path: path.to_string(),
            lineno,
            directive: directive.to_string(),
        });
    }
    Ok(reason)
}

/// Comment style for marker extraction, selected by source extension.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CommentStyle {
    /// `//` line comments (Rust, Go, C-family, Java/Kotlin/Scala, C#/F#,
    /// JavaScript/TypeScript including `.mjs`/`.cjs`/`.mts`/`.cts`).
    SlashSlash,
    /// `#` line comments (Python including `.pyi` stubs, Starlark, TOML, shell, YAML).
    Hash,
    /// `<!-- ... -->` segments (Markdown, HTML).
    Html,
}

/// Marker comment style for `path`, by file extension. Unknown extensions
/// keep the historical `//` behavior.
fn comment_style(path: &str) -> CommentStyle {
    if path.ends_with(".py")
        || path.ends_with(".pyi")
        || path.ends_with(".bzl")
        || path.ends_with(".toml")
        || path.ends_with(".sh")
        || path.ends_with(".yaml")
        || path.ends_with(".yml")
    {
        CommentStyle::Hash
    } else if path.ends_with(".md")
        || path.ends_with(".html")
        || path.ends_with(".htm")
        || path.ends_with(".mdx")
    {
        CommentStyle::Html
    } else {
        CommentStyle::SlashSlash
    }
}

/// Compiled comment scanners: the leading alternatives skip
/// `"`/`'` literals (with backslash escapes) so the trailing `marker`
/// group only matches a comment opener outside literals. `OnceLock`
/// caching keeps the per-line scan allocation-free after the first use;
/// `None` (impossible for these static patterns) falls back to the
/// byte-loop below so the crate stays infallible without `expect`/`unwrap`
/// (crate denies both outside tests).
fn slash_scan() -> Option<&'static Regex> {
    static SCAN: OnceLock<Regex> = OnceLock::new();
    if let Some(compiled) = SCAN.get() {
        return Some(compiled);
    }
    match Regex::new(r#""(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|(?P<marker>//)"#) {
        Ok(compiled) => {
            let _ = SCAN.set(compiled);
            SCAN.get()
        }
        Err(_) => None, // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
    }
}

fn hash_scan() -> Option<&'static Regex> {
    static SCAN: OnceLock<Regex> = OnceLock::new();
    if let Some(compiled) = SCAN.get() {
        return Some(compiled);
    }
    match Regex::new(r#""(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|(?P<marker>#)"#) {
        Ok(compiled) => {
            let _ = SCAN.set(compiled);
            SCAN.get()
        }
        Err(_) => None, // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
    }
}

/// Word-boundary check for directive suffixes (`_LINE`/`_START`/`_STOP`):
/// `^_(LINE|START|STOP)\b` replaces the hand-rolled
/// `strip_prefix` + `is_alphanumeric/_` test with declarative `\b`.
fn directive_suffix() -> Option<&'static Regex> {
    static SUFFIX: OnceLock<Regex> = OnceLock::new();
    if let Some(compiled) = SUFFIX.get() {
        return Some(compiled);
    }
    match Regex::new(r"^_(LINE|START|STOP)\b") {
        Ok(compiled) => {
            let _ = SUFFIX.set(compiled);
            SUFFIX.get()
        }
        Err(_) => None, // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
    }
}

/// Regex-first comment scan: first `marker` capture outside literals wins.
/// Falls back to the byte loop when the static pattern fails to compile
/// (unreachable; keeps the non-test build `expect`/`unwrap`-free).
fn scan_with(line: &str, compiled: Option<&Regex>) -> Option<usize> {
    let re = compiled?;
    for captures in re.captures_iter(line) {
        if let Some(marker) = captures.name("marker") {
            return Some(marker.start() + marker.as_str().len());
        }
    }
    None
}

/// Comment text after the `opener` comment start, honoring `"`/`'`
/// literals and backslash escapes. Returns `None` when the line has no
/// line comment.
fn line_comment_with<'a>(line: &'a str, opener: &[u8]) -> Option<&'a str> {
    if opener == b"//" {
        if let Some(end) = scan_with(line, slash_scan()) {
            return Some(&line[end..]);
        }
        if slash_scan().is_some() {
            return None;
        } // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
    } else if opener == b"#" {
        if let Some(end) = scan_with(line, hash_scan()) {
            return Some(&line[end..]);
        }
        if hash_scan().is_some() {
            return None;
        } // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
    } // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
    line_comment_with_fallback(line, opener) // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
}

/// Byte-loop fallback for [`line_comment_with`] (unreachable unless the
/// static `regex` patterns fail to compile).
// LCOV_EXCL_START - policy: docs/testing/README.md#coverage
fn line_comment_with_fallback<'a>(line: &'a str, opener: &[u8]) -> Option<&'a str> {
    let bytes = line.as_bytes();
    let mut index = 0;
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
        } else if in_char {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'\'' {
                in_char = false;
            }
        } else if byte == b'"' {
            in_string = true;
        } else if byte == b'\'' {
            in_char = true;
        } else if bytes[index..].starts_with(opener) {
            return Some(&line[index + opener.len()..]);
        }
        index += 1;
    }
    None
}
// LCOV_EXCL_STOP - policy: docs/testing/README.md#coverage

/// Comment text after the `//` comment start, honoring `"`/`'` literals and
/// backslash escapes. Returns `None` when the line has no line comment.
fn line_comment(line: &str) -> Option<&str> {
    line_comment_with(line, b"//")
}

/// Comment text after the `#` comment start, with the same literal
/// handling as [`line_comment`]. Returns `None` when the line has no
/// `#` comment.
fn hash_comment(line: &str) -> Option<&str> {
    line_comment_with(line, b"#")
}

/// Concatenated `<!-- ... -->` comment segments on one line. An opening
/// marker without a closer runs to end of line; text outside segments is
/// code and never scanned for directives.
fn html_comments(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(open) = rest.find("<!--") {
        let after = &rest[open + "<!--".len()..];
        match after.find("-->") {
            Some(close) => {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(&after[..close]);
                rest = &after[close + "-->".len()..];
            }
            None => {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(after);
                break;
            }
        }
    }
    out
}

/// Scannable comment text for one source `line` of `path`, dispatching on
/// the extension-selected [`CommentStyle`].
fn comment_text(path: &str, line: &str) -> String {
    match comment_style(path) {
        CommentStyle::SlashSlash => line_comment(line).unwrap_or_default().to_string(),
        CommentStyle::Hash => hash_comment(line).unwrap_or_default().to_string(),
        CommentStyle::Html => html_comments(line),
    }
}

/// Whether `rest` (text right after the common marker prefix) is `word`
/// followed by a non-word character or end of text. Declarative `\b`
/// via [`directive_suffix`]; the byte fallback preserves
/// the historical `is_alphanumeric/_` semantics when the static pattern
/// fails to compile.
fn take_word(rest: &str, word: &str) -> bool {
    if let Some(re) = directive_suffix() {
        return match re.find(rest) {
            Some(matched) => matched.as_str() == word,
            None => false,
        };
    } // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
      // LCOV_EXCL_START - policy: docs/testing/README.md#coverage
    if let Some(tail) = rest.strip_prefix(word) {
        !tail.starts_with(|c: char| c == '_' || c.is_alphanumeric())
    } else {
        false
    }
    // LCOV_EXCL_STOP - policy: docs/testing/README.md#coverage
}

/// Validate the exclusion markers in the `source` of `path`.
///
/// Every LINE/START/STOP directive needs a nearby specific non-empty short
/// (`MAX_REASON_LEN`) `reason:` plus `issue:` tracking on the same or
/// previous line; bare `policy:` pointers without `reason:` fail via
/// [`LcovError::BarePolicyWithoutReason`] and reasons without `issue:` fail
/// via [`LcovError::MissingIssue`]. Ranges must open and close exactly once;
/// any other spelling of the marker prefix is an unrecognized directive and
/// fails. Markers are honored only inside the extension-selected comment
/// style (see [`comment_style`]) outside literals. Block comments
/// (`/* ... */`) and raw strings (`r#"..."#`) stay wont-fix out of scope:
/// the scan is line-comment textual only and no eligible source uses those
/// shapes, so markers there are inert.
pub fn find_ignores(path: &str, source: &str) -> Result<Ignores, LcovError> {
    let lines: Vec<&str> = source.lines().collect();
    let mut ignores = Ignores::default();
    let mut open: Option<(usize, String)> = None;
    for (index, line) in lines.iter().enumerate() {
        let lineno = index + 1;
        let comment = comment_text(path, line);
        for (pos, _) in comment.match_indices("LCOV_EXCL") {
            let rest = &comment[pos + "LCOV_EXCL".len()..];
            if take_word(rest, "_LINE") {
                let reason = nearby_reason(path, "LINE directive", lineno, &lines)?;
                ignores.singles.insert(lineno as u32, reason);
            } else if take_word(rest, "_START") {
                let reason = nearby_reason(path, "START directive", lineno, &lines)?;
                if open.is_some() {
                    return Err(LcovError::NestedStart {
                        path: path.to_string(),
                        lineno,
                    });
                }
                open = Some((lineno, reason));
            } else if take_word(rest, "_STOP") {
                let reason = nearby_reason(path, "STOP directive", lineno, &lines)?;
                match open.take() {
                    Some((start, start_reason)) => {
                        ignores
                            .ranges
                            .push((start as u32, lineno as u32, start_reason));
                        let _ = reason;
                    }
                    None => {
                        return Err(LcovError::StopWithoutStart {
                            path: path.to_string(),
                            lineno,
                        });
                    }
                }
            } else {
                return Err(LcovError::UnrecognizedDirective {
                    path: path.to_string(),
                    lineno,
                });
            }
        }
    }
    if let Some((start, _)) = open {
        return Err(LcovError::UnclosedStart {
            path: path.to_string(),
            start,
        });
    }
    Ok(ignores)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a marker suffix without spelling the contiguous literal in this
    /// file: the gate scans its own sources, so test data must not contribute
    /// directives. Every marker below lives inside string literals, which the
    /// comment scanner ignores.
    fn marker(kind: &str) -> String {
        ["LCOV", "_EXCL", kind].concat()
    }

    fn file_lines(lines: &[String]) -> String {
        let mut out = lines.join("\n");
        out.push('\n');
        out
    }

    #[test]
    fn single_line_ignore_needs_same_line_reason() {
        let source = file_lines(&[
            "pub fn f() -> u32 {".to_string(),
            format!("    // {} - reason: fixture, issue: 1055.", marker("_LINE")),
            "    1".to_string(),
            "}".to_string(),
        ]);
        let ignores = find_ignores("t.rs", &source).unwrap();
        assert_eq!(ignores.singles.len(), 1);
        assert!(ignores.singles.contains_key(&2));
        assert!(is_ignored(&ignores, 2));
        assert!(!is_ignored(&ignores, 1));
        assert!(!is_ignored(&ignores, 3));
    }

    #[test]
    fn single_line_ignore_accepts_previous_line_reason() {
        let source = file_lines(&[
            "    // reason: fixture explains the next line, issue: 1055.".to_string(),
            format!("    // {}", marker("_LINE")),
            "    1".to_string(),
        ]);
        let ignores = find_ignores("t.rs", &source).unwrap();
        assert!(ignores.singles.contains_key(&2));
    }

    #[test]
    fn single_line_ignore_accepts_marker_at_end_of_line() {
        let source = file_lines(&[
            "    // reason: fixture, issue: 1055.".to_string(),
            format!("    // {}", marker("_LINE")),
            "    1".to_string(),
        ]);
        assert!(find_ignores("t.rs", &source)
            .unwrap()
            .singles
            .contains_key(&2));
    }

    #[test]
    fn missing_reason_fails_on_first_line() {
        let source = file_lines(&[format!("// {}", marker("_LINE")), "fn f() {}".to_string()]);
        let err = find_ignores("t.rs", &source).unwrap_err();
        assert!(
            err.to_string().contains("t.rs:1") && err.to_string().contains("reason"),
            "{err}"
        );
    }

    #[test]
    fn missing_reason_fails_with_unrelated_previous_line() {
        let source = file_lines(&[
            "fn f() {".to_string(),
            "    // nothing here.".to_string(),
            format!("    // {} - oops, no reason key.", marker("_LINE")),
            "}".to_string(),
        ]);
        assert!(find_ignores("t.rs", &source).is_err());
    }

    #[test]
    fn empty_reason_fails() {
        let source = file_lines(&[
            "    // reason:   ".to_string(),
            format!("    // {}", marker("_LINE")),
            "    1".to_string(),
        ]);
        assert!(find_ignores("t.rs", &source).is_err());
    }

    #[test]
    fn distant_reason_fails() {
        let source = file_lines(&[
            "    // reason: too far above.".to_string(),
            "    // filler.".to_string(),
            format!("    // {}", marker("_LINE")),
        ]);
        assert!(find_ignores("t.rs", &source).is_err());
    }

    #[test]
    fn range_excludes_interior_and_boundaries() {
        let source = file_lines(&[
            "fn f() {".to_string(),
            format!("    // {} - reason: range opens, issue: 1055.", marker("_START")),
            "    1".to_string(),
            format!(
                "    // {} - reason: range closes, issue: 1055.",
                marker("_STOP")
            ),
            "}".to_string(),
        ]);
        let ignores = find_ignores("t.rs", &source).unwrap();
        assert_eq!(ignores.ranges.len(), 1);
        assert!(is_ignored(&ignores, 2));
        assert!(is_ignored(&ignores, 3));
        assert!(is_ignored(&ignores, 4));
        assert!(!is_ignored(&ignores, 1));
        assert!(!is_ignored(&ignores, 5));
    }

    #[test]
    fn stop_without_reason_fails() {
        let source = file_lines(&[
            format!("// {} - reason: opens, issue: 1055.", marker("_START")),
            "code();".to_string(),
            format!("// {}", marker("_STOP")),
        ]);
        assert!(find_ignores("t.rs", &source).is_err());
    }

    #[test]
    fn stop_without_start_fails() {
        let source = file_lines(&[format!(
            "// {} - reason: stray stop, issue: 1055.",
            marker("_STOP")
        )]);
        let err = find_ignores("t.rs", &source).unwrap_err();
        assert!(err.to_string().contains("without START"), "{err}");
    }

    #[test]
    fn nested_start_fails() {
        let source = file_lines(&[
            format!("// {} - reason: outer, issue: 1055.", marker("_START")),
            format!("// {} - reason: inner, issue: 1055.", marker("_START")),
            format!("// {} - reason: close, issue: 1055.", marker("_STOP")),
            format!("// {} - reason: close, issue: 1055.", marker("_STOP")),
        ]);
        let err = find_ignores("t.rs", &source).unwrap_err();
        assert!(err.to_string().contains("nested"), "{err}");
    }

    #[test]
    fn unclosed_start_fails() {
        let source = file_lines(&[
            "fn f() {".to_string(),
            format!(
                "    // {} - reason: never closed, issue: 1055.",
                marker("_START")
            ),
            "}".to_string(),
        ]);
        let err = find_ignores("t.rs", &source).unwrap_err();
        assert!(
            err.to_string().contains("unclosed") && err.to_string().contains("t.rs:2"),
            "{err}"
        );
    }

    #[test]
    fn unrecognized_suffix_fails() {
        let source = file_lines(&[format!("// {} - reason: typo.", marker("_RANGE"))]);
        let err = find_ignores("t.rs", &source).unwrap_err();
        assert!(err.to_string().contains("unrecognized"), "{err}");
    }

    #[test]
    fn bare_prefix_fails() {
        let source = file_lines(&[format!("// {} - reason: bare.", marker(""))]);
        assert!(find_ignores("t.rs", &source).is_err());
    }

    #[test]
    fn word_boundary_suffix_fails() {
        let source = file_lines(&[format!("// {}X - reason: glued.", marker("_LINE"))]);
        assert!(find_ignores("t.rs", &source).is_err());
    }

    #[test]
    fn markers_inside_string_literals_are_ignored() {
        let tricky = format!("let s = \"code with // {} inside\";", marker("_LINE"));
        let source = file_lines(&[tricky, "real();".to_string()]);
        let ignores = find_ignores("t.rs", &source).unwrap();
        assert!(ignores.singles.is_empty());
        assert!(ignores.ranges.is_empty());
    }

    #[test]
    fn lexer_survives_escapes_and_char_literals() {
        let source = file_lines(&[
            "let s = \"a\\\"b\";".to_string(),
            "let q = '\\'';".to_string(),
            "let c = '/';".to_string(),
            "code();".to_string(),
        ]);
        assert!(find_ignores("t.rs", &source).unwrap().singles.is_empty());
    }

    #[test]
    fn marker_after_string_state_is_recognized() {
        let source = file_lines(&[
            "let s = \"a\\\"b\";".to_string(),
            format!("// {} - reason: after strings, issue: 1055.", marker("_LINE")),
            "code();".to_string(),
        ]);
        let ignores = find_ignores("t.rs", &source).unwrap();
        assert!(ignores.singles.contains_key(&2));
    }

    #[test]
    fn hash_comment_markers_are_honored_for_python() {
        let source = file_lines(&[
            "def f():".to_string(),
            format!(
                "    pass  # {} - reason: fixture defensive line, issue: 1055.",
                marker("_LINE")
            ),
            "    return 1".to_string(),
        ]);
        let ignores = find_ignores("t.py", &source).unwrap();
        assert!(ignores.singles.contains_key(&2));
        assert!(!is_ignored(&ignores, 3));
    }

    #[test]
    fn hash_markers_inside_python_strings_are_ignored() {
        let tricky = format!("s = \"code with # {} inside\";", marker("_LINE"));
        let source = file_lines(&[tricky, "real();".to_string()]);
        let ignores = find_ignores("t.py", &source).unwrap();
        assert!(ignores.singles.is_empty());
        assert!(ignores.ranges.is_empty());
    }

    #[test]
    fn slash_markers_are_not_honored_for_python() {
        let source = file_lines(&[format!("// {} - reason: wrong style.", marker("_LINE"))]);
        let ignores = find_ignores("t.py", &source).unwrap();
        assert!(ignores.singles.is_empty());
        assert!(ignores.ranges.is_empty());
    }

    #[test]
    fn hash_marker_without_reason_fails_for_python() {
        let source = file_lines(&[format!("# {}", marker("_LINE")), "x = 1".to_string()]);
        let err = find_ignores("t.py", &source).unwrap_err();
        assert!(
            err.to_string().contains("t.py:1") && err.to_string().contains("reason"),
            "{err}"
        );
    }

    #[test]
    fn hash_malformed_directive_fails_for_python() {
        let source = file_lines(&[format!("# {} - reason: typo.", marker("_RANGE"))]);
        let err = find_ignores("t.py", &source).unwrap_err();
        assert!(err.to_string().contains("unrecognized"), "{err}");
    }

    #[test]
    fn hash_range_excludes_boundaries_for_starlark() {
        let source = file_lines(&[
            "def f():".to_string(),
            format!("    # {} - reason: range opens, issue: 1055.", marker("_START")),
            "    pass".to_string(),
            format!(
                "    # {} - reason: range closes, issue: 1055.",
                marker("_STOP")
            ),
        ]);
        let ignores = find_ignores("t.bzl", &source).unwrap();
        assert_eq!(ignores.ranges.len(), 1);
        assert!(is_ignored(&ignores, 2));
        assert!(is_ignored(&ignores, 3));
        assert!(is_ignored(&ignores, 4));
        assert!(!is_ignored(&ignores, 1));
    }

    #[test]
    fn html_comment_markers_are_honored_for_markdown() {
        let source = file_lines(&[
            "# Title".to_string(),
            format!(
                "<!-- {} - reason: fixture prose, issue: 1055. -->",
                marker("_LINE")
            ),
            "Body.".to_string(),
        ]);
        let ignores = find_ignores("t.md", &source).unwrap();
        assert!(ignores.singles.contains_key(&2));
    }

    #[test]
    fn html_second_comment_on_line_keeps_separator() {
        let line = format!(
            "prose <!-- dropped --> more <!-- {} - reason: second segment, issue: 1055. -->",
            marker("_LINE")
        );
        let source = file_lines(&[line]);
        let ignores = find_ignores("t.md", &source).unwrap();
        assert!(ignores.singles.contains_key(&1));
    }

    #[test]
    fn html_unterminated_comment_after_content_keeps_prefix() {
        let line = format!(
            "prose <!-- dropped --> tail <!-- {} - reason: unterminated, issue: 1055.",
            marker("_LINE")
        );
        let source = file_lines(&[line]);
        let ignores = find_ignores("t.md", &source).unwrap();
        assert!(ignores.singles.contains_key(&1));
    }

    #[test]
    fn html_code_outside_segments_is_not_scanned_for_markdown() {
        let source = file_lines(&["real `code` here.".to_string(), "More.".to_string()]);
        let ignores = find_ignores("t.md", &source).unwrap();
        assert!(ignores.singles.is_empty());
        assert!(ignores.ranges.is_empty());
    }

    #[test]
    fn regex_comment_scan_skips_string_then_finds_real_marker() {
        // `"//"` inside the string must not win; the trailing `// MARK`
        // outside the literal does.
        let line = format!(
            "let s = \"code with // {} inside\"; // {} - reason: real, issue: 1055.",
            marker("_LINE"),
            marker("_LINE")
        );
        let source = file_lines(&[line]);
        let ignores = find_ignores("t.rs", &source).unwrap();
        assert!(ignores.singles.contains_key(&1));
    }

    #[test]
    fn regex_hash_scan_skips_char_literal_hash() {
        // `'#'` is a char literal; the later `# MARK` is the comment.
        let line = format!(
            "let c = '#'; # {} - reason: after char, issue: 1055.",
            marker("_LINE")
        );
        let source = file_lines(&[line]);
        let ignores = find_ignores("t.py", &source).unwrap();
        assert!(ignores.singles.contains_key(&1));
    }

    #[test]
    fn regex_word_boundary_rejects_glued_suffixes() {
        for suffix in ["_LINES", "_LINE2", "_LINE_", "_STARTX", "_STOPPED"] {
            let source = file_lines(&[format!("// {} - reason: glued.", marker(suffix))]);
            let err = find_ignores("t.rs", &source).unwrap_err();
            assert!(err.to_string().contains("unrecognized"), "{suffix}: {err}");
        }
    }

    #[test]
    fn regex_word_boundary_accepts_punctuation_suffix() {
        for suffix in ["_LINE", "_START", "_STOP"] {
            let open = marker(suffix);
            let source = if suffix == "_STOP" {
                file_lines(&[
                    format!("// {} - reason: opens, issue: 1055.", marker("_START")),
                    "code();".to_string(),
                    format!("// {open} - reason: closes, issue: 1055."),
                ])
            } else if suffix == "_START" {
                file_lines(&[
                    format!("// {open} - reason: opens, issue: 1055."),
                    "code();".to_string(),
                    format!("// {} - reason: closes, issue: 1055.", marker("_STOP")),
                ])
            } else {
                file_lines(&[format!("// {open} - reason: ok, issue: 1055.")])
            };
            assert!(find_ignores("t.rs", &source).is_ok(), "{suffix}");
        }
    }

    #[test]
    fn block_comment_markers_are_inert_wontfix() {
        // Wont-fix: the line-comment textual scan never looks
        // inside `/* ... */`, so a marker there is inert, not an ignore.
        let source = file_lines(&[
            "fn f() {".to_string(),
            format!("    /* {} - reason: block. */", marker("_LINE")),
            "    1".to_string(),
            "}".to_string(),
        ]);
        let ignores = find_ignores("t.rs", &source).unwrap();
        assert!(ignores.singles.is_empty());
        assert!(ignores.ranges.is_empty());
    }

    #[test]
    fn raw_string_markers_are_inert_wontfix() {
        // Wont-fix: raw strings (`r#"..."#`) are not decoded,
        // so a marker inside one is inert, not an ignore.
        let tricky = format!("let s = r#\"code with // {} inside\"#;", marker("_LINE"));
        let source = file_lines(&[tricky, "real();".to_string()]);
        let ignores = find_ignores("t.rs", &source).unwrap();
        assert!(ignores.singles.is_empty());
        assert!(ignores.ranges.is_empty());
    }

    #[test]
    fn hash_comment_markers_are_honored_for_pyi_stubs() {
        let source = file_lines(&[
            "def f() -> int: ...".to_string(),
            format!(
                "    pass  # {} - reason: fixture stub line, issue: 1055.",
                marker("_LINE")
            ),
        ]);
        let ignores = find_ignores("t.pyi", &source).unwrap();
        assert!(ignores.singles.contains_key(&2));
    }

    #[test]
    fn bare_policy_without_reason_is_rejected() {
        let source = file_lines(&[format!(
            "// {} - policy: docs/testing/strategy-details.md#coverage",
            marker("_LINE")
        )]);
        let err = find_ignores("t.rs", &source).unwrap_err();
        assert!(
            err.to_string().contains("bare policy"),
            "{err}"
        );
    }

    #[test]
    fn bare_policy_on_previous_line_is_rejected() {
        let source = file_lines(&[
            "    // policy: docs/testing/strategy-details.md#coverage".to_string(),
            format!("    // {}", marker("_LINE")),
            "    1".to_string(),
        ]);
        let err = find_ignores("t.rs", &source).unwrap_err();
        assert!(
            err.to_string().contains("bare policy"),
            "{err}"
        );
    }

    #[test]
    fn reason_without_issue_is_rejected() {
        let source = file_lines(&[format!(
            "// {} - reason: specific but untracked.",
            marker("_LINE")
        )]);
        let err = find_ignores("t.rs", &source).unwrap_err();
        assert!(
            err.to_string().contains("issue"),
            "{err}"
        );
    }

    #[test]
    fn reason_plus_issue_plus_policy_is_accepted() {
        let source = file_lines(&[format!(
            "// {} - reason: thin shim, issue: 1055, policy: docs/testing/strategy-details.md#coverage",
            marker("_LINE")
        )]);
        let ignores = find_ignores("t.rs", &source).unwrap();
        assert!(ignores.singles.contains_key(&1));
    }

    #[test]
    fn split_reason_and_issue_across_lines_is_accepted() {
        let source = file_lines(&[
            "    // reason: thin shim.".to_string(),
            format!("    // {} - issue: 1055.", marker("_LINE")),
            "    1".to_string(),
        ]);
        let ignores = find_ignores("t.rs", &source).unwrap();
        assert!(ignores.singles.contains_key(&2));
    }

    #[test]
    fn long_reason_fails_for_single_line() {
        let long = "x".repeat(MAX_REASON_LEN + 1);
        let source = file_lines(&[format!(
            "// {} - reason: {long}, issue: 1055",
            marker("_LINE")
        )]);
        let err = find_ignores("t.rs", &source).unwrap_err();
        assert!(err.to_string().contains("too long"), "{err}");
        assert!(err.to_string().contains("t.rs:1"), "{err}");
    }

    #[test]
    fn long_reason_fails_for_range_start() {
        let long = "y".repeat(MAX_REASON_LEN + 40);
        let source = file_lines(&[
            format!("// {} - reason: {long}, issue: 1055", marker("_START")),
            "code();".to_string(),
            format!("// {} - reason: closes, issue: 1055.", marker("_STOP")),
        ]);
        let err = find_ignores("t.rs", &source).unwrap_err();
        assert!(err.to_string().contains("too long"), "{err}");
    }

    #[test]
    fn max_length_reason_passes_at_boundary() {
        let suffix = ", issue: 1055";
        let exact = "z".repeat(MAX_REASON_LEN - suffix.len());
        let source = file_lines(&[format!(
            "// {} - reason: {exact}{suffix}",
            marker("_LINE")
        )]);
        assert!(find_ignores("t.rs", &source)
            .unwrap()
            .singles
            .contains_key(&1));
    }

    #[test]
    fn empty_policy_reason_fails() {
        let source = file_lines(&[
            "    // policy:   ".to_string(),
            format!("    // {}", marker("_LINE")),
            "    1".to_string(),
        ]);
        assert!(find_ignores("t.rs", &source).is_err());
    }

    #[test]
    fn slash_markers_are_honored_for_wrapped_langs() {
        for path in [
            "t.java", "t.kt", "t.scala", "t.cs", "t.fs", "t.fsi", "t.mjs", "t.cjs", "t.mts",
            "t.cts",
        ] {
            let source = file_lines(&[format!(
                "// {} - reason: fixture, issue: 1055.",
                marker("_LINE")
            )]);
            let ignores = find_ignores(path, &source).unwrap();
            assert!(ignores.singles.contains_key(&1), "{path}");
        }
    }

    #[test]
    fn reason_wins_when_line_carries_both_keys() {
        let source = file_lines(&[format!(
            "// {} - reason: first policy: second.",
            marker("_LINE")
        )]);
        let ignores = find_ignores("t.rs", &source).unwrap();
        assert!(ignores.singles.contains_key(&1));
        assert_eq!(ignores.singles[&1], "first policy: second.".to_string());
    }
}
