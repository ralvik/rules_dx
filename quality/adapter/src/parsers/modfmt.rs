//! Modfmt output grammar.
//!
//! Per-family module of [`crate::parsers`]: the pinned-shape contract
//! and [`ParseError`] semantics live in the parent module docs.
//!
//! See: `docs/quality/tool-integrations.md#initial-adapter-qualification`

use super::{code_name, known, point, FileFinding, ParseError};
use crate::{Finding, ToolSeverity};

/// Parses `modfmt` diff stdout for `go.mod` plus `go.work`.
///
/// Whole-file rewrite with check/diff mode: unified diff markers
/// yield one `1:1` format finding per file; empty output on exit 0
/// is clean. Any other shape is a grammar mismatch.
pub fn parse_modfmt(
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "modfmt";
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
    if code != Some(0) && code != Some(1) {
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
    const DIRTY: &str = "--- a/go.mod\n+++ b/go.mod\n@@ -1 +1 @@\n-BADFMT\n+fixed\n";
    #[test]
    fn modfmt_reports_diff_files() {
        let findings = parse_modfmt(DIRTY.as_bytes(), Some(1), &["go.mod"]).expect("parsed");
        assert_eq!(findings.len(), 1);
        let clean = parse_modfmt(b"", Some(0), &["go.mod"]).expect("parsed");
        assert!(clean.is_empty());
        assert!(parse_modfmt(b"", Some(1), &["go.mod"]).is_err());
        assert!(parse_modfmt(DIRTY.as_bytes(), Some(1), &["other.mod"]).is_err());
        assert!(parse_modfmt(&[0xff], Some(0), &["x"]).is_err());
    }
}
