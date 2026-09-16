//! Markdown checker output grammar.
//!
//! Per-family module of [`crate::parsers`]: the pinned-shape contract
//! and [`ParseError`] semantics live in the parent module docs.

use serde::Deserialize;

use super::{code_name, known, point, FileFinding, ParseError};
use crate::{Finding, ToolSeverity};

/// One newline-delimited JSON finding line from the repo-owned Markdown
/// checker, keyed by the `--source` workspace path.
#[derive(Debug, Deserialize)]
struct MarkdownLine {
    path: String,
    line: u64,
    kind: String,
    message: String,
}

/// Parses Markdown checker stdout. `files` are the workspace paths from
/// the `--source` mappings, in stage order: the checker keys sibling
/// resolution and finding paths off those keys, so the backend re-roots
/// each validated path onto its scratch-absolute path before placement.
/// Findings are line-level points at column 1 with the kebab-case kind as
/// the rule; every kind is an error (broken links and structure fail the
/// lint gate). Clean output is empty stdout on exit 0; any other exit is
/// an operational failure, never a partial result.
pub fn parse_markdown_findings(
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "markdown_check";
    const KINDS: &[&str] = &[
        "missing-file-target",
        "missing-anchor",
        "missing-heading",
        "duplicate-heading",
        "missing-language-tag",
    ];
    if code != Some(0) {
        return Err(ParseError::Shape {
            tool: TOOL,
            detail: format!("exit {}: findings exist only on exit 0", code_name(code)),
        });
    }
    let text = std::str::from_utf8(stdout).map_err(|err| ParseError::Json {
        tool: TOOL,
        detail: err.to_string(),
    })?;
    let mut findings = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let parsed: MarkdownLine = serde_json::from_str(line).map_err(|err| ParseError::Json {
            tool: TOOL,
            detail: err.to_string(),
        })?;
        if !KINDS.contains(&parsed.kind.as_str()) {
            return Err(ParseError::Shape {
                tool: TOOL,
                detail: format!("unknown finding kind {:?}", parsed.kind),
            });
        }
        if parsed.line == 0 {
            return Err(ParseError::Shape {
                tool: TOOL,
                detail: format!("zero line for {}", parsed.kind),
            });
        }
        let checked = known(TOOL, files, &parsed.path)?;
        let (start, end) = point(parsed.line, 1);
        findings.push(FileFinding {
            file: checked.to_owned(),
            finding: Finding {
                tool_id: TOOL.to_owned(),
                rule_id: parsed.kind,
                message: parsed.message,
                severity: ToolSeverity::Error,
                start,
                end,
                suggestions: Vec::new(),
            },
        });
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TextPosition;

    #[test]
    fn markdown_parses_workspace_keyed_findings() {
        let stdout = concat!(
            "{\"path\":\"doc/guide.md\",\"line\":3,\"kind\":\"missing-file-target\",\"message\":\"link target \\\"nope.md\\\" does not match a checked source\"}\n",
            "{\"path\":\"doc/guide.md\",\"line\":7,\"kind\":\"missing-anchor\",\"message\":\"anchor \\\"#nope\\\" not found\"}\n",
        );
        let findings =
            parse_markdown_findings(stdout.as_bytes(), Some(0), &["doc/guide.md"]).expect("parsed");
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].file, "doc/guide.md");
        assert_eq!(findings[0].finding.tool_id, "markdown_check");
        assert_eq!(findings[0].finding.rule_id, "missing-file-target");
        assert_eq!(findings[0].finding.severity, ToolSeverity::Error);
        assert_eq!(
            (findings[0].finding.start, findings[0].finding.end),
            (TextPosition { line: 3, column: 1 }, None)
        );
        assert_eq!(findings[1].finding.rule_id, "missing-anchor");
        assert!(findings[0].finding.suggestions.is_empty());
        // Clean output is empty stdout on exit 0; blank lines are skipped.
        assert!(parse_markdown_findings(b"\n", Some(0), &["doc/guide.md"])
            .expect("parsed")
            .is_empty());
    }

    #[test]
    fn markdown_rejects_exits_kinds_lines_and_files() {
        let clean =
            "{\"path\":\"doc/guide.md\",\"line\":1,\"kind\":\"missing-heading\",\"message\":\"m\"}";
        // Findings exist only on exit 0: any other exit is an action
        // failure even with parseable lines.
        assert!(parse_markdown_findings(clean.as_bytes(), Some(1), &["doc/guide.md"]).is_err());
        assert!(parse_markdown_findings(clean.as_bytes(), None, &["doc/guide.md"]).is_err());
        assert!(parse_markdown_findings(b"", Some(2), &["doc/guide.md"]).is_err());
        // Non-UTF-8 and malformed lines are grammar errors.
        assert!(parse_markdown_findings(b"\xff", Some(0), &["doc/guide.md"]).is_err());
        assert!(parse_markdown_findings(b"{nope", Some(0), &["doc/guide.md"]).is_err());
        let missing_field = r#"{"path":"doc/guide.md","line":1,"kind":"missing-heading"}"#;
        assert!(
            parse_markdown_findings(missing_field.as_bytes(), Some(0), &["doc/guide.md"]).is_err()
        );
        let unknown_kind = r#"{"path":"doc/guide.md","line":1,"kind":"bad-kind","message":"m"}"#;
        assert!(
            parse_markdown_findings(unknown_kind.as_bytes(), Some(0), &["doc/guide.md"]).is_err()
        );
        let zero_line =
            r#"{"path":"doc/guide.md","line":0,"kind":"missing-heading","message":"m"}"#;
        assert!(parse_markdown_findings(zero_line.as_bytes(), Some(0), &["doc/guide.md"]).is_err());
        let elsewhere = r#"{"path":"other.md","line":1,"kind":"missing-heading","message":"m"}"#;
        assert!(parse_markdown_findings(elsewhere.as_bytes(), Some(0), &["doc/guide.md"]).is_err());
    }
}
