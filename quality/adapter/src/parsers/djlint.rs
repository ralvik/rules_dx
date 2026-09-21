//! Djlint output grammar.
//!
//! Per-family module of [`crate::parsers`]: the pinned-shape contract
//! and [`ParseError`] semantics live in the parent module docs.
//!
//! See: `docs/quality/tool-integrations.md#initial-adapter-qualification`

use super::{code_name, known, missing, point, FileFinding, ParseError};
use crate::{Finding, ToolSeverity};

/// Parses `djlint --lint` text diagnostics on stdout. `files` are the
/// workspace-relative mirror paths.
///
/// Each finding is one line `<path>:<line>:<col>: <rule> <message>`.
/// Findings are points at the reported position; every diagnostic is
/// a warning. Clean is exit 0 with no lines; findings exit 1.
pub fn parse_djlint(
    stderr: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "djlint";
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
        let mut parts = rest.splitn(3, ':');
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
        let tail = parts
            .next()
            .ok_or_else(|| missing(TOOL, "message", line))?
            .trim();
        if tail.is_empty() {
            return Err(missing(TOOL, "message", line));
        }
        let (rule, message) = match tail.split_once(' ') {
            Some((r, m)) if !r.is_empty() && !m.is_empty() => (r.to_owned(), m.to_owned()),
            _ => (String::new(), tail.to_owned()),
        };
        let (start, end) = point(line_no, col_no);
        findings.push(FileFinding {
            file: checked.to_owned(),
            finding: Finding {
                tool_id: TOOL.to_owned(),
                rule_id: rule,
                message,
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

/// Parses `djlint --reformat --check` diff stdout.
///
/// Whole-file rewrite with check/diff mode: unified diff markers
/// yield one `1:1` format finding per file; empty output on exit 0
/// is clean.
pub fn parse_djlint_format(
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "djlint";
    let text = std::str::from_utf8(stdout).map_err(|err| ParseError::Shape {
        tool: TOOL,
        detail: err.to_string(),
    })?;
    let mut mentioned: Vec<String> = Vec::new();
    for line in text.lines() {
        if let Some(path) = line.strip_prefix("--- ") {
            let path = path.strip_prefix("a/").unwrap_or(path).trim();
            if path.is_empty() || path == "/dev/null" {
                continue;
            }
            if !mentioned.iter().any(|known| known == path) {
                mentioned.push(path.to_owned());
            }
        }
    }
    if mentioned.is_empty() {
        if code == Some(0) {
            return Ok(Vec::new());
        }
        return Err(ParseError::Shape {
            tool: TOOL,
            detail: format!("exit {} with no diff markers", code_name(code)),
        });
    }
    if code == Some(0) {
        return Err(ParseError::Shape {
            tool: TOOL,
            detail: format!("exit {} with diff markers", code_name(code)),
        });
    }
    let mut findings = Vec::with_capacity(mentioned.len());
    for path in mentioned {
        let checked = known(TOOL, files, &path)?;
        let (start, end) = point(1, 1);
        findings.push(FileFinding {
            file: checked.to_owned(),
            finding: Finding {
                tool_id: TOOL.to_owned(),
                rule_id: String::new(),
                message: "file is not formatted".to_owned(),
                severity: ToolSeverity::Warning,
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
    const LINT_DIRTY: &str = "base.html:3:1: H006 img tags require alt text\n";
    const FMT_DIRTY: &str = "--- a/base.html\n+++ b/base.html\n@@ -1 +1 @@\n-BADFMT\n+fixed\n";
    #[test]
    fn djlint_reports_lint_and_format() {
        let findings =
            parse_djlint(LINT_DIRTY.as_bytes(), Some(1), &["base.html"]).expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].finding.rule_id, "H006");
        let clean = parse_djlint(b"", Some(0), &["base.html"]).expect("parsed");
        assert!(clean.is_empty());
        assert!(parse_djlint(b"", Some(1), &["base.html"]).is_err());
        let fmt =
            parse_djlint_format(FMT_DIRTY.as_bytes(), Some(1), &["base.html"]).expect("parsed");
        assert_eq!(fmt.len(), 1);
        let fmt_clean = parse_djlint_format(b"", Some(0), &["base.html"]).expect("parsed");
        assert!(fmt_clean.is_empty());
        assert!(parse_djlint(&[0xff], Some(1), &["x"]).is_err());
    }
}
