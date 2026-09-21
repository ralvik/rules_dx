//! Clang-tidy output grammar.
//!
//! Per-family module of [`crate::parsers`]: the pinned-shape contract
//! and [`ParseError`] semantics live in the parent module docs.
//!
//! See: `docs/quality/tool-integrations.md#initial-adapter-qualification`

use super::{code_name, known, missing, point, FileFinding, ParseError};
use crate::{Finding, ToolSeverity};

/// Parses clang-tidy text diagnostics on stderr. `files` are the
/// scratch-absolute paths the tool checked.
///
/// Each finding is one line shaped
/// `<path>:<line>:<col>: <warning|error>: <message> [<check>]`.
/// The trailing `[check]` names the rule; lines without it report
/// under an empty rule. Findings are start points. Clean is exit 0
/// with no diagnostic lines; findings exit 1. Any other shape is a
/// grammar mismatch, never a silent pass.
pub fn parse_clang_tidy(
    stderr: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "clang_tidy";
    let text = std::str::from_utf8(stderr).map_err(|err| ParseError::Shape {
        tool: TOOL,
        detail: err.to_string(),
    })?;
    let mut findings = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let (path, rest) = trimmed
            .split_once(':')
            .ok_or_else(|| missing(TOOL, "location", line))?;
        let checked = known(TOOL, files, path)?;
        let rest = rest.trim_start_matches(':').trim_start();
        let mut parts = rest.splitn(4, ':');
        let line_no: u64 = parts
            .next()
            .ok_or_else(|| missing(TOOL, "line", line))?
            .trim()
            .parse()
            .map_err(|_| missing(TOOL, "line", line))?;
        let col_no: u64 = parts
            .next()
            .ok_or_else(|| missing(TOOL, "column", line))?
            .trim()
            .parse()
            .map_err(|_| missing(TOOL, "column", line))?;
        let severity_word = parts
            .next()
            .ok_or_else(|| missing(TOOL, "severity", line))?
            .trim();
        let severity = match severity_word {
            "warning" => ToolSeverity::Warning,
            "error" => ToolSeverity::Error,
            _ => return Err(missing(TOOL, "severity", line)),
        };
        let message = parts
            .next()
            .ok_or_else(|| missing(TOOL, "message", line))?
            .trim();
        if message.is_empty() {
            return Err(missing(TOOL, "message", line));
        }
        let (message, rule_id) = match message.rfind(" [") {
            Some(start) if message.ends_with(']') => (
                message[..start].trim_end().to_owned(),
                message[start + 2..message.len() - 1].to_owned(),
            ),
            _ => (message.to_owned(), String::new()),
        };
        if message.is_empty() {
            return Err(missing(TOOL, "message", line));
        }
        let (start, end) = point(line_no, col_no);
        findings.push(FileFinding {
            file: checked.to_owned(),
            finding: Finding {
                tool_id: TOOL.to_owned(),
                rule_id,
                message,
                severity,
                start,
                end,
                suggestions: Vec::new(),
            },
        });
    }
    if findings.is_empty() && code != Some(0) {
        return Err(ParseError::Shape {
            tool: TOOL,
            detail: format!("exit {} with no diagnostics", code_name(code)),
        });
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIRTY: &str = "cc/tests/fixtures/clang_tidy/Sample.c:4:3: warning: do not use 'else' after 'return' [readability-else-after-return]\n";

    #[test]
    fn clang_tidy_reports_text_diagnostics() {
        let findings = parse_clang_tidy(
            DIRTY.as_bytes(),
            Some(1),
            &["cc/tests/fixtures/clang_tidy/Sample.c"],
        )
        .expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, "cc/tests/fixtures/clang_tidy/Sample.c");
        assert_eq!(findings[0].finding.rule_id, "readability-else-after-return");
        assert_eq!(
            findings[0].finding.message,
            "do not use 'else' after 'return'"
        );
        let clean = parse_clang_tidy(b"", Some(0), &["cc/tests/fixtures/clang_tidy/Sample.c"])
            .expect("parsed");
        assert!(clean.is_empty());
        assert!(
            parse_clang_tidy(b"", Some(1), &["cc/tests/fixtures/clang_tidy/Sample.c"]).is_err()
        );
        assert!(parse_clang_tidy(DIRTY.as_bytes(), Some(1), &["other.c"]).is_err());
        assert!(parse_clang_tidy(b"nope\n", Some(1), &["x"]).is_err());
        assert!(parse_clang_tidy(&[0xff], Some(1), &["x"]).is_err());
    }
}
