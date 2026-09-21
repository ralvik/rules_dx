//! Keep-sorted output grammar.
//!
//! Per-family module of [`crate::parsers`]: the pinned-shape contract
//! and [`ParseError`] semantics live in the parent module docs.
//!
//! See: `docs/quality/tool-integrations.md#initial-adapter-qualification`

use super::{code_name, known, missing, point, FileFinding, ParseError};
use crate::{Finding, ToolSeverity};

/// Parses `keep-sorted` text diagnostics on stdout. `files` are the
/// workspace-relative mirror paths.
///
/// Each finding is one line `<path>:<line>: <message>` at line
/// granularity (keep-sorted reports unsorted blocks, not columns).
/// Every diagnostic is a warning. Clean is exit 0 with no lines;
/// findings exit non-zero. Check-only with sandbox-apply-and-diff.
pub fn parse_keep_sorted(
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "keep_sorted";
    let text = std::str::from_utf8(stdout).map_err(|err| ParseError::Shape {
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
        let mut parts = rest.splitn(2, ':');
        let line_no: u64 = parts
            .next()
            .ok_or_else(|| missing(TOOL, "line", line))?
            .trim()
            .parse()
            .map_err(|_| missing(TOOL, "line", line))?;
        let message = parts
            .next()
            .ok_or_else(|| missing(TOOL, "message", line))?
            .trim();
        if message.is_empty() {
            return Err(missing(TOOL, "message", line));
        }
        let (start, end) = point(line_no, 1);
        findings.push(FileFinding {
            file: checked.to_owned(),
            finding: Finding {
                tool_id: TOOL.to_owned(),
                rule_id: String::new(),
                message: message.to_owned(),
                severity: ToolSeverity::Warning,
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
    const DIRTY: &str = "notes.txt:4: block is not sorted\n";
    #[test]
    fn keep_sorted_reports_line_points() {
        let findings =
            parse_keep_sorted(DIRTY.as_bytes(), Some(1), &["notes.txt"]).expect("parsed");
        assert_eq!(findings.len(), 1);
        let clean = parse_keep_sorted(b"", Some(0), &["notes.txt"]).expect("parsed");
        assert!(clean.is_empty());
        assert!(parse_keep_sorted(b"", Some(1), &["notes.txt"]).is_err());
        assert!(parse_keep_sorted(DIRTY.as_bytes(), Some(1), &["other.txt"]).is_err());
        assert!(parse_keep_sorted(&[0xff], Some(1), &["x"]).is_err());
    }
}
