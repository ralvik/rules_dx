//! Govet output grammar.
//!
//! Per-family module of [`crate::parsers`]: the pinned-shape contract
//! and [`ParseError`] semantics live in the parent module docs.
//!
//! See: `docs/quality/tool-integrations.md#initial-adapter-qualification`

use super::{code_name, known, missing, point, FileFinding, ParseError};
use crate::{Finding, ToolSeverity};

/// Parses `go vet` text diagnostics on stderr. `files` are the
/// workspace-relative mirror paths (vet reports working-directory
/// relative paths, so the caller re-anchors them like Prettier).
///
/// Each finding is one line shaped `<path>:<line>:<col>: <message>`
/// (split from the left because messages contain colons). Findings
/// are points at the reported 1-based position; every diagnostic is
/// a warning. Clean is exit 0 with no diagnostic lines; findings
/// exit 1. Any other shape is a grammar mismatch, never a silent
/// pass.
pub fn parse_govet(
    stderr: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "govet";
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
        let message = parts
            .next()
            .ok_or_else(|| missing(TOOL, "message", line))?
            .trim();
        if message.is_empty() {
            return Err(missing(TOOL, "message", line));
        }
        let (start, end) = point(line_no, col_no);
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

    const DIRTY: &str =
        "go/tests/fixtures/govet/Sample.go:7:2: non-constant format string in call to fmt.Printf\n";

    #[test]
    fn govet_reports_text_diagnostics() {
        let findings = parse_govet(
            DIRTY.as_bytes(),
            Some(1),
            &["go/tests/fixtures/govet/Sample.go"],
        )
        .expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, "go/tests/fixtures/govet/Sample.go");
        assert_eq!(
            findings[0].finding.message,
            "non-constant format string in call to fmt.Printf"
        );
        let clean =
            parse_govet(b"", Some(0), &["go/tests/fixtures/govet/Sample.go"]).expect("parsed");
        assert!(clean.is_empty());
        assert!(parse_govet(b"", Some(1), &["go/tests/fixtures/govet/Sample.go"]).is_err());
        assert!(parse_govet(DIRTY.as_bytes(), Some(1), &["other.go"]).is_err());
        assert!(parse_govet(b"nope\n", Some(1), &["x"]).is_err());
        assert!(parse_govet(&[0xff], Some(1), &["x"]).is_err());
    }
}
